//! Franchises. A franchisee reports its sales for a period within its agreement, one report to any
//! day; the royalty and the marketing fee are its percentages of them, invoiced through ERP to the
//! franchisee's customer, once.

use crate::obj;
use async_trait::async_trait;
use ferrum_domain::flow::created;
use ferrum_domain::util::{d, elevated, id, money, round, s};
use ferrum_kernel::{Doc, DocHook, Error, HookCtx, Result};
use rust_decimal::Decimal;
use serde_json::{Value, json};
use uuid::Uuid;

pub struct FranchiseeHook;

#[async_trait]
impl DocHook for FranchiseeHook {
    fn name(&self) -> &'static str {
        "retail.franchisee"
    }

    async fn validate(&self, _hc: &mut HookCtx<'_>, doc: &Doc) -> Result<()> {
        if s(doc, "ends_on") <= s(doc, "starts_on") {
            return Err(Error::field("ends_on", "is after the agreement starts"));
        }
        let pct = |k| d(doc, k);
        if !(Decimal::ZERO..=Decimal::ONE_HUNDRED).contains(&pct("royalty_percent"))
            || !(Decimal::ZERO..=Decimal::ONE_HUNDRED).contains(&pct("marketing_percent"))
        {
            return Err(Error::field("royalty_percent", "percentages are from 0 to 100"));
        }
        Ok(())
    }
}

pub struct ReportHook;

#[async_trait]
impl DocHook for ReportHook {
    fn name(&self) -> &'static str {
        "retail.royalty"
    }

    fn fills(&self) -> &'static [&'static str] {
        &["royalty", "marketing", "total"]
    }

    async fn before_validate(&self, hc: &mut HookCtx<'_>, doc: &mut Doc) -> Result<()> {
        let rates: Option<(Option<Decimal>, Option<Decimal>)> =
            sqlx::query_as("select royalty_percent, marketing_percent from e_franchisee where tenant_id = $1 and id = $2")
                .bind(hc.ctx.tenant)
                .bind(id(doc, "franchisee"))
                .fetch_optional(&mut **hc.tx)
                .await?;
        let (r, m) = rates.map(|(r, m)| (r.unwrap_or_default(), m.unwrap_or_default())).unwrap_or_default();
        let sales = d(doc, "gross_sales");
        let royalty = round(sales * r / Decimal::ONE_HUNDRED, 2);
        let marketing = round(sales * m / Decimal::ONE_HUNDRED, 2);
        doc.insert("royalty".into(), money(royalty));
        doc.insert("marketing".into(), money(marketing));
        doc.insert("total".into(), money(royalty + marketing));
        Ok(())
    }

    async fn validate(&self, hc: &mut HookCtx<'_>, doc: &Doc) -> Result<()> {
        if d(doc, "gross_sales") < Decimal::ZERO {
            return Err(Error::field("gross_sales", "is zero or more"));
        }
        let (from, to) = (s(doc, "period_from").unwrap_or_default(), s(doc, "period_to").unwrap_or_default());
        if to < from {
            return Err(Error::field("period_to", "is on or after the period starts"));
        }
        let agreement: Option<(Option<chrono::NaiveDate>, Option<chrono::NaiveDate>)> =
            sqlx::query_as("select starts_on, ends_on from e_franchisee where tenant_id = $1 and id = $2")
                .bind(hc.ctx.tenant)
                .bind(id(doc, "franchisee"))
                .fetch_optional(&mut **hc.tx)
                .await?;
        let (starts, ends) = agreement.ok_or_else(|| Error::field("franchisee", "is required"))?;
        let day = |v: &str| chrono::NaiveDate::parse_from_str(v, "%Y-%m-%d").ok();
        if starts.zip(day(from)).is_some_and(|(a, f)| f < a) || ends.zip(day(to)).is_some_and(|(e, t)| t > e) {
            return Err(Error::field("period_from", "falls outside the franchise agreement"));
        }
        let other: Option<String> = sqlx::query_scalar(
            "select number from e_royalty_report where tenant_id = $1 and franchisee = $2 and id <> $3 and period_from <= $5::date and period_to >= $4::date limit 1",
        )
        .bind(hc.ctx.tenant)
        .bind(id(doc, "franchisee"))
        .bind(id(doc, "id").unwrap_or_else(Uuid::nil))
        .bind(from)
        .bind(to)
        .fetch_optional(&mut **hc.tx)
        .await?;
        if let Some(n) = other {
            return Err(Error::field("period_from", format!("{n} reports some of these days already")));
        }
        Ok(())
    }

    async fn validate_change(&self, hc: &mut HookCtx<'_>, old: Option<&Doc>, _doc: &Doc) -> Result<()> {
        if !hc.ctx.is_system && old.is_some_and(|o| s(o, "status") == Some("Invoiced")) {
            return Err(Error::invalid("an invoiced report is not changed"));
        }
        Ok(())
    }

    async fn action(&self, hc: &mut HookCtx<'_>, name: &str, doc: &Doc, _params: &Value) -> Result<Option<Value>> {
        if name != "invoice_royalty" {
            return Ok(None);
        }
        let report = id(doc, "id").ok_or_else(|| Error::internal("a report without an id"))?;
        let settings: Option<(Option<Uuid>, Option<Uuid>, Option<Uuid>)> =
            sqlx::query_as("select company, royalty_item, marketing_item from e_retail_settings where tenant_id = $1 limit 1")
                .bind(hc.ctx.tenant)
                .fetch_optional(&mut **hc.tx)
                .await?;
        let Some((Some(company), Some(royalty_item), marketing_item)) = settings else {
            return Err(Error::invalid("set the company and the royalty item in Retail Settings first"));
        };
        let customer: Option<Uuid> = sqlx::query_scalar("select customer from e_franchisee where tenant_id = $1 and id = $2")
            .bind(hc.ctx.tenant)
            .bind(id(doc, "franchisee"))
            .fetch_optional(&mut **hc.tx)
            .await?
            .flatten();
        let customer = customer.ok_or_else(|| Error::field("franchisee", "has no customer"))?;
        let period = format!("{} to {}", s(doc, "period_from").unwrap_or_default(), s(doc, "period_to").unwrap_or_default());
        let line = |item: Uuid, amount: Decimal, what: &str| json!({"item": item.to_string(), "qty": "1", "price_list_rate": money(amount), "discount_percent": "0", "description": format!("{what}, {period}")});
        let mut items = vec![line(royalty_item, d(doc, "royalty"), "Royalty")];
        if d(doc, "marketing") > Decimal::ZERO {
            items.push(line(marketing_item.unwrap_or(royalty_item), d(doc, "marketing"), "Marketing fee"));
        }
        let system = elevated(hc.ctx);
        let currency: Option<Uuid> = sqlx::query_scalar("select default_currency from e_company where tenant_id = $1 and id = $2")
            .bind(hc.ctx.tenant)
            .bind(company)
            .fetch_optional(&mut **hc.tx)
            .await?
            .flatten();
        let inv = hc
            .kernel
            .insert_tx(
                &system,
                hc.tx,
                "SalesInvoice",
                obj(json!({"company": company.to_string(), "customer": customer.to_string(), "currency": currency.map(|c| c.to_string()), "items": items,
                           "remarks": format!("Franchise fees, {period}")})),
            )
            .await?;
        hc.kernel.set_values_tx(&system, hc.tx, "RoyaltyReport", report, obj(json!({"status": "Invoiced", "sales_invoice": inv.get("id")}))).await?;
        Ok(Some(json!({"status": "Invoiced", "sales_invoice": created("SalesInvoice", &inv)})))
    }
}
