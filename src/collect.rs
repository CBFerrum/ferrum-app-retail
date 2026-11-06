//! Click and collect. An order bought online is collected from the store it ships from, one
//! collection to an order, and never one also shipped from a store. It is ready only once the store
//! holds every line of it; what it lacks can be brought from another store by a transfer (the
//! endless aisle). It is handed over only against its pickup code, when the order's delivery note
//! is made and submitted, so the goods leave the store's stock.

use crate::obj;
use async_trait::async_trait;
use chrono::Utc;
use ferrum_domain::flow::created;
use ferrum_domain::util::{d, elevated, id, money, rows, s, unit_factor};
use ferrum_kernel::{Doc, DocHook, Error, HookCtx, Result, Tx};
use rust_decimal::Decimal;
use serde_json::{Value, json};
use uuid::Uuid;

pub struct CollectHook;

/// The quantity of `item` in `warehouse`, from ERP's balances.
pub(crate) async fn on_hand(tx: &mut Tx, tenant: Uuid, warehouse: Uuid, item: Uuid) -> Result<Decimal> {
    Ok(sqlx::query_scalar("select coalesce(sum(qty), 0) from stock_balance where tenant_id = $1 and warehouse_id = $2 and item_id = $3")
        .bind(tenant)
        .bind(warehouse)
        .bind(item)
        .fetch_one(&mut **tx)
        .await?)
}

/// Whether `item` is kept in stock (a service is not).
async fn stocked(tx: &mut Tx, tenant: Uuid, item: Uuid) -> Result<bool> {
    let kept: Option<bool> = sqlx::query_scalar("select coalesce(is_stock_item, false) from e_item where tenant_id = $1 and id = $2")
        .bind(tenant)
        .bind(item)
        .fetch_optional(&mut **tx)
        .await?;
    Ok(kept.unwrap_or(false))
}

/// What an order still has to deliver of its stock items, in stock units: (item, its name, quantity).
pub(crate) async fn to_deliver(tx: &mut Tx, tenant: Uuid, order: &Doc) -> Result<Vec<(Uuid, String, Decimal)>> {
    let mut need: Vec<(Uuid, String, Decimal)> = vec![];
    for line in rows(order, "items") {
        let Some(item) = id(&line, "item") else { continue };
        let left = (d(&line, "qty") - d(&line, "delivered_qty")) * unit_factor(&line);
        if left <= Decimal::ZERO || !stocked(tx, tenant, item).await? {
            continue;
        }
        match need.iter_mut().find(|n| n.0 == item) {
            Some(n) => n.2 += left,
            None => need.push((item, s(&line, "item_name").unwrap_or("an item").to_string(), left)),
        }
    }
    Ok(need)
}

/// Of `need`, what `store` holds too little of: (item, its name, held, needed).
pub(crate) async fn short_of(tx: &mut Tx, tenant: Uuid, store: Uuid, need: &[(Uuid, String, Decimal)]) -> Result<Vec<(Uuid, String, Decimal, Decimal)>> {
    let mut short = vec![];
    for (item, name, qty) in need {
        let have = on_hand(tx, tenant, store, *item).await?;
        if have < *qty {
            short.push((*item, name.clone(), have, *qty));
        }
    }
    Ok(short)
}

/// What is short, as "Kettle (1 of 2), Toaster (0 of 1)".
pub(crate) fn say(short: &[(Uuid, String, Decimal, Decimal)]) -> String {
    short.iter().map(|(_, name, have, need)| format!("{name} ({} of {})", have.normalize(), need.normalize())).collect::<Vec<_>>().join(", ")
}

/// Whether `warehouse` is a store (Is A Store) of `company`.
pub(crate) async fn is_store(tx: &mut Tx, tenant: Uuid, warehouse: Uuid, company: Option<Uuid>) -> Result<bool> {
    let store: Option<bool> = sqlx::query_scalar(
        "select coalesce((ext->>'retail_store')::boolean, false) and not coalesce(is_group, false) and company is not distinct from $3 from e_warehouse where tenant_id = $1 and id = $2",
    )
    .bind(tenant)
    .bind(warehouse)
    .bind(company)
    .fetch_optional(&mut **tx)
    .await?;
    Ok(store.unwrap_or(false))
}

/// A live shipment from a store of `order`, other than `except`: its number.
pub(crate) async fn shipping(tx: &mut Tx, tenant: Uuid, order: Uuid, except: Option<Uuid>) -> Result<Option<String>> {
    Ok(sqlx::query_scalar(
        "select number from e_store_shipment where tenant_id = $1 and sales_order = $2 and id is distinct from $3 and status = 'To Ship' limit 1",
    )
    .bind(tenant)
    .bind(order)
    .bind(except)
    .fetch_optional(&mut **tx)
    .await?
    .flatten())
}

/// A six-digit pickup code.
fn code() -> String {
    let n = u128::from_le_bytes(*Uuid::new_v4().as_bytes()) % 1_000_000;
    format!("{n:06}")
}

#[async_trait]
impl DocHook for CollectHook {
    fn name(&self) -> &'static str {
        "retail.collect"
    }

    fn fills(&self) -> &'static [&'static str] {
        &["pickup_code"]
    }

    async fn before_validate(&self, _hc: &mut HookCtx<'_>, doc: &mut Doc) -> Result<()> {
        if s(doc, "pickup_code").is_none() {
            doc.insert("pickup_code".into(), json!(code()));
        }
        Ok(())
    }

    async fn validate(&self, hc: &mut HookCtx<'_>, doc: &Doc) -> Result<()> {
        if matches!(s(doc, "status"), Some("Collected" | "Cancelled")) {
            return Ok(());
        }
        let order = id(doc, "sales_order").ok_or_else(|| Error::field("sales_order", "is required"))?;
        let row: Option<(i16, Option<Uuid>)> = sqlx::query_as("select docstatus, set_warehouse from e_sales_order where tenant_id = $1 and id = $2")
            .bind(hc.ctx.tenant)
            .bind(order)
            .fetch_optional(&mut **hc.tx)
            .await?;
        let (docstatus, ships_from) = row.ok_or_else(|| Error::field("sales_order", "is not this workspace's"))?;
        if docstatus != 1 {
            return Err(Error::field("sales_order", "is not submitted yet"));
        }
        if ships_from != id(doc, "store") {
            return Err(Error::field("store", "is not where the order ships from"));
        }
        let other: Option<String> = sqlx::query_scalar(
            "select number from e_collect_order where tenant_id = $1 and sales_order = $2 and id <> $3 and status in ('Ordered', 'Ready', 'Collected') limit 1",
        )
        .bind(hc.ctx.tenant)
        .bind(order)
        .bind(id(doc, "id").unwrap_or_else(Uuid::nil))
        .fetch_optional(&mut **hc.tx)
        .await?;
        if let Some(n) = other {
            return Err(Error::field("sales_order", format!("is collected under {n}")));
        }
        if let Some(n) = shipping(hc.tx, hc.ctx.tenant, order, None).await? {
            return Err(Error::field("sales_order", format!("is shipped from a store under {n}")));
        }
        Ok(())
    }

    async fn action(&self, hc: &mut HookCtx<'_>, name: &str, doc: &Doc, params: &Value) -> Result<Option<Value>> {
        let collect = id(doc, "id").ok_or_else(|| Error::internal("a collection without an id"))?;
        let system = elevated(hc.ctx);
        let now = Utc::now().to_rfc3339();
        match name {
            "bring_from" => {
                let from: Uuid = params
                    .get("from_store")
                    .and_then(Value::as_str)
                    .and_then(|v| v.parse().ok())
                    .ok_or_else(|| Error::field("from_store", "names the store to bring it from"))?;
                let (order, store) = (id(doc, "sales_order").unwrap_or_default(), id(doc, "store").unwrap_or_default());
                if from == store {
                    return Err(Error::field("from_store", "is another store than the one it is collected from"));
                }
                if let Some(t) = id(doc, "transfer") {
                    let open: Option<String> = sqlx::query_scalar(
                        "select number from e_store_transfer where tenant_id = $1 and id = $2 and status in ('Requested', 'Sent', 'Part Received')",
                    )
                    .bind(hc.ctx.tenant)
                    .bind(t)
                    .fetch_optional(&mut **hc.tx)
                    .await?
                    .flatten();
                    if let Some(n) = open {
                        return Err(Error::invalid(format!("{n} is already bringing it")));
                    }
                }
                let so = hc.kernel.get_tx(&system, hc.tx, "SalesOrder", order, false).await?;
                let need = to_deliver(hc.tx, hc.ctx.tenant, &so).await?;
                let short = short_of(hc.tx, hc.ctx.tenant, store, &need).await?;
                if short.is_empty() {
                    return Err(Error::invalid("the store holds all of it already"));
                }
                let lines: Vec<Value> = short.iter().map(|(item, _, have, need)| json!({"item": item.to_string(), "qty": money(*need - *have)})).collect();
                let note = format!("For click and collect {}", s(doc, "number").unwrap_or_default());
                // Asked by the person, as if they had asked themselves.
                let asked = hc
                    .kernel
                    .insert_tx(
                        hc.ctx,
                        hc.tx,
                        "StoreTransfer",
                        obj(json!({"from_store": from.to_string(), "to_store": store.to_string(), "items": lines, "note": note})),
                    )
                    .await?;
                let transfer = id(&asked, "id").ok_or_else(|| Error::internal("a transfer without an id"))?;
                hc.kernel.set_values_tx(&system, hc.tx, "CollectOrder", collect, obj(json!({"transfer": transfer.to_string()}))).await?;
                Ok(Some(json!({"transfer": created("StoreTransfer", &asked)})))
            }
            "make_ready" => {
                let (order, store) = (id(doc, "sales_order").unwrap_or_default(), id(doc, "store").unwrap_or_default());
                let so = hc.kernel.get_tx(&system, hc.tx, "SalesOrder", order, false).await?;
                let need = to_deliver(hc.tx, hc.ctx.tenant, &so).await?;
                let short = short_of(hc.tx, hc.ctx.tenant, store, &need).await?;
                if !short.is_empty() {
                    return Err(Error::invalid(format!("not in the store yet: {}", say(&short))));
                }
                hc.kernel.set_values_tx(&system, hc.tx, "CollectOrder", collect, obj(json!({"status": "Ready", "ready_at": now}))).await?;
                Ok(Some(json!({"status": "Ready"})))
            }
            "hand_over" => {
                let given = params.get("code").and_then(Value::as_str).map(str::trim).unwrap_or_default();
                if given.is_empty() || Some(given) != s(doc, "pickup_code") {
                    return Err(Error::field("code", "the pickup code does not match"));
                }
                let order = id(doc, "sales_order").unwrap_or_default();
                let made = hc.kernel.run_action_tx(&system, hc.tx, "SalesOrder", order, "make_delivery_note", &json!({})).await?;
                let note: Uuid =
                    made.get("id").and_then(Value::as_str).and_then(|v| v.parse().ok()).ok_or_else(|| Error::internal("no delivery note was made"))?;
                let submitted = hc.kernel.submit_tx(&system, hc.tx, "DeliveryNote", note, None).await?;
                hc.kernel
                    .set_values_tx(
                        &system,
                        hc.tx,
                        "CollectOrder",
                        collect,
                        obj(json!({"status": "Collected", "collected_at": now, "delivery_note": note.to_string()})),
                    )
                    .await?;
                Ok(Some(json!({"status": "Collected", "delivery_note": created("DeliveryNote", &submitted)})))
            }
            "cancel_collect" => {
                hc.kernel.set_values_tx(&system, hc.tx, "CollectOrder", collect, obj(json!({"status": "Cancelled"}))).await?;
                Ok(Some(json!({"status": "Cancelled"})))
            }
            _ => Ok(None),
        }
    }
}
