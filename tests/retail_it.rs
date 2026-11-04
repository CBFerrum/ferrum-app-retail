//! A chain of shops (FERRUM-INDUSTRY-003) against a real PostgreSQL
//! (`FERRUM_RETAIL_TEST_DATABASE_URL`, default `postgres://ferrum:ferrum@localhost:5432/ferrum_retail_test`;
//! reset first): an order bought online and collected in the store it ships from: ready only once
//! the store holds it, handed over only against its pickup code, its delivery note then made and
//! submitted so the goods leave the store; one collection to an order; franchisees' sales reported
//! for periods within their agreement, never two for a day, their royalty and marketing fee
//! invoiced through ERP once; and who may do what.

use ferrum_kernel::secret::SecretKey;
use ferrum_kernel::tenant::NewTenant;
use ferrum_kernel::{Ctx, Db, DbConfig, Doc, Error, HookRegistry, Kernel, KernelSettings, ListQuery};
use ferrum_meta::Catalog;
use serde_json::{Value, json};
use std::sync::Arc;
use uuid::Uuid;

fn obj(v: Value) -> Doc {
    v.as_object().cloned().unwrap()
}

fn uid(v: &Value) -> Uuid {
    v.as_str().unwrap_or_else(|| panic!("not an id: {v}")).parse().unwrap()
}

fn dec(v: &Value) -> f64 {
    v.as_str().map(|s| s.parse().unwrap()).or_else(|| v.as_f64()).unwrap_or_else(|| panic!("not a number: {v}"))
}

struct T {
    k: Arc<Kernel>,
    tenant: Uuid,
    admin: Ctx,
}

impl T {
    async fn insert(&self, ctx: &Ctx, entity: &str, v: Value) -> Doc {
        self.k.insert(ctx, entity, obj(v)).await.unwrap_or_else(|e| panic!("insert {entity}: {e:?}"))
    }
    async fn get(&self, entity: &str, id: Uuid) -> Doc {
        self.k.get(&self.admin, entity, id, false).await.unwrap_or_else(|e| panic!("get {entity}: {e:?}"))
    }
    async fn find_one(&self, entity: &str, filters: Value) -> Doc {
        let q = ListQuery { filters: ferrum_kernel::query::parse_filters(&filters).unwrap(), limit: Some(50), ..Default::default() };
        let rows = self.k.list(&self.admin, entity, &q).await.unwrap().rows;
        assert_eq!(rows.len(), 1, "{entity} {filters}");
        rows[0].clone()
    }
    async fn user(&self, email: &str, roles: &[&str]) -> Ctx {
        let roles: Vec<String> = roles.iter().map(|r| r.to_string()).collect();
        let id = self.k.create_user(&self.admin, email, email, Some("Correct-Horse-9"), &roles, false).await.expect("user");
        let s = self.k.login("shops", email, "Correct-Horse-9").await.expect("login");
        self.k.load_ctx(self.tenant, id, Some(s.token_version), "it".into()).await.expect("ctx")
    }
    async fn act(&self, ctx: &Ctx, entity: &str, id: Uuid, action: &str, params: Value) -> Result<Value, Error> {
        self.k.run_action(ctx, entity, id, action, params).await
    }
    /// The quantity of an item in a warehouse.
    async fn stock(&self, warehouse: Uuid, item: Uuid) -> f64 {
        let mut tx = self.k.db.tenant_tx(self.tenant).await.unwrap();
        let q: rust_decimal::Decimal =
            sqlx::query_scalar("select coalesce(sum(qty), 0) from stock_balance where tenant_id = $1 and warehouse_id = $2 and item_id = $3")
                .bind(self.tenant)
                .bind(warehouse)
                .bind(item)
                .fetch_one(&mut *tx)
                .await
                .unwrap();
        tx.commit().await.unwrap();
        q.to_string().parse().unwrap()
    }
}

fn refused<V: std::fmt::Debug>(r: Result<V, Error>, kind: &str, says: &str) {
    let msg = format!("{r:?}");
    let ok = matches!(
        (&r, kind),
        (Err(Error::Invalid(_) | Error::Validation(_)), "invalid") | (Err(Error::Forbidden(_)), "forbidden") | (Err(Error::NotFound(_)), "not found")
    );
    assert!(ok && msg.contains(says), "expected {kind} saying {says:?}, got {msg}");
}

#[test]
fn a_chain_of_shops() {
    let rt = tokio::runtime::Builder::new_multi_thread().enable_all().thread_stack_size(16 << 20).build().unwrap();
    rt.block_on(async { tokio::spawn(scenario()).await.unwrap() });
}

async fn scenario() {
    let url = std::env::var("FERRUM_RETAIL_TEST_DATABASE_URL").unwrap_or_else(|_| "postgres://ferrum:ferrum@localhost:5432/ferrum_retail_test".into());
    let db = Db::connect(DbConfig { url, max_connections: 16, ..Default::default() }).await.expect("connect");
    sqlx::raw_sql("drop schema if exists public cascade; create schema public;").execute(&db.pool).await.expect("reset");
    let mut packages = ferrum_domain::packages().expect("platform packages");
    packages.extend(ferrum_app_erp::packages().expect("ERP's packages"));
    packages.extend(ferrum_app_retail::packages().expect("app packages"));
    let catalog = Catalog::from_packages(packages).expect("catalog");
    let mut hooks = HookRegistry::default();
    ferrum_domain::register(&mut hooks);
    ferrum_app_erp::register(&mut hooks);
    ferrum_app_retail::register(&mut hooks);
    let settings = KernelSettings { secret_key: Some(SecretKey::new([7u8; 32])), ..Default::default() };
    let k = Kernel::boot(db, catalog, hooks, settings, None).await.expect("boot");
    ferrum_domain::maintenance(&k).await.expect("partitions");
    ferrum_app_erp::maintenance(&k).await.expect("partitions");
    let tenant = k
        .create_tenant(NewTenant {
            slug: "shops".into(),
            name: "Corner Shops".into(),
            admin_email: "admin@shops.test".into(),
            admin_password: "Correct-Horse-9".into(),
            admin_name: None,
            packages: vec!["retail".into()],
            settings: Some(json!({"fiscal_year_start_month": 4, "timezone": "Asia/Kolkata"})),
        })
        .await
        .expect("tenant");
    let session = k.login("shops", "admin@shops.test", "Correct-Horse-9").await.expect("login");
    let admin = k.load_ctx(tenant, session.user, Some(session.token_version), "it".into()).await.expect("ctx");
    let t = T { k: k.clone(), tenant, admin };
    let a = t.admin.clone();

    // ───────────── the books, the stores and the stock ─────────────
    let inr = t.find_one("Currency", json!({"code": "INR"})).await;
    let company = uid(&t.insert(&a, "Company", json!({"company_name": "Corner Shops", "abbr": "CS", "default_currency": inr["id"]})).await["id"]);
    let products = t.find_one("ItemGroup", json!({"group_name": "Products"})).await;
    let services = t.find_one("ItemGroup", json!({"group_name": "Services"})).await;
    let nos = t.find_one("Uom", json!({"uom_name": "Nos"})).await;
    let kettle = uid(&t
        .insert(&a, "Item", json!({"item_code": "KETTLE", "item_name": "Kettle", "item_group": products["id"], "stock_uom": nos["id"], "is_stock_item": true}))
        .await["id"]);
    let service = |code: &str| json!({"item_code": code, "item_name": code, "item_group": services["id"], "stock_uom": nos["id"], "is_stock_item": false});
    let royalty_item = uid(&t.insert(&a, "Item", service("ROYALTY")).await["id"]);
    let marketing_item = uid(&t.insert(&a, "Item", service("MARKETING")).await["id"]);
    let dc = uid(&t.find_one("Warehouse", json!({"warehouse_name": "Stores", "company": company.to_string()})).await["id"]);
    let downtown = uid(&t.insert(&a, "Warehouse", json!({"warehouse_name": "Downtown", "company": company.to_string()})).await["id"]);
    let uptown = uid(&t.insert(&a, "Warehouse", json!({"warehouse_name": "Uptown", "company": company.to_string()})).await["id"]);
    let receipt = t
        .insert(&a, "StockEntry", json!({"company": company.to_string(), "purpose": "Material Receipt", "items": [{"item": kettle.to_string(), "qty": "20", "basic_rate": "900", "target_warehouse": dc.to_string()}]}))
        .await;
    t.k.submit(&a, "StockEntry", uid(&receipt["id"]), None).await.expect("received at the centre");
    let manager = t.user("manager@shops.test", &["Retail Manager"]).await;
    let associate = t.user("till@shops.test", &["Store Associate"]).await;
    t.insert(
        &manager,
        "RetailSettings",
        json!({"company": company.to_string(), "royalty_item": royalty_item.to_string(), "marketing_item": marketing_item.to_string()}),
    )
    .await;

    // ───────────── click and collect ─────────────
    let ana = uid(&t.insert(&a, "Customer", json!({"customer_name": "Ana Rao", "customer_type": "Individual"})).await["id"]);
    let order = |store: Uuid, qty: &str| {
        json!({"company": company.to_string(), "customer": ana.to_string(), "set_warehouse": store.to_string(), "delivery_date": "2026-12-31",
               "items": [{"item": kettle.to_string(), "qty": qty, "price_list_rate": "1500", "discount_percent": "0"}]})
    };
    let so = uid(&t.insert(&a, "SalesOrder", order(downtown, "2")).await["id"]);
    let collect = |so: Uuid, store: Uuid| json!({"sales_order": so.to_string(), "store": store.to_string()});
    refused(t.k.insert(&associate, "CollectOrder", obj(collect(so, downtown))).await, "invalid", "is not submitted yet");
    t.k.submit(&a, "SalesOrder", so, None).await.expect("the order submitted");
    refused(t.k.insert(&associate, "CollectOrder", obj(collect(so, uptown))).await, "invalid", "is not where the order ships from");
    let cc = t.insert(&associate, "CollectOrder", collect(so, downtown)).await;
    let cc_id = uid(&cc["id"]);
    let code = cc["pickup_code"].as_str().unwrap().to_string();
    assert!(code.len() == 6 && code.chars().all(|c| c.is_ascii_digit()), "a six-digit code: {code}");
    refused(t.k.insert(&associate, "CollectOrder", obj(collect(so, downtown))).await, "invalid", "is collected under CC-");
    refused(t.act(&associate, "CollectOrder", cc_id, "make_ready", Value::Null).await, "invalid", "not in the store yet: Kettle (0 of 2)");
    let transfer = t
        .insert(
            &a,
            "StockEntry",
            json!({"company": company.to_string(), "purpose": "Material Transfer", "items": [{"item": kettle.to_string(), "qty": "1", "source_warehouse": dc.to_string(), "target_warehouse": downtown.to_string()}]}),
        )
        .await;
    t.k.submit(&a, "StockEntry", uid(&transfer["id"]), None).await.unwrap();
    refused(t.act(&associate, "CollectOrder", cc_id, "make_ready", Value::Null).await, "invalid", "Kettle (1 of 2)");
    let more = t
        .insert(
            &a,
            "StockEntry",
            json!({"company": company.to_string(), "purpose": "Material Transfer", "items": [{"item": kettle.to_string(), "qty": "4", "source_warehouse": dc.to_string(), "target_warehouse": downtown.to_string()}]}),
        )
        .await;
    t.k.submit(&a, "StockEntry", uid(&more["id"]), None).await.unwrap();
    t.act(&associate, "CollectOrder", cc_id, "make_ready", Value::Null).await.expect("ready");
    assert!(t.get("CollectOrder", cc_id).await["ready_at"].is_string());
    refused(t.act(&associate, "CollectOrder", cc_id, "hand_over", json!({"code": "000000"})).await, "invalid", "does not match");
    refused(t.act(&associate, "CollectOrder", cc_id, "hand_over", Value::Null).await, "invalid", "does not match");
    assert_eq!(t.stock(downtown, kettle).await, 5.0);
    let handed = t.act(&associate, "CollectOrder", cc_id, "hand_over", json!({"code": format!(" {code} ")})).await.expect("collected");
    let dn = t.get("DeliveryNote", uid(&handed["delivery_note"]["id"])).await;
    assert_eq!((dn["docstatus"].as_i64(), dn["customer"].as_str()), (Some(1), Some(ana.to_string().as_str())), "the order's delivery note, submitted");
    assert_eq!(t.stock(downtown, kettle).await, 3.0, "the kettles left the store");
    let done = t.get("CollectOrder", cc_id).await;
    assert_eq!((done["status"].as_str(), done["delivery_note"].as_str()), (Some("Collected"), dn["id"].as_str()));
    refused(t.k.insert(&associate, "CollectOrder", obj(collect(so, downtown))).await, "invalid", "is collected under");
    // A collection cancelled frees its order for another.
    let so2 = uid(&t.insert(&a, "SalesOrder", order(uptown, "1")).await["id"]);
    t.k.submit(&a, "SalesOrder", so2, None).await.unwrap();
    let first = uid(&t.insert(&associate, "CollectOrder", collect(so2, uptown)).await["id"]);
    t.act(&associate, "CollectOrder", first, "cancel_collect", Value::Null).await.unwrap();
    t.insert(&associate, "CollectOrder", collect(so2, uptown)).await;

    // ───────────── franchises ─────────────
    let franco = uid(&t.insert(&a, "Customer", json!({"customer_name": "Franco Stores", "customer_type": "Company"})).await["id"]);
    let franchise = |royalty: &str, marketing: &str, from: &str, to: &str| json!({"customer": franco.to_string(), "territory": "North", "royalty_percent": royalty, "marketing_percent": marketing, "starts_on": from, "ends_on": to});
    refused(t.k.insert(&manager, "Franchisee", obj(franchise("6", "2", "2026-01-01", "2025-12-31"))).await, "invalid", "after the agreement starts");
    refused(t.k.insert(&manager, "Franchisee", obj(franchise("120", "2", "2026-01-01", "2030-12-31"))).await, "invalid", "from 0 to 100");
    let fr = uid(&t.insert(&manager, "Franchisee", franchise("6", "2", "2026-01-01", "2030-12-31")).await["id"]);
    let report = |from: &str, to: &str, sales: &str| json!({"franchisee": fr.to_string(), "period_from": from, "period_to": to, "gross_sales": sales});
    let oct = t.insert(&manager, "RoyaltyReport", report("2026-10-01", "2026-10-31", "250000")).await;
    assert_eq!((dec(&oct["royalty"]), dec(&oct["marketing"]), dec(&oct["total"])), (15000.0, 5000.0, 20000.0));
    refused(t.k.insert(&manager, "RoyaltyReport", obj(report("2026-10-15", "2026-11-14", "1000"))).await, "invalid", "reports some of these days already");
    refused(t.k.insert(&manager, "RoyaltyReport", obj(report("2025-12-01", "2025-12-31", "1000"))).await, "invalid", "outside the franchise agreement");
    refused(t.k.insert(&manager, "RoyaltyReport", obj(report("2030-12-01", "2031-01-31", "1000"))).await, "invalid", "outside the franchise agreement");
    refused(t.k.insert(&manager, "RoyaltyReport", obj(report("2026-11-30", "2026-11-01", "1000"))).await, "invalid", "on or after the period starts");
    refused(t.k.insert(&manager, "RoyaltyReport", obj(report("2026-11-01", "2026-11-30", "-5"))).await, "invalid", "zero or more");
    t.insert(&manager, "RoyaltyReport", report("2026-11-01", "2026-11-30", "100000")).await;
    let oct_id = uid(&oct["id"]);
    let invoiced = t.act(&manager, "RoyaltyReport", oct_id, "invoice_royalty", Value::Null).await.expect("invoiced");
    let inv = t.get("SalesInvoice", uid(&invoiced["sales_invoice"]["id"])).await;
    let mut lines: Vec<(String, f64)> =
        inv["items"].as_array().unwrap().iter().map(|l| (l["item"].as_str().unwrap().to_string(), dec(&l["price_list_rate"]))).collect();
    lines.sort_by(|x, y| x.1.total_cmp(&y.1));
    assert_eq!(lines, vec![(marketing_item.to_string(), 5000.0), (royalty_item.to_string(), 15000.0)]);
    assert_eq!((inv["customer"].as_str(), dec(&inv["grand_total"])), (Some(franco.to_string().as_str()), 20000.0));
    refused(t.act(&manager, "RoyaltyReport", oct_id, "invoice_royalty", Value::Null).await, "invalid", "not available");
    refused(t.k.update(&manager, "RoyaltyReport", oct_id, obj(json!({"gross_sales": "1"})), None).await, "invalid", "an invoiced report is not changed");

    // ───────────── who may do what ─────────────
    refused(t.k.insert(&associate, "RoyaltyReport", obj(report("2026-12-01", "2026-12-31", "1"))).await, "forbidden", "");
    refused(t.k.insert(&associate, "Franchisee", obj(franchise("5", "1", "2026-01-01", "2027-01-01"))).await, "forbidden", "");
}
