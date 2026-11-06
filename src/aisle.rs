//! The endless aisle: what a store does not hold, it can still sell. Stock by Store says where an
//! item is (each store and warehouse holding it, the stores first, the most first), so the order a
//! store takes is shipped to the customer from one of them (ship from store), or brought to the
//! store to collect (Bring It From Another Store on its click and collect).

use async_trait::async_trait;
use ferrum_domain::reports::{Params, ReportSource, col, read_tx};
use ferrum_kernel::{Ctx, Error, Kernel, Result};
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use serde_json::{Value, json};
use std::collections::HashMap;

/// The most rows the report gives.
const MOST: i64 = 2000;

pub struct AisleReports;

#[async_trait]
impl ReportSource for AisleReports {
    fn catalog(&self) -> Vec<Value> {
        vec![json!({"name": "retail_stock_by_store", "label": "Stock by Store", "module": "retail", "entity": "Warehouse", "scoped": false,
                    "params": [{"name": "item", "label": "Item", "type": "link", "to": "Item"}]})]
    }

    async fn run(&self, k: &Kernel, ctx: &Ctx, name: &str, params: &HashMap<String, String>) -> Result<Value> {
        if name != "retail_stock_by_store" {
            return Err(Error::NotFound(format!("report {name}")));
        }
        let item = Params { raw: params }.uuid("item")?;
        let mut tx = read_tx(k, ctx).await?;
        let found: Vec<(String, Option<String>, String, bool, Decimal)> = sqlx::query_as(
            "select i.item_code, i.item_name, w.warehouse_name, coalesce(w.retail_store, false), sum(b.qty) \
             from stock_balance b join e_warehouse w on w.tenant_id = b.tenant_id and w.id = b.warehouse_id \
             join e_item i on i.tenant_id = b.tenant_id and i.id = b.item_id \
             where b.tenant_id = $1 and ($2::uuid is null or b.item_id = $2) and not coalesce(w.disabled, false) \
             group by i.item_code, i.item_name, w.warehouse_name, w.retail_store having sum(b.qty) > 0 \
             order by i.item_code, coalesce(w.retail_store, false) desc, sum(b.qty) desc, w.warehouse_name limit $3",
        )
        .bind(ctx.tenant)
        .bind(item)
        .bind(MOST)
        .fetch_all(&mut *tx)
        .await?;
        tx.rollback().await?;
        let rows: Vec<Value> = found
            .into_iter()
            .map(|(code, item_name, place, store, qty)| {
                json!({"item_code": code, "item_name": item_name, "warehouse": place, "kind": if store { "Store" } else { "Warehouse" }, "qty": qty.to_f64()})
            })
            .collect();
        Ok(json!({
            "columns": [col("item_code", "Item", "data"), col("item_name", "Item Name", "data"), col("warehouse", "Where", "data"),
                col("kind", "Kind", "data"), col("qty", "On Hand", "decimal")],
            "rows": rows,
        }))
    }
}
