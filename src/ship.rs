//! Ship from store. An order is shipped to its customer from a store's stock rather than from the
//! distribution centre: from the store someone names, or else from the one holding the most of it
//! among the stores holding all of it. Shipped, the order's delivery note is made from that store,
//! every line, whatever warehouse the order named or did not, and submitted, so the goods leave its
//! stock, with the carrier and tracking number kept. An order is either shipped from a store or
//! collected in one, never both.

use crate::collect::{is_store, say, shipping, short_of, to_deliver};
use crate::obj;
use async_trait::async_trait;
use chrono::Utc;
use ferrum_domain::flow::{MapSpec, created, map_document_with, shared_line_fields};
use ferrum_domain::util::{elevated, id, s};
use ferrum_kernel::{Doc, DocHook, Error, HookCtx, Result, Tx};
use rust_decimal::Decimal;
use serde_json::{Value, json};
use std::collections::HashMap;
use uuid::Uuid;

pub struct ShipHook;

/// What a delivery note takes from its order, as ERP's own "Create Delivery Note" maps it; the
/// warehouse is the store's.
const HEADER: &[&str] = &["company", "currency", "conversion_rate", "tax_template", "discount_percent", "project", "customer", "price_list", "po_no"];
const LINE: &[&str] = &["item", "uom", "price_list_rate", "discount_percent", "batch_no", "serial_no"];

/// The store of `company` holding the most of `need` among those holding all of it.
async fn pick_store(tx: &mut Tx, tenant: Uuid, company: Option<Uuid>, need: &[(Uuid, String, Decimal)]) -> Result<Option<Uuid>> {
    let items: Vec<Uuid> = need.iter().map(|n| n.0).collect();
    let held: Vec<(Uuid, String, Uuid, Decimal)> = sqlx::query_as(
        "select w.id, w.warehouse_name, b.item_id, sum(b.qty) from e_warehouse w \
         join stock_balance b on b.tenant_id = w.tenant_id and b.warehouse_id = w.id \
         where w.tenant_id = $1 and w.company is not distinct from $2 and coalesce((w.ext->>'retail_store')::boolean, false) and not coalesce(w.disabled, false) \
         and b.item_id = any($3) group by w.id, w.warehouse_name, b.item_id",
    )
    .bind(tenant)
    .bind(company)
    .bind(&items)
    .fetch_all(&mut **tx)
    .await?;
    let mut stores: HashMap<Uuid, (String, HashMap<Uuid, Decimal>)> = HashMap::new();
    for (store, name, item, qty) in held {
        stores.entry(store).or_insert_with(|| (name, HashMap::new())).1.insert(item, qty);
    }
    let best = stores
        .into_iter()
        .filter(|(_, (_, has))| need.iter().all(|(item, _, qty)| has.get(item).is_some_and(|h| h >= qty)))
        .map(|(store, (name, has))| (has.values().copied().sum::<Decimal>(), name, store))
        // The most of it, then the first by name.
        .max_by(|a, b| a.0.cmp(&b.0).then_with(|| b.1.cmp(&a.1)));
    Ok(best.map(|(_, _, store)| store))
}

/// The order's submitted state and company: (docstatus, status, company).
async fn order_state(tx: &mut Tx, tenant: Uuid, order: Uuid) -> Result<(i16, Option<String>, Option<Uuid>)> {
    let row: Option<(i16, Option<String>, Option<Uuid>)> =
        sqlx::query_as("select docstatus, status, company from e_sales_order where tenant_id = $1 and id = $2")
            .bind(tenant)
            .bind(order)
            .fetch_optional(&mut **tx)
            .await?;
    row.ok_or_else(|| Error::field("sales_order", "is not this workspace's"))
}

#[async_trait]
impl DocHook for ShipHook {
    fn name(&self) -> &'static str {
        "retail.ship"
    }

    async fn before_validate(&self, hc: &mut HookCtx<'_>, doc: &mut Doc) -> Result<()> {
        if id(doc, "store").is_some() || s(doc, "status").is_some_and(|st| st != "To Ship") {
            return Ok(());
        }
        let Some(order) = id(doc, "sales_order") else { return Ok(()) };
        let (_, _, company) = order_state(hc.tx, hc.ctx.tenant, order).await?;
        let so = hc.kernel.get_tx(&elevated(hc.ctx), hc.tx, "SalesOrder", order, false).await?;
        let need = to_deliver(hc.tx, hc.ctx.tenant, &so).await?;
        if need.is_empty() {
            return Ok(());
        }
        match pick_store(hc.tx, hc.ctx.tenant, company, &need).await? {
            Some(store) => {
                doc.insert("store".into(), json!(store.to_string()));
                Ok(())
            }
            None => Err(Error::field("store", "no store holds all of it: name one, or bring what it lacks there first")),
        }
    }

    async fn validate(&self, hc: &mut HookCtx<'_>, doc: &Doc) -> Result<()> {
        if s(doc, "status").is_some_and(|st| st != "To Ship") {
            return Ok(());
        }
        let order = id(doc, "sales_order").ok_or_else(|| Error::field("sales_order", "is required"))?;
        let (docstatus, status, company) = order_state(hc.tx, hc.ctx.tenant, order).await?;
        if docstatus != 1 {
            return Err(Error::field("sales_order", "is not submitted yet"));
        }
        if status.as_deref() == Some("Closed") {
            return Err(Error::field("sales_order", "is closed"));
        }
        let so = hc.kernel.get_tx(&elevated(hc.ctx), hc.tx, "SalesOrder", order, false).await?;
        if to_deliver(hc.tx, hc.ctx.tenant, &so).await?.is_empty() {
            return Err(Error::field("sales_order", "has nothing left to ship"));
        }
        let store = id(doc, "store").ok_or_else(|| Error::field("store", "is required"))?;
        if !is_store(hc.tx, hc.ctx.tenant, store, company).await? {
            return Err(Error::field("store", "is not a store of the order's company (Is A Store, on the warehouse)"));
        }
        if let Some(n) = shipping(hc.tx, hc.ctx.tenant, order, id(doc, "id")).await? {
            return Err(Error::field("sales_order", format!("is shipped under {n}")));
        }
        let collected: Option<String> = sqlx::query_scalar(
            "select number from e_collect_order where tenant_id = $1 and sales_order = $2 and status in ('Ordered', 'Ready', 'Collected') limit 1",
        )
        .bind(hc.ctx.tenant)
        .bind(order)
        .fetch_optional(&mut **hc.tx)
        .await?
        .flatten();
        if let Some(n) = collected {
            return Err(Error::field("sales_order", format!("is collected in a store under {n}")));
        }
        Ok(())
    }

    async fn action(&self, hc: &mut HookCtx<'_>, name: &str, doc: &Doc, params: &Value) -> Result<Option<Value>> {
        let shipment = id(doc, "id").ok_or_else(|| Error::internal("a shipment without an id"))?;
        let system = elevated(hc.ctx);
        match name {
            "ship_it" => {
                let (order, store) = (id(doc, "sales_order").unwrap_or_default(), id(doc, "store").unwrap_or_default());
                let so = hc.kernel.get_tx(&system, hc.tx, "SalesOrder", order, false).await?;
                let need = to_deliver(hc.tx, hc.ctx.tenant, &so).await?;
                if need.is_empty() {
                    return Err(Error::invalid("nothing is left to ship on this order"));
                }
                let short = short_of(hc.tx, hc.ctx.tenant, store, &need).await?;
                if !short.is_empty() {
                    return Err(Error::invalid(format!("the store does not hold it: {}", say(&short))));
                }
                let given = |k: &str| {
                    params.get(k).and_then(Value::as_str).map(str::trim).filter(|v| !v.is_empty()).map(str::to_string).or_else(|| s(doc, k).map(str::to_string))
                };
                let (carrier, tracking) = (given("carrier"), given("tracking_no"));
                // The order's delivery note, every line from this store: ERP's own copies the order's
                // warehouse, and an order from the web names none.
                let spec = MapSpec { done_field: "delivered_qty", link_field: "sales_order", line_field: "so_line", header: HEADER, line_fields: LINE };
                let mut input = map_document_with(&so, &spec, &shared_line_fields(&hc.ctx.schema, "SalesOrder", "DeliveryNote"))?;
                for line in input.get_mut("items").and_then(Value::as_array_mut).into_iter().flatten().filter_map(Value::as_object_mut) {
                    line.insert("warehouse".into(), json!(store.to_string()));
                }
                input.insert("set_warehouse".into(), json!(store.to_string()));
                input.insert("transporter".into(), json!(carrier));
                let draft = hc.kernel.insert_tx(&system, hc.tx, "DeliveryNote", input).await?;
                let note = id(&draft, "id").ok_or_else(|| Error::internal("no delivery note was made"))?;
                let submitted = hc.kernel.submit_tx(&system, hc.tx, "DeliveryNote", note, None).await?;
                let values = json!({"status": "Shipped", "shipped_at": Utc::now().to_rfc3339(), "delivery_note": note.to_string(),
                                    "carrier": carrier, "tracking_no": tracking});
                hc.kernel.set_values_tx(&system, hc.tx, "StoreShipment", shipment, obj(values)).await?;
                Ok(Some(json!({"status": "Shipped", "delivery_note": created("DeliveryNote", &submitted)})))
            }
            "cancel_shipment" => {
                hc.kernel.set_values_tx(&system, hc.tx, "StoreShipment", shipment, obj(json!({"status": "Cancelled"}))).await?;
                Ok(Some(json!({"status": "Cancelled"})))
            }
            _ => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn what_is_short_is_said_plainly() {
        let k = Uuid::now_v7();
        assert_eq!(say(&[(k, "Kettle".into(), Decimal::ONE, Decimal::TWO)]), "Kettle (1 of 2)");
    }
}
