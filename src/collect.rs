//! Click and collect. An order bought online is collected from the store it ships from, one
//! collection to an order. It is ready only once the store holds every line of it; it is handed
//! over only against its pickup code, when the order's delivery note is made and submitted, so the
//! goods leave the store's stock.

use crate::obj;
use async_trait::async_trait;
use chrono::Utc;
use ferrum_domain::flow::created;
use ferrum_domain::util::{d, elevated, id, rows, s};
use ferrum_kernel::{Doc, DocHook, Error, HookCtx, Result, Tx};
use rust_decimal::Decimal;
use serde_json::{Value, json};
use uuid::Uuid;

pub struct CollectHook;

/// The quantity of `item` in `warehouse`, from ERP's balances.
async fn on_hand(tx: &mut Tx, tenant: Uuid, warehouse: Uuid, item: Uuid) -> Result<Decimal> {
    Ok(sqlx::query_scalar("select coalesce(sum(qty), 0) from stock_balance where tenant_id = $1 and warehouse_id = $2 and item_id = $3")
        .bind(tenant)
        .bind(warehouse)
        .bind(item)
        .fetch_one(&mut **tx)
        .await?)
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
        Ok(())
    }

    async fn action(&self, hc: &mut HookCtx<'_>, name: &str, doc: &Doc, params: &Value) -> Result<Option<Value>> {
        let collect = id(doc, "id").ok_or_else(|| Error::internal("a collection without an id"))?;
        let system = elevated(hc.ctx);
        let now = Utc::now().to_rfc3339();
        match name {
            "make_ready" => {
                let (order, store) = (id(doc, "sales_order").unwrap_or_default(), id(doc, "store").unwrap_or_default());
                let so = hc.kernel.get_tx(&system, hc.tx, "SalesOrder", order, false).await?;
                let mut short = vec![];
                for line in rows(&so, "items") {
                    let Some(item) = id(&line, "item") else { continue };
                    let have = on_hand(hc.tx, hc.ctx.tenant, store, item).await?;
                    if have < d(&line, "qty") {
                        short.push(format!("{} ({} of {})", s(&line, "item_name").unwrap_or("an item"), have.normalize(), d(&line, "qty").normalize()));
                    }
                }
                if !short.is_empty() {
                    return Err(Error::invalid(format!("not in the store yet: {}", short.join(", "))));
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
