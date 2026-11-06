# Retail

Retail and distribution for [Ferrum](https://github.com/CBFerrum/Ferrum):
- click and collect: an order bought online, collected in its store with a pickup code;
- franchises: franchisees' sales reported, their royalty and marketing fee invoiced through ERP.

| | |
|---|---|
| 🏬 App | `retail`, native, in the server |
| 🧩 Package | `retail` (needs ERP's selling, inventory and accounting) |
| 🖥️ Interface | its own, at `/retail/`: collections to get ready and hand over, transfers asked for and on the way, royalties to invoice, and the records; and the portal's "Your franchise" at `/portal/franchise/` |
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
- **Between stores.** A store asks another for stock. The giving store sends only what it holds:
  the stock leaves it for the goods in transit, and the store that asked takes it in. Without a
  transit warehouse in the settings, what is sent arrives at once. Each move is an ERP stock
  entry, submitted. A transfer may be turned down, saying why, or cancelled. Once sent, it is kept
  as it was.
- **Franchises.** A franchisee has a royalty and a marketing percentage, and an agreement's dates.
  Its sales are reported for a period within the agreement, never two reports for one day.
- **Royalties invoiced once.** The royalty and the marketing fee go through ERP to the
  franchisee's customer, each on its own line. An invoiced report is not changed.
- **Your franchise, on the portal.** A franchisee signs in to the workspace's portal and sees their
  agreements, the sales they reported and the fees on them. Each invoice opens on ERP's "Your
  invoices" once issued. They report a period's sales under their own agreements only, by the
  same rules as a report made in the office, and the report keeps who made it.

## Develop

```bash
cargo test                       # needs FERRUM_RETAIL_TEST_DATABASE_URL's database
cd ui && npm run build && npm run check
```

`tests/retail_it.rs` walks a chain of shops:
- an order collected in its store, once stock came from the centre;
- a collection cancelled and made again;
- a franchisee's reports, and their fees invoiced;
- who may do what;
- transfers refused, sent at once, sent through the goods in transit and taken in, turned down and
  cancelled, with the stock in each store;
- a franchisee reporting on the portal, another person refused, and an issued invoice opening.

## Not yet

- Ship from store, endless aisle and returns to any store.
- Store-level pricing and promotions (ERP's pricing rules and POS profiles cover a store's prices
  today).
- Transfers received short, or in parts; and audits of what franchisees report.

## Changelog

| Version | Highlights |
|---|---|
| **0.2.2** | Rebuilt with Ferrum-Frontend 24c3951: times on the workspace's clock; drafts headed as drafts. |
| **0.2.1** | Rebuilt with Ferrum-Frontend bb60115: empty numbers read as the server reads them; settings open as their record. |
| **0.2.0** | Transfers between stores through the goods in transit; franchisees reporting their sales on the portal. |
| **0.1.3** | Rebuilt with Ferrum-Frontend 00dea07: a workflow state's kept fields are read-only, its needed ones required. |
| **0.1.2** | Rebuilt with Ferrum-Frontend daeadda: actions say what they did. |
| **0.1.1** | Rebuilt with Ferrum-Frontend 80ff179: labels name their fields, and linked titles show their titles. |
| **0.1.0** | Click and collect with a pickup code from the store an order ships from, and franchise royalties invoiced through ERP (FERRUM-INDUSTRY-003). |

## License

AGPL-3.0-only. See [LICENSE](LICENSE).
