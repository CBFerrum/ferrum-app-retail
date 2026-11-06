//! Retail's page of the workspace's portal (Ferrum docs/plan/apps-ui.md §9.1), "Your franchise" at
//! `/portal/franchise/`, from `/api/app/retail/portal/…`: a franchisee's agreements, the sales they
//! reported and the fees invoiced on them, and a new report of a period's sales. Portal people hold
//! no roles: their agreements are those of the customers their address belongs to, and a report
//! they make runs as the system after these checks, with the same rules as one made in the office.

use ferrum_domain::util::{b, id, s, sys};
use ferrum_kernel::{Ctx, Doc, Error, Filter, Kernel, Result, Site, SiteRequest, SiteResponse, Tx, with_retry};
use serde_json::{Map, Value, json};
use uuid::Uuid;

pub struct RetailSite;

#[async_trait::async_trait]
impl Site for RetailSite {
    async fn handle(&self, k: &Kernel, req: SiteRequest) -> Result<SiteResponse> {
        let user = req.user.as_ref().ok_or_else(|| Error::Unauthorized("sign in first".into()))?;
        let segs: Vec<&str> = req.path.split('/').filter(|p| !p.is_empty()).collect();
        let (status, out) = match (req.method.as_str(), segs.as_slice()) {
            ("GET", ["portal", "franchise"]) => (200, franchise(k, user).await?),
            ("POST", ["portal", "reports"]) => (201, report(k, user, &body(&req)?).await?),
            _ => return Err(Error::NotFound(format!("retail endpoint {} {}", req.method, req.path))),
        };
        let mut r = SiteResponse::json(status, &out);
        r.headers.push(("cache-control".into(), "no-store".into()));
        Ok(r)
    }
}

fn body(req: &SiteRequest) -> Result<Map<String, Value>> {
    match serde_json::from_slice::<Value>(&req.body) {
        Ok(Value::Object(m)) => Ok(m),
        _ => Err(Error::invalid("the request body must be a JSON object")),
    }
}

/// The customers an address belongs to: the customer's own email, or one of its contacts'.
async fn customers_of(k: &Kernel, sctx: &Ctx, tx: &mut Tx, email: &str) -> Result<Vec<Uuid>> {
    let mut out: Vec<Uuid> = k
        .find_tx(sctx, tx, "Customer", &[Filter::eq("email", email)], Some("created_at asc"), 20)
        .await?
        .iter()
        .filter(|c| !b(c, "disabled"))
        .filter_map(|c| id(c, "id"))
        .collect();
    if sctx.schema.entity("Contact").is_some_and(|c| c.field("emails").is_some() && c.field("customer").is_some()) {
        let contacts = k.find_tx(sctx, tx, "Contact", &[Filter::eq("emails.email", email)], Some("created_at asc"), 20).await?;
        out.extend(contacts.iter().filter_map(|c| id(c, "customer")));
    }
    out.sort();
    out.dedup();
    Ok(out)
}

/// The caller's franchise agreements, the latest first.
async fn agreements(k: &Kernel, sctx: &Ctx, tx: &mut Tx, email: &str) -> Result<Vec<Doc>> {
    let mut out = vec![];
    for c in customers_of(k, sctx, tx, email).await? {
        out.extend(k.find_tx(sctx, tx, "Franchisee", &[Filter::eq("customer", c.to_string())], Some("starts_on desc"), 20).await?);
    }
    Ok(out)
}

/// `GET portal/franchise`: the caller's agreements, each with the sales reported and the fees on
/// them; an invoice opens on ERP's "Your invoices" once it is issued.
async fn franchise(k: &Kernel, user: &Ctx) -> Result<Value> {
    let email = user.email.to_lowercase();
    if email.is_empty() {
        return Ok(json!({"agreements": []}));
    }
    let sctx = sys(user);
    let mut tx = k.db.tenant_tx(user.tenant).await?;
    let mut out = vec![];
    for a in agreements(k, &sctx, &mut tx, &email).await? {
        let reports =
            k.find_tx(&sctx, &mut tx, "RoyaltyReport", &[Filter::eq("franchisee", s(&a, "id").unwrap_or_default())], Some("period_from desc"), 120).await?;
        let mut lines = Vec::with_capacity(reports.len());
        for r in &reports {
            let issued = match id(r, "sales_invoice") {
                Some(si) => {
                    k.find_tx(&sctx, &mut tx, "SalesInvoice", &[Filter::eq("id", si.to_string()), Filter::eq("docstatus", 1)], None, 1).await?.len() == 1
                }
                None => false,
            };
            lines.push(json!({"id": s(r, "id"), "number": s(r, "number"), "period_from": s(r, "period_from"), "period_to": s(r, "period_to"),
                              "gross_sales": s(r, "gross_sales"), "royalty": s(r, "royalty"), "marketing": s(r, "marketing"), "total": s(r, "total"),
                              "status": s(r, "status"), "invoice": if issued { s(r, "sales_invoice") } else { None }, "issued": issued}));
        }
        out.push(json!({"id": s(&a, "id"), "territory": s(&a, "territory"), "royalty_percent": s(&a, "royalty_percent"),
                        "marketing_percent": s(&a, "marketing_percent"), "starts_on": s(&a, "starts_on"), "ends_on": s(&a, "ends_on"), "reports": lines}));
    }
    tx.commit().await?;
    Ok(json!({"agreements": out}))
}

/// `POST portal/reports` `{franchisee, period_from, period_to, gross_sales}`: a period's sales,
/// reported by the franchisee whose agreement it is.
async fn report(k: &Kernel, user: &Ctx, body: &Map<String, Value>) -> Result<Value> {
    let email = user.email.to_lowercase();
    if email.is_empty() {
        return Err(Error::Forbidden("this account has no email address to answer".into()));
    }
    let franchisee: Uuid =
        body.get("franchisee").and_then(Value::as_str).and_then(|f| f.parse().ok()).ok_or_else(|| Error::field("franchisee", "is required"))?;
    let text = |key: &str| body.get(key).and_then(|v| v.as_str().map(str::to_string).or_else(|| v.as_f64().map(|f| f.to_string())));
    let (from, to, sales) = (text("period_from"), text("period_to"), text("gross_sales"));
    if from.is_none() || to.is_none() || sales.is_none() {
        return Err(Error::invalid("a report gives its period, from and to, and its gross sales"));
    }
    k.rate_limit(
        &format!("retail|report|{}|{}", user.tenant, user.user),
        std::time::Duration::from_secs(3600),
        20,
        "too many reports in an hour; try again later",
    )
    .await?;
    let sctx = sys(user);
    with_retry(|| async {
        let mut tx = k.db.tenant_tx(user.tenant).await?;
        let theirs = agreements(k, &sctx, &mut tx, &email).await?.iter().any(|a| id(a, "id") == Some(franchisee));
        if !theirs {
            return Err(Error::field("franchisee", "is not an agreement of yours"));
        }
        let saved = k
            .insert_tx(
                &sctx,
                &mut tx,
                "RoyaltyReport",
                crate::obj(json!({"franchisee": franchisee.to_string(), "period_from": from, "period_to": to, "gross_sales": sales, "reported_by": email})),
            )
            .await?;
        tx.commit().await?;
        Ok(json!({"id": s(&saved, "id"), "number": s(&saved, "number"), "total": s(&saved, "total"), "status": s(&saved, "status")}))
    })
    .await
}
