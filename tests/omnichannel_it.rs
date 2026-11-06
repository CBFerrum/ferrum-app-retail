//! Stores selling as one (FERRUM-INDUSTRY-003) against a real PostgreSQL
//! (`FERRUM_RETAIL_OMNI_TEST_DATABASE_URL`, default
//! `postgres://ferrum:ferrum@localhost:5432/ferrum_retail_omni_test`; reset first): an online order
//! shipped from the store holding the most of it, never from a warehouse that is no store, never
//! twice and never also collected, its delivery note made from that store; Stock by Store saying
//! where an item is, and what a store lacks for a collection brought from another; goods brought
//! back to any store against their invoice, never more than it sold less what came back, credited,
//! and back in that store's stock at the cost they left at, whether a delivery note or the invoice
//! sent them; and a transfer taken in item by item, what never arrived written off.

use ferrum_kernel::secret::SecretKey;
use ferrum_kernel::tenant::NewTenant;
use ferrum_kernel::{Ctx, Db, DbConfig, Doc, Error, HookRegistry, Kernel, KernelSettings, ListQuery};
use ferrum_meta::Catalog;
use serde_json::{Value, json};
use std::collections::HashMap;
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
    async fn find(&self, entity: &str, filters: Value) -> Vec<Doc> {
        let q = ListQuery { filters: ferrum_kernel::query::parse_filters(&filters).unwrap(), limit: Some(50), ..Default::default() };
        self.k.list(&self.admin, entity, &q).await.unwrap().rows
    }
    async fn find_one(&self, entity: &str, filters: Value) -> Doc {
        let rows = self.find(entity, filters.clone()).await;
        assert_eq!(rows.len(), 1, "{entity} {filters}");
        rows[0].clone()
    }
    async fn user(&self, email: &str, roles: &[&str]) -> Ctx {
        let roles: Vec<String> = roles.iter().map(|r| r.to_string()).collect();
        let id = self.k.create_user(&self.admin, email, email, Some("Correct-Horse-9"), &roles, false).await.expect("user");
        let s = self.k.login("chain", email, "Correct-Horse-9").await.expect("login");
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
    /// What a document's stock moves into `warehouse` were worth.
    async fn value_in(&self, entity: &str, doc: Uuid, warehouse: Uuid) -> f64 {
        let mut tx = self.k.db.tenant_tx(self.tenant).await.unwrap();
        let v: rust_decimal::Decimal = sqlx::query_scalar(
            "select coalesce(sum(value), 0) from stock_move where tenant_id = $1 and source_entity = $2 and source_id = $3 and warehouse_id = $4 and qty > 0",
        )
        .bind(self.tenant)
        .bind(entity)
        .bind(doc)
        .bind(warehouse)
        .fetch_one(&mut *tx)
        .await
        .unwrap();
        tx.commit().await.unwrap();
        v.to_string().parse().unwrap()
    }
    /// Moves stock between two warehouses by a submitted stock entry.
    async fn transfer(&self, company: Uuid, item: Uuid, qty: &str, from: Uuid, to: Uuid) {
        let e = self
            .insert(
                &self.admin,
                "StockEntry",
                json!({"company": company.to_string(), "purpose": "Material Transfer",
                       "items": [{"item": item.to_string(), "qty": qty, "source_warehouse": from.to_string(), "target_warehouse": to.to_string()}]}),
            )
            .await;
        self.k.submit(&self.admin, "StockEntry", uid(&e["id"]), None).await.expect("moved");
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
fn stores_selling_as_one() {
    let rt = tokio::runtime::Builder::new_multi_thread().enable_all().thread_stack_size(16 << 20).build().unwrap();
    rt.block_on(async { tokio::spawn(scenario()).await.unwrap() });
}

async fn scenario() {
    let url =
        std::env::var("FERRUM_RETAIL_OMNI_TEST_DATABASE_URL").unwrap_or_else(|_| "postgres://ferrum:ferrum@localhost:5432/ferrum_retail_omni_test".into());
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
    for r in ferrum_app_retail::reports() {
        ferrum_domain::reports::add_source(r);
    }
    let settings = KernelSettings { secret_key: Some(SecretKey::new([7u8; 32])), ..Default::default() };
    let k = Kernel::boot(db, catalog, hooks, settings, None).await.expect("boot");
    ferrum_domain::maintenance(&k).await.expect("partitions");
    ferrum_app_erp::maintenance(&k).await.expect("partitions");
    let tenant = k
        .create_tenant(NewTenant {
            slug: "chain".into(),
            name: "Chain Stores".into(),
            admin_email: "admin@chain.test".into(),
            admin_password: "Correct-Horse-9".into(),
            admin_name: None,
            packages: vec!["retail".into()],
            settings: Some(json!({"fiscal_year_start_month": 4, "timezone": "Asia/Kolkata"})),
        })
        .await
        .expect("tenant");
    let session = k.login("chain", "admin@chain.test", "Correct-Horse-9").await.expect("login");
    let admin = k.load_ctx(tenant, session.user, Some(session.token_version), "it".into()).await.expect("ctx");
    let t = T { k: k.clone(), tenant, admin };
    let a = t.admin.clone();

    // ───────────── the books, the stores and the stock ─────────────
    let inr = t.find_one("Currency", json!({"code": "INR"})).await;
    let company = uid(&t.insert(&a, "Company", json!({"company_name": "Chain Stores", "abbr": "CH", "default_currency": inr["id"]})).await["id"]);
    let products = t.find_one("ItemGroup", json!({"group_name": "Products"})).await;
    let nos = t.find_one("Uom", json!({"uom_name": "Nos"})).await;
    let item =
        |code: &str, name: &str| json!({"item_code": code, "item_name": name, "item_group": products["id"], "stock_uom": nos["id"], "is_stock_item": true});
    let kettle = uid(&t.insert(&a, "Item", item("KETTLE", "Kettle")).await["id"]);
    let toaster = uid(&t.insert(&a, "Item", item("TOASTER", "Toaster")).await["id"]);
    let dc = uid(&t.find_one("Warehouse", json!({"warehouse_name": "Stores", "company": company.to_string()})).await["id"]);
    let store = |name: &str| json!({"warehouse_name": name, "company": company.to_string(), "retail_store": true});
    let downtown = uid(&t.insert(&a, "Warehouse", store("Downtown")).await["id"]);
    let uptown = uid(&t.insert(&a, "Warehouse", store("Uptown")).await["id"]);
    let transit = uid(&t.insert(&a, "Warehouse", json!({"warehouse_name": "In Transit", "company": company.to_string()})).await["id"]);
    let receipt = t
        .insert(
            &a,
            "StockEntry",
            json!({"company": company.to_string(), "purpose": "Material Receipt", "items": [
                {"item": kettle.to_string(), "qty": "20", "basic_rate": "900", "target_warehouse": dc.to_string()},
                {"item": toaster.to_string(), "qty": "10", "basic_rate": "400", "target_warehouse": dc.to_string()}]}),
        )
        .await;
    t.k.submit(&a, "StockEntry", uid(&receipt["id"]), None).await.expect("received at the centre");
    t.transfer(company, kettle, "3", dc, uptown).await;
    t.transfer(company, kettle, "1", dc, downtown).await;
    let manager = t.user("manager@chain.test", &["Retail Manager"]).await;
    let associate = t.user("till@chain.test", &["Store Associate"]).await;
    t.insert(&manager, "RetailSettings", json!({"company": company.to_string(), "transit_warehouse": transit.to_string()})).await;
    let ana = uid(&t.insert(&a, "Customer", json!({"customer_name": "Ana Rao", "customer_type": "Individual"})).await["id"]);
    let order = |ships_from: Uuid, lines: Value| json!({"company": company.to_string(), "customer": ana.to_string(), "set_warehouse": ships_from.to_string(), "delivery_date": "2026-12-31", "items": lines});
    let line = |item: Uuid, qty: &str, price: &str| json!({"item": item.to_string(), "qty": qty, "price_list_rate": price, "discount_percent": "0"});

    // ───────────── ship from store ─────────────
    let big = uid(&t.insert(&a, "SalesOrder", order(dc, json!([line(kettle, "10", "1500")]))).await["id"]);
    t.k.submit(&a, "SalesOrder", big, None).await.unwrap();
    refused(t.k.insert(&associate, "StoreShipment", obj(json!({"sales_order": big.to_string()}))).await, "invalid", "no store holds all of it");
    let so1 = uid(&t.insert(&a, "SalesOrder", order(dc, json!([line(kettle, "2", "1500")]))).await["id"]);
    t.k.submit(&a, "SalesOrder", so1, None).await.unwrap();
    refused(
        t.k.insert(&associate, "StoreShipment", obj(json!({"sales_order": so1.to_string(), "store": dc.to_string()}))).await,
        "invalid",
        "is not a store of the order's company",
    );
    // Uptown holds all of it; Downtown holds one.
    let shipment = t.insert(&associate, "StoreShipment", json!({"sales_order": so1.to_string()})).await;
    assert_eq!((shipment["store"].as_str(), shipment["status"].as_str()), (Some(uptown.to_string().as_str()), Some("To Ship")), "the store holding all of it");
    let shipment = uid(&shipment["id"]);
    refused(t.k.insert(&associate, "StoreShipment", obj(json!({"sales_order": so1.to_string()}))).await, "invalid", "is shipped under SFS-");
    refused(
        t.k.insert(&associate, "CollectOrder", obj(json!({"sales_order": so1.to_string(), "store": dc.to_string()}))).await,
        "invalid",
        "is shipped from a store under SFS-",
    );
    let shipped = t.act(&associate, "StoreShipment", shipment, "ship_it", json!({"carrier": "Blue Dart", "tracking_no": "BD123"})).await.expect("shipped");
    let dn1 = uid(&shipped["delivery_note"]["id"]);
    let note = t.get("DeliveryNote", dn1).await;
    let wh: Vec<&str> = note["items"].as_array().unwrap().iter().map(|l| l["warehouse"].as_str().unwrap()).collect();
    assert_eq!((note["docstatus"].as_i64(), wh, note["transporter"].as_str()), (Some(1), vec![uptown.to_string().as_str()], Some("Blue Dart")));
    assert_eq!((t.stock(uptown, kettle).await, t.stock(dc, kettle).await), (1.0, 16.0), "the kettles left the store, not the centre");
    let s = t.get("StoreShipment", shipment).await;
    assert_eq!(
        (s["status"].as_str(), s["tracking_no"].as_str(), s["delivery_note"].as_str()),
        (Some("Shipped"), Some("BD123"), Some(dn1.to_string().as_str()))
    );
    refused(t.k.insert(&associate, "StoreShipment", obj(json!({"sales_order": so1.to_string()}))).await, "invalid", "has nothing left to ship");
    assert_eq!(dec(&t.get("SalesOrder", so1).await["per_delivered"]), 100.0);

    // ───────────── the endless aisle ─────────────
    let stock = ferrum_domain::reports::run(&k, &associate, "retail_stock_by_store", &HashMap::from([("item".to_string(), kettle.to_string())]))
        .await
        .expect("stock by store");
    let places: Vec<(String, String, f64)> = stock["rows"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| (r["warehouse"].as_str().unwrap().to_string(), r["kind"].as_str().unwrap().to_string(), r["qty"].as_f64().unwrap()))
        .collect();
    let place = |w: &str, kind: &str, q: f64| (w.to_string(), kind.to_string(), q);
    assert_eq!(places, vec![place("Downtown", "Store", 1.0), place("Uptown", "Store", 1.0), place("Stores", "Warehouse", 16.0)], "the stores first");
    // A collection in a store that lacks it, brought from the centre.
    let so2 = uid(&t.insert(&a, "SalesOrder", order(downtown, json!([line(toaster, "2", "800")]))).await["id"]);
    t.k.submit(&a, "SalesOrder", so2, None).await.unwrap();
    let cc = uid(&t.insert(&associate, "CollectOrder", json!({"sales_order": so2.to_string(), "store": downtown.to_string()})).await["id"]);
    refused(t.act(&associate, "CollectOrder", cc, "make_ready", Value::Null).await, "invalid", "not in the store yet: Toaster (0 of 2)");
    refused(t.act(&associate, "CollectOrder", cc, "bring_from", json!({"from_store": downtown.to_string()})).await, "invalid", "another store");
    let brought = t.act(&associate, "CollectOrder", cc, "bring_from", json!({"from_store": dc.to_string()})).await.expect("asked for");
    let st = t.get("StoreTransfer", uid(&brought["transfer"]["id"])).await;
    assert_eq!(
        (st["from_store"].as_str(), st["to_store"].as_str(), st["requested_by"].as_str(), dec(&st["items"][0]["qty"])),
        (Some(dc.to_string().as_str()), Some(downtown.to_string().as_str()), Some("till@chain.test"), 2.0)
    );
    assert_eq!(t.get("CollectOrder", cc).await["transfer"], st["id"]);
    refused(t.act(&associate, "CollectOrder", cc, "bring_from", json!({"from_store": dc.to_string()})).await, "invalid", "is already bringing it");
    let st = uid(&st["id"]);
    t.act(&associate, "StoreTransfer", st, "send_transfer", Value::Null).await.expect("sent");
    t.act(&associate, "StoreTransfer", st, "receive_transfer", Value::Null).await.expect("taken in");
    t.act(&associate, "CollectOrder", cc, "make_ready", Value::Null).await.expect("ready once brought");

    // ───────────── returns to any store ─────────────
    let made = t.act(&a, "DeliveryNote", dn1, "make_sales_invoice", Value::Null).await.expect("invoiced");
    let invoice = uid(&made["id"]);
    let back = |inv: Uuid, at: Uuid, lines: Value| json!({"sales_invoice": inv.to_string(), "store": at.to_string(), "items": lines});
    let kettles = |qty: &str| json!([{"item": kettle.to_string(), "qty": qty, "reason": "Damaged"}]);
    refused(t.k.insert(&associate, "StoreReturn", obj(back(invoice, downtown, kettles("1")))).await, "invalid", "is not issued");
    t.k.submit(&a, "SalesInvoice", invoice, None).await.expect("issued");
    refused(t.k.insert(&associate, "StoreReturn", obj(back(invoice, dc, kettles("1")))).await, "invalid", "is not a store of the invoice's company");
    refused(t.k.insert(&associate, "StoreReturn", obj(back(invoice, downtown, kettles("3")))).await, "invalid", "Kettle: 2 can still come back");
    refused(
        t.k.insert(&associate, "StoreReturn", obj(back(invoice, downtown, json!([{"item": toaster.to_string(), "qty": "1"}])))).await,
        "invalid",
        "Toaster is not on the invoice",
    );
    // Shipped from Uptown, brought back to Downtown.
    let r1 = t.insert(&associate, "StoreReturn", back(invoice, downtown, kettles("1"))).await;
    assert_eq!((r1["customer"].as_str(), r1["status"].as_str()), (Some(ana.to_string().as_str()), Some("Draft")));
    let r1 = uid(&r1["id"]);
    let taken = t.act(&associate, "StoreReturn", r1, "take_back", Value::Null).await.expect("taken back");
    let credit = t.get("SalesInvoice", uid(&taken["credit_note"]["id"])).await;
    assert_eq!(
        (credit["docstatus"].as_i64(), credit["is_return"].as_bool(), credit["return_against"].as_str(), credit["update_stock"].as_bool()),
        (Some(1), Some(true), Some(invoice.to_string().as_str()), Some(false)),
        "the invoice credited; the goods come back by a delivery note"
    );
    let credited: Vec<(String, f64, f64)> =
        credit["items"].as_array().unwrap().iter().map(|l| (l["item"].as_str().unwrap().to_string(), dec(&l["qty"]), dec(&l["price_list_rate"]))).collect();
    assert_eq!(credited, vec![(kettle.to_string(), -1.0, 1500.0)], "at what was paid");
    let returned = t.find_one("DeliveryNote", json!({"return_against": dn1.to_string()})).await;
    let returned = uid(&returned["id"]);
    assert_eq!(t.get("DeliveryNote", returned).await["docstatus"].as_i64(), Some(1));
    assert_eq!((t.stock(downtown, kettle).await, t.stock(uptown, kettle).await), (2.0, 1.0), "back in the store it was brought to");
    assert_eq!(t.value_in("DeliveryNote", returned, downtown).await, 900.0, "at the cost it left at");
    let r = t.get("StoreReturn", r1).await;
    assert!(r["status"] == "Taken Back" && r["goods_back_by"].as_str().is_some_and(|n| n.starts_with("DN-")) && r["taken_by"] == "till@chain.test", "{r:?}");
    refused(t.k.insert(&associate, "StoreReturn", obj(back(invoice, uptown, kettles("2")))).await, "invalid", "Kettle: 1 can still come back");
    refused(t.k.update(&associate, "StoreReturn", r1, obj(json!({"store": uptown.to_string()})), None).await, "invalid", "kept as it was");
    // Sold at a store's till, the invoice delivering it; brought back to another.
    let till = t
        .insert(
            &a,
            "SalesInvoice",
            json!({"company": company.to_string(), "customer": ana.to_string(), "update_stock": true, "set_warehouse": uptown.to_string(),
                   "items": [{"item": kettle.to_string(), "qty": "1", "price_list_rate": "1500", "discount_percent": "0", "warehouse": uptown.to_string()}]}),
        )
        .await;
    let till = uid(&till["id"]);
    t.k.submit(&a, "SalesInvoice", till, None).await.expect("sold at the till");
    assert_eq!(t.stock(uptown, kettle).await, 0.0);
    let r2 = uid(&t.insert(&associate, "StoreReturn", back(till, downtown, kettles("1"))).await["id"]);
    let taken = t.act(&associate, "StoreReturn", r2, "take_back", Value::Null).await.expect("taken back");
    let credit = uid(&taken["credit_note"]["id"]);
    let c = t.get("SalesInvoice", credit).await;
    assert_eq!((c["update_stock"].as_bool(), c["set_warehouse"].as_str()), (Some(true), Some(downtown.to_string().as_str())), "the credit note takes it in");
    assert_eq!(t.stock(downtown, kettle).await, 3.0);
    assert_eq!(t.value_in("SalesInvoice", credit, downtown).await, 900.0, "at the cost it left at");
    assert!(t.get("StoreReturn", r2).await["goods_back_by"].is_null());

    // ───────────── transfers taken in item by item ─────────────
    let ask = |from: Uuid, to: Uuid, qty: &str| json!({"from_store": from.to_string(), "to_store": to.to_string(), "items": [{"item": kettle.to_string(), "qty": qty}]});
    let van = uid(&t.insert(&associate, "StoreTransfer", ask(dc, downtown, "3")).await["id"]);
    t.act(&associate, "StoreTransfer", van, "send_transfer", Value::Null).await.expect("sent");
    let part = |item: Uuid, qty: &str| json!({"item": item.to_string(), "qty": qty});
    t.act(&associate, "StoreTransfer", van, "receive_part", part(kettle, "1")).await.expect("one arrived");
    let v = t.get("StoreTransfer", van).await;
    assert_eq!((v["status"].as_str(), dec(&v["items"][0]["received_qty"])), (Some("Part Received"), 1.0));
    assert_eq!((t.stock(downtown, kettle).await, t.stock(transit, kettle).await), (4.0, 2.0));
    refused(t.act(&associate, "StoreTransfer", van, "receive_part", part(kettle, "5")).await, "invalid", "only 2 of it is still on the way");
    refused(t.act(&associate, "StoreTransfer", van, "receive_part", part(toaster, "1")).await, "invalid", "is not on this transfer");
    refused(t.act(&associate, "StoreTransfer", van, "write_off_rest", json!({"why": " "})).await, "invalid", "says what happened");
    t.act(&associate, "StoreTransfer", van, "write_off_rest", json!({"why": "The van was broken into"})).await.expect("written off");
    let v = t.get("StoreTransfer", van).await;
    assert_eq!((v["status"].as_str(), dec(&v["items"][0]["lost_qty"])), (Some("Closed Short"), 2.0));
    assert!(v["note"].as_str().is_some_and(|n| n.contains("Lost on the way (2 Kettle): The van was broken into")), "{v:?}");
    let issue = t.get("StockEntry", uid(&v["lost_entry"])).await;
    assert_eq!((issue["purpose"].as_str(), issue["docstatus"].as_i64()), (Some("Material Issue"), Some(1)));
    assert_eq!((t.stock(transit, kettle).await, t.stock(downtown, kettle).await), (0.0, 4.0), "out of the goods in transit, never into the store");
    let rest = uid(&t.insert(&associate, "StoreTransfer", ask(dc, uptown, "2")).await["id"]);
    t.act(&associate, "StoreTransfer", rest, "send_transfer", Value::Null).await.expect("sent");
    t.act(&associate, "StoreTransfer", rest, "receive_part", part(kettle, "1")).await.expect("one arrived");
    t.act(&associate, "StoreTransfer", rest, "receive_transfer", Value::Null).await.expect("the rest arrived");
    let r = t.get("StoreTransfer", rest).await;
    assert_eq!((r["status"].as_str(), dec(&r["items"][0]["received_qty"])), (Some("Received"), 2.0));
    assert_eq!((t.stock(uptown, kettle).await, t.stock(transit, kettle).await), (2.0, 0.0));
    refused(t.act(&associate, "StoreTransfer", rest, "receive_transfer", Value::Null).await, "invalid", "not available");
}
