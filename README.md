# Retail

Retail and distribution for [Ferrum](https://github.com/CBFerrum/Ferrum):
- click and collect: an order bought online, collected in its store with a pickup code;
- franchises: franchisees' sales reported, their royalty and marketing fee invoiced through ERP.

| | |
|---|---|
| 🏬 App | `retail`, native, in the server |
| 🧩 Package | `retail` (needs ERP's selling, inventory and accounting) |
| 🖥️ Interface | its own, at `/retail/`: collections to get ready and hand over, royalties to invoice, and the records |
| 🔐 Roles | Retail Manager, Store Associate |
| 📜 Licence | AGPL-3.0-only |

**Replenishing stores is ERP's.** Give each item an auto-reorder level for each store's warehouse
in ERP Procurement, as a material transfer from the distribution centre. Procurement then raises
the transfers when a store's projected stock falls below its level. Retail adds nothing there.

## Highlights

- **Click and collect.** An order bought online is collected from the store it ships from, one
  collection to an order. It gets a six-digit pickup code.
- **Ready only when it is there.** A collection is ready only once the store holds every line of
  the order. Otherwise it says what is short, and how many the store has.
- **Handed over against the code.** The order's delivery note is then made and submitted, so the
  goods leave the store's stock. A cancelled collection frees its order for another.
- **Franchises.** A franchisee has a royalty and a marketing percentage, and an agreement's dates.
  Its sales are reported for a period within the agreement, never two reports for one day.
- **Royalties invoiced once.** The royalty and the marketing fee go through ERP to the
  franchisee's customer, each on its own line. An invoiced report is not changed.

## Develop

```bash
cargo test                       # needs FERRUM_RETAIL_TEST_DATABASE_URL's database
cd ui && npm run build && npm run check
```

`tests/retail_it.rs` walks a chain of shops:
- an order collected in its store, once stock came from the centre;
- a collection cancelled and made again;
- a franchisee's reports, and their fees invoiced;
- who may do what.

## Not yet

- Ship from store, endless aisle and returns to any store.
- Store transfers between stores, and store-level pricing and promotions (ERP's pricing rules and
  POS profiles cover a store's prices today).
- Franchisees reporting their own sales on the portal, and audits of what they report.

## Changelog

| Version | Highlights |
|---|---|
| **0.1.2** | Rebuilt with Ferrum-Frontend daeadda: actions say what they did. |
| **0.1.1** | Rebuilt with Ferrum-Frontend 80ff179: labels name their fields, and linked titles show their titles. |
| **0.1.0** | Click and collect with a pickup code from the store an order ships from, and franchise royalties invoiced through ERP (FERRUM-INDUSTRY-003). |

## License

AGPL-3.0-only. See [LICENSE](LICENSE).
