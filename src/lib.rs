//! Retail and distribution (FERRUM-INDUSTRY-003): click and collect with a pickup code, and
//! franchise royalties invoiced through ERP. Stores are replenished by ERP Procurement's
//! auto-reorder: a level for each store's warehouse, as transfers from the distribution centre.
//!
//! * `collect`: an online order collected in its store: ready once the store holds it, handed over
//!   against its code, its delivery note then made and submitted.
//! * `franchise`: a franchisee's reported sales for a period, its royalty and marketing fee
//!   invoiced once.
//! * `transfer`: one store asking another for stock, sent through the goods in transit and taken
//!   in, each move an ERP stock entry.
//! * `site`: the portal's "Your franchise", where a franchisee reports its sales and sees the fees
//!   invoiced on them.
//!
//! This app's own design; no code was copied (docs/PROVENANCE.md). Hooks are registered per entity,
//! so a workspace without the `retail` package never runs them.

mod collect;
mod franchise;
mod site;
mod transfer;

use ferrum_domain::reports::ReportSource;
use ferrum_kernel::{Doc, HookRegistry, Kernel, Result};
use serde_json::Value;
use std::sync::Arc;

include!(concat!(env!("OUT_DIR"), "/packages.rs"));
include!(concat!(env!("OUT_DIR"), "/ui.rs"));

/// This app's manifest (`app.yaml`).
pub const MANIFEST: &str = include_str!("../app.yaml");

/// This app's metadata packages, embedded at build time.
pub fn packages() -> std::result::Result<Vec<ferrum_meta::Package>, ferrum_meta::MetaError> {
    ferrum_meta::catalog::load_embedded(EMBEDDED)
}

/// Registers this app's document hooks, and the portal's "Your franchise".
pub fn register(hooks: &mut HookRegistry) {
    hooks
        .register("CollectOrder", Arc::new(collect::CollectHook))
        .register("Franchisee", Arc::new(franchise::FranchiseeHook))
        .register("RoyaltyReport", Arc::new(franchise::ReportHook))
        .register("StoreTransfer", Arc::new(transfer::TransferHook));
    hooks.add_site("retail", Arc::new(site::RetailSite));
}

/// This app's reports: none of its own yet.
pub fn reports() -> Vec<Arc<dyn ReportSource>> {
    vec![]
}

/// Upkeep: nothing runs by itself.
pub async fn maintenance(_k: &Kernel) -> Result<()> {
    Ok(())
}

/// A document from JSON.
pub(crate) fn obj(v: Value) -> Doc {
    v.as_object().cloned().unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_interface_is_a_release_the_platform_takes() {
        let m = ferrum_apps::AppManifest::parse(MANIFEST).expect("app.yaml");
        assert_eq!(m.routes(), vec![("retail".to_string(), vec![])]);
        let files = UI.iter().map(|(p, b)| (p.to_string(), b.to_vec())).collect();
        let ui = ferrum_apps::ui::prepare("retail", files).expect("ui/dist is an interface release");
        assert!(UI.iter().any(|(p, _)| *p == "index.html") && !UI.iter().any(|(p, _)| p.starts_with('.')), "{}", ui.digest);
    }
}
