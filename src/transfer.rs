//! Store-to-store transfers. A store asks another for stock; the giving store sends it, the stock
//! leaving it for the goods in transit (the settings' transit warehouse), and the store that asked
//! takes it in. Without a transit warehouse, what is sent arrives at once. Each move is an ERP stock
//! entry, submitted, and a store sends only what it holds. Once sent, a transfer is kept as it was.

use crate::collect::on_hand;
use crate::obj;
use async_trait::async_trait;
use chrono::Utc;
use ferrum_domain::util::{d, elevated, id, rows, s};
use ferrum_kernel::{Ctx, Doc, DocHook, Error, HookCtx, Kernel, Result, Tx};
use rust_decimal::Decimal;
use serde_json::{Value, json};
use std::collections::HashSet;
use uuid::Uuid;

pub struct TransferHook;

/// A submitted stock entry moving `lines` between two warehouses, (from, to): its id.
async fn move_stock(k: &Kernel, ctx: &Ctx, tx: &mut Tx, company: Uuid, lines: &[Doc], (from, to): (Uuid, Uuid), remarks: &str) -> Result<Uuid> {
    let items: Vec<Value> = lines
        .iter()
        .map(|l| json!({"item": s(l, "item"), "qty": l.get("qty"), "source_warehouse": from.to_string(), "target_warehouse": to.to_string()}))
        .collect();
    let entry = k
        .insert_tx(ctx, tx, "StockEntry", obj(json!({"company": company.to_string(), "purpose": "Material Transfer", "items": items, "remarks": remarks})))
        .await?;
    let entry = id(&entry, "id").ok_or_else(|| Error::internal("a stock entry without an id"))?;
    k.submit_tx(ctx, tx, "StockEntry", entry, None).await?;
    Ok(entry)
}

#[async_trait]
impl DocHook for TransferHook {
    fn name(&self) -> &'static str {
        "retail.transfer"
    }

    fn fills(&self) -> &'static [&'static str] {
        &["requested_by"]
    }

    async fn before_validate(&self, hc: &mut HookCtx<'_>, doc: &mut Doc) -> Result<()> {
        if s(doc, "requested_by").is_none() && !hc.ctx.email.is_empty() {
            doc.insert("requested_by".into(), json!(hc.ctx.email.to_lowercase()));
        }
        Ok(())
    }

    async fn validate(&self, _hc: &mut HookCtx<'_>, doc: &Doc) -> Result<()> {
        if id(doc, "from_store") == id(doc, "to_store") {
            return Err(Error::field("to_store", "is another store than the one it comes from"));
        }
        let lines = rows(doc, "items");
        if lines.is_empty() {
            return Err(Error::field("items", "a transfer moves something"));
        }
        let mut seen = HashSet::new();
        for l in &lines {
            if d(l, "qty") <= Decimal::ZERO {
                return Err(Error::field("items", "each line moves more than nothing"));
            }
            if !seen.insert(id(l, "item")) {
                return Err(Error::field("items", "an item is on one line"));
            }
        }
        Ok(())
    }

    async fn validate_change(&self, hc: &mut HookCtx<'_>, old: Option<&Doc>, _doc: &Doc) -> Result<()> {
        if !hc.ctx.is_system && old.is_some_and(|o| s(o, "status") != Some("Requested")) {
            return Err(Error::invalid("a transfer past its request is kept as it was"));
        }
        Ok(())
    }

    async fn action(&self, hc: &mut HookCtx<'_>, name: &str, doc: &Doc, params: &Value) -> Result<Option<Value>> {
        let transfer = id(doc, "id").ok_or_else(|| Error::internal("a transfer without an id"))?;
        let system = elevated(hc.ctx);
        let now = Utc::now().to_rfc3339();
        let (from, to) = (id(doc, "from_store").unwrap_or_default(), id(doc, "to_store").unwrap_or_default());
        let settings: Option<(Option<Uuid>, Option<Uuid>)> =
            sqlx::query_as("select company, transit_warehouse from e_retail_settings where tenant_id = $1 limit 1")
                .bind(hc.ctx.tenant)
                .fetch_optional(&mut **hc.tx)
                .await?;
        let remarks = format!("Store transfer {}", s(doc, "number").unwrap_or_default());
        let values = match name {
            "send_transfer" => {
                let Some((Some(company), transit)) = settings else {
                    return Err(Error::invalid("set the company in Retail Settings first"));
                };
                let lines = rows(doc, "items");
                let mut short = vec![];
                for l in &lines {
                    let Some(item) = id(l, "item") else { continue };
                    let have = on_hand(hc.tx, hc.ctx.tenant, from, item).await?;
                    if have < d(l, "qty") {
                        short.push(format!("{} ({} of {})", s(l, "item_name").unwrap_or("an item"), have.normalize(), d(l, "qty").normalize()));
                    }
                }
                if !short.is_empty() {
                    return Err(Error::invalid(format!("the store does not hold it: {}", short.join(", "))));
                }
                let entry = move_stock(hc.kernel, &system, hc.tx, company, &lines, (from, transit.unwrap_or(to)), &remarks).await?;
                match transit {
                    Some(_) => json!({"status": "Sent", "sent_at": now, "sent_entry": entry.to_string()}),
                    // Nowhere in between: it arrives as it leaves.
                    None => {
                        json!({"status": "Received", "sent_at": now, "received_at": now, "sent_entry": entry.to_string(), "received_entry": entry.to_string()})
                    }
                }
            }
            "receive_transfer" => {
                let Some((Some(company), Some(transit))) = settings else {
                    return Err(Error::invalid("the settings name no goods in transit to take it from"));
                };
                let entry = move_stock(hc.kernel, &system, hc.tx, company, &rows(doc, "items"), (transit, to), &remarks).await?;
                json!({"status": "Received", "received_at": now, "received_entry": entry.to_string()})
            }
            "reject_transfer" => {
                let why = params
                    .get("why")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|w| !w.is_empty())
                    .ok_or_else(|| Error::field("why", "turning it down says why"))?;
                let note = match s(doc, "note") {
                    Some(n) => format!("{n}\nTurned down: {why}"),
                    None => format!("Turned down: {why}"),
                };
                json!({"status": "Rejected", "note": note})
            }
            "cancel_transfer" => json!({"status": "Cancelled"}),
            _ => return Ok(None),
        };
        let status = values["status"].clone();
        hc.kernel.set_values_tx(&system, hc.tx, "StoreTransfer", transfer, obj(values)).await?;
        Ok(Some(json!({"status": status})))
    }
}
