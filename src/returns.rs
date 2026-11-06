//! Returns to any store. Goods bought online or in another store come back to whichever store the
//! customer brings them to, against the invoice that sold them, never more of an item than it sold
//! less what was credited already. Taken back, the invoice is credited for them, at what was paid,
//! and they go into the store's stock at the cost they left at: with the credit note itself when the
//! invoice delivered them, or else by a return delivery note against each note that delivered them,
//! as Commerce's returns do, but into this store.

use crate::collect::is_store;
use crate::obj;
use async_trait::async_trait;
use chrono::Utc;
use ferrum_domain::flow::created;
use ferrum_domain::util::{b, d, elevated, id, money, rows, s};
use ferrum_kernel::{Ctx, Doc, DocHook, Error, HookCtx, Result, Tx};
use rust_decimal::Decimal;
use serde_json::{Value, json};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

pub struct ReturnHook;

/// What an invoice can still take back of each item it sold: sold less credited, in its lines' units.
async fn returnable(tx: &mut Tx, tenant: Uuid, invoice: Uuid) -> Result<HashMap<Uuid, Decimal>> {
    let rows: Vec<(Uuid, Decimal)> = sqlx::query_as(
        "select i.item, i.qty from e_sales_invoice__items i where i.tenant_id = $1 and i.parent_id = $2 \
         union all select c.item, c.qty from e_sales_invoice__items c join e_sales_invoice s on s.tenant_id = c.tenant_id and s.id = c.parent_id \
         where c.tenant_id = $1 and s.docstatus = 1 and coalesce(s.is_return, false) and s.return_against = $2",
    )
    .bind(tenant)
    .bind(invoice)
    .fetch_all(&mut **tx)
    .await?;
    let mut room = HashMap::new();
    for (item, qty) in rows {
        // A credit note's lines are negative.
        *room.entry(item).or_insert(Decimal::ZERO) += qty;
    }
    Ok(room)
}

/// The return's lines, checked against what its invoice can still take back: (item, name, quantity).
async fn checked(tx: &mut Tx, tenant: Uuid, invoice: Uuid, doc: &Doc) -> Result<Vec<(Uuid, String, Decimal)>> {
    let lines = rows(doc, "items");
    if lines.is_empty() {
        return Err(Error::field("items", "a return brings something back"));
    }
    let room = returnable(tx, tenant, invoice).await?;
    let (mut seen, mut out) = (HashSet::new(), vec![]);
    for l in &lines {
        let Some(item) = id(l, "item") else { continue };
        let qty = d(l, "qty");
        let name: String = match s(l, "item_name") {
            Some(n) => n.to_string(),
            None => sqlx::query_scalar("select item_name from e_item where tenant_id = $1 and id = $2")
                .bind(tenant)
                .bind(item)
                .fetch_optional(&mut **tx)
                .await?
                .flatten()
                .unwrap_or_else(|| "an item".to_string()),
        };
        if qty <= Decimal::ZERO {
            return Err(Error::field("items", "each line brings back more than nothing"));
        }
        if !seen.insert(item) {
            return Err(Error::field("items", "an item is on one line"));
        }
        match room.get(&item) {
            None => return Err(Error::field("items", format!("{name} is not on the invoice"))),
            Some(left) if qty > *left => {
                return Err(Error::field("items", format!("{name}: {} can still come back on this invoice", left.max(&Decimal::ZERO).normalize())));
            }
            Some(_) => out.push((item, name, qty)),
        }
    }
    Ok(out)
}

/// Whether `item` is kept in stock.
async fn stocked(tx: &mut Tx, tenant: Uuid, item: Uuid) -> Result<bool> {
    let kept: Option<bool> = sqlx::query_scalar("select coalesce(is_stock_item, false) from e_item where tenant_id = $1 and id = $2")
        .bind(tenant)
        .bind(item)
        .fetch_optional(&mut **tx)
        .await?;
    Ok(kept.unwrap_or(false))
}

/// The goods of `invoice` that delivery notes sent, put back into `store` by a return delivery note
/// against each note that sent them, newest first, at the cost they left at: the notes' numbers.
async fn return_notes(hc: &mut HookCtx<'_>, system: &Ctx, invoice: Uuid, store: Uuid, back: &[(Uuid, String, Decimal)]) -> Result<Vec<String>> {
    let tenant = hc.ctx.tenant;
    // (note, its line, quantity) for each item, from the lines that billed it: by the note line an
    // invoice line names, or else by the order line both name.
    let mut picks: Vec<(Uuid, Uuid, Uuid, Decimal)> = vec![];
    for (item, name, qty) in back {
        if !stocked(hc.tx, tenant, *item).await? {
            continue;
        }
        let sent: Vec<(Uuid, Uuid, Decimal)> = sqlx::query_as(
            "select n.id, i.id, i.qty from e_delivery_note__items i join e_delivery_note n on n.tenant_id = i.tenant_id and n.id = i.parent_id \
             where i.tenant_id = $1 and n.docstatus = 1 and not coalesce(n.is_return, false) and i.item = $3 and ( \
               i.id::text in (select si.dn_line from e_sales_invoice__items si where si.tenant_id = $1 and si.parent_id = $2 and si.dn_line is not null) \
               or i.so_line in (select si.so_line from e_sales_invoice__items si where si.tenant_id = $1 and si.parent_id = $2 and si.so_line is not null)) \
             order by n.posting_date desc, n.created_at desc, i.idx",
        )
        .bind(tenant)
        .bind(invoice)
        .bind(item)
        .fetch_all(&mut **hc.tx)
        .await?;
        let notes: Vec<Uuid> = sent.iter().map(|x| x.0).collect();
        let came_back: Vec<(Uuid, Decimal)> = sqlx::query_as(
            "select n.return_against, -sum(c.qty) from e_delivery_note__items c join e_delivery_note n on n.tenant_id = c.tenant_id and n.id = c.parent_id \
             where c.tenant_id = $1 and n.docstatus = 1 and coalesce(n.is_return, false) and n.return_against = any($2) and c.item = $3 group by n.return_against",
        )
        .bind(tenant)
        .bind(&notes)
        .bind(item)
        .fetch_all(&mut **hc.tx)
        .await?;
        let mut came_back: HashMap<Uuid, Decimal> = came_back.into_iter().collect();
        let mut left = *qty;
        for (note, line, sent_qty) in sent {
            if left <= Decimal::ZERO {
                break;
            }
            // What came back against the note is taken off its lines in turn.
            let already = came_back.entry(note).or_default();
            let used = (*already).min(sent_qty);
            *already -= used;
            let take = (sent_qty - used).min(left);
            if take > Decimal::ZERO {
                picks.push((note, line, *item, take));
                left -= take;
            }
        }
        if left > Decimal::ZERO {
            return Err(Error::invalid(format!("{name}: only {} of it was delivered and not yet returned", (*qty - left).normalize())));
        }
    }
    let mut numbers = vec![];
    let mut notes: Vec<Uuid> = picks.iter().map(|p| p.0).collect();
    notes.dedup();
    for note in notes {
        let from = hc.kernel.get_tx(system, hc.tx, "DeliveryNote", note, false).await?;
        let mut input = Doc::new();
        for f in ["company", "customer", "currency", "conversion_rate", "price_list", "tax_template", "ship_to", "customer_address", "discount_percent"] {
            if let Some(v) = from.get(f).filter(|v| !v.is_null()) {
                input.insert(f.to_string(), v.clone());
            }
        }
        input.insert("is_return".into(), json!(true));
        input.insert("return_against".into(), json!(note.to_string()));
        input.insert("set_warehouse".into(), json!(store.to_string()));
        let lines: Vec<Value> = rows(&from, "items")
            .iter()
            .filter_map(|l| {
                let (_, _, _, q) = picks.iter().find(|p| p.0 == note && Some(p.1) == id(l, "id"))?;
                let mut r = Doc::new();
                for f in ["item", "uom", "price_list_rate", "discount_percent", "batch_no"] {
                    if let Some(v) = l.get(f).filter(|v| !v.is_null()) {
                        r.insert(f.to_string(), v.clone());
                    }
                }
                r.insert("qty".into(), money(-*q));
                r.insert("warehouse".into(), json!(store.to_string()));
                Some(Value::Object(r))
            })
            .collect();
        input.insert("items".into(), Value::Array(lines));
        let draft = hc.kernel.insert_tx(system, hc.tx, "DeliveryNote", input).await?;
        let made = id(&draft, "id").ok_or_else(|| Error::internal("a return without an id"))?;
        let done = hc.kernel.submit_tx(system, hc.tx, "DeliveryNote", made, None).await?;
        numbers.push(s(&done, "number").unwrap_or_default().to_string());
    }
    Ok(numbers)
}

#[async_trait]
impl DocHook for ReturnHook {
    fn name(&self) -> &'static str {
        "retail.return"
    }

    fn fills(&self) -> &'static [&'static str] {
        &["customer"]
    }

    async fn before_validate(&self, hc: &mut HookCtx<'_>, doc: &mut Doc) -> Result<()> {
        if let Some(invoice) = id(doc, "sales_invoice") {
            let customer: Option<Uuid> = sqlx::query_scalar("select customer from e_sales_invoice where tenant_id = $1 and id = $2")
                .bind(hc.ctx.tenant)
                .bind(invoice)
                .fetch_optional(&mut **hc.tx)
                .await?
                .flatten();
            doc.insert("customer".into(), json!(customer.map(|c| c.to_string())));
        }
        Ok(())
    }

    async fn validate(&self, hc: &mut HookCtx<'_>, doc: &Doc) -> Result<()> {
        if s(doc, "status").is_some_and(|st| st != "Draft") {
            return Ok(());
        }
        let invoice = id(doc, "sales_invoice").ok_or_else(|| Error::field("sales_invoice", "is required"))?;
        let row: Option<(i16, bool, Option<Uuid>)> =
            sqlx::query_as("select docstatus, coalesce(is_return, false), company from e_sales_invoice where tenant_id = $1 and id = $2")
                .bind(hc.ctx.tenant)
                .bind(invoice)
                .fetch_optional(&mut **hc.tx)
                .await?;
        let (docstatus, is_return, company) = row.ok_or_else(|| Error::field("sales_invoice", "is not this workspace's"))?;
        if docstatus != 1 {
            return Err(Error::field("sales_invoice", "is not issued"));
        }
        if is_return {
            return Err(Error::field("sales_invoice", "is a credit note: return against the invoice that sold it"));
        }
        let store = id(doc, "store").ok_or_else(|| Error::field("store", "is required"))?;
        if !is_store(hc.tx, hc.ctx.tenant, store, company).await? {
            return Err(Error::field("store", "is not a store of the invoice's company (Is A Store, on the warehouse)"));
        }
        checked(hc.tx, hc.ctx.tenant, invoice, doc).await?;
        Ok(())
    }

    async fn validate_change(&self, hc: &mut HookCtx<'_>, old: Option<&Doc>, _doc: &Doc) -> Result<()> {
        if !hc.ctx.is_system && old.is_some_and(|o| s(o, "status") != Some("Draft")) {
            return Err(Error::invalid("a return taken back or cancelled is kept as it was"));
        }
        Ok(())
    }

    async fn action(&self, hc: &mut HookCtx<'_>, name: &str, doc: &Doc, _params: &Value) -> Result<Option<Value>> {
        let ret = id(doc, "id").ok_or_else(|| Error::internal("a return without an id"))?;
        let system = elevated(hc.ctx);
        match name {
            "take_back" => {
                let (invoice, store) = (id(doc, "sales_invoice").unwrap_or_default(), id(doc, "store").unwrap_or_default());
                // Checked again: another return may have taken some back since.
                let back = checked(hc.tx, hc.ctx.tenant, invoice, doc).await?;
                let sold = hc.kernel.get_tx(&system, hc.tx, "SalesInvoice", invoice, false).await?;
                let delivered_by_invoice = b(&sold, "update_stock");
                // ERP's credit note for the invoice, cut to what comes back.
                let made = hc.kernel.run_action_tx(&system, hc.tx, "SalesInvoice", invoice, "make_credit_note", &json!({})).await?;
                let note: Uuid =
                    made.get("id").and_then(Value::as_str).and_then(|v| v.parse().ok()).ok_or_else(|| Error::internal("no credit note was made"))?;
                let draft = hc.kernel.get_tx(&system, hc.tx, "SalesInvoice", note, false).await?;
                let mut left: HashMap<Uuid, Decimal> = back.iter().map(|(item, _, q)| (*item, *q)).collect();
                let mut lines = vec![];
                for mut l in rows(&draft, "items") {
                    let Some(want) = id(&l, "item").and_then(|item| left.get_mut(&item)) else { continue };
                    let q = (*want).min(-d(&l, "qty"));
                    if q <= Decimal::ZERO {
                        continue;
                    }
                    *want -= q;
                    l.insert("qty".into(), money(-q));
                    if delivered_by_invoice {
                        l.insert("warehouse".into(), json!(store.to_string()));
                    }
                    lines.push(Value::Object(l));
                }
                let remarks = format!("Brought back to a store: {}", s(doc, "number").unwrap_or_default());
                let patch = json!({"items": lines, "update_stock": delivered_by_invoice,
                                   "set_warehouse": delivered_by_invoice.then(|| store.to_string()), "remarks": remarks});
                hc.kernel.update_tx(&system, hc.tx, "SalesInvoice", note, obj(patch), None).await?;
                let notes = if delivered_by_invoice { vec![] } else { return_notes(hc, &system, invoice, store, &back).await? };
                let credited = hc.kernel.submit_tx(&system, hc.tx, "SalesInvoice", note, None).await?;
                let values = json!({"status": "Taken Back", "credit_note": note.to_string(), "goods_back_by": (!notes.is_empty()).then(|| notes.join(", ")),
                                    "taken_at": Utc::now().to_rfc3339(), "taken_by": (!hc.ctx.email.is_empty()).then(|| hc.ctx.email.to_lowercase())});
                hc.kernel.set_values_tx(&system, hc.tx, "StoreReturn", ret, obj(values)).await?;
                Ok(Some(json!({"status": "Taken Back", "credit_note": created("SalesInvoice", &credited), "goods_back_by": notes})))
            }
            "cancel_return" => {
                hc.kernel.set_values_tx(&system, hc.tx, "StoreReturn", ret, obj(json!({"status": "Cancelled"}))).await?;
                Ok(Some(json!({"status": "Cancelled"})))
            }
            _ => Ok(None),
        }
    }
}
