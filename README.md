# Retail

Retail and distribution for [Ferrum](https://github.com/CBFerrum/Ferrum):
- click and collect: an order bought online, collected in its store with a pickup code;
- ship from store: an online order shipped from the store holding it;
- returns to any store, against the invoice that sold the goods;
- the endless aisle: where an item is, and what a store lacks brought from another;
- franchises: franchisees' sales reported, their royalty and marketing fee invoiced through ERP.

| | |
|---|---|
| 🏬 App | `retail`, native, in the server |
| 🧩 Package | `retail` (needs ERP's selling, inventory and accounting) |
| 🖥️ Interface | its own, at `/retail/`: collections to get ready and hand over, orders to ship from a store, returns to take back, transfers asked for and on the way, royalties to invoice, Stock by Store, and the records; and the portal's "Your franchise" at `/portal/franchise/` |
| 🔐 Roles | Retail Manager, Store Associate |
| 📜 Licence | AGPL-3.0-only |

**Stores are warehouses marked Is A Store**, a check Retail adds to ERP's warehouse.

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
- **Ship from store.** An online order ships to the customer from a store rather than the
  distribution centre: from a store named, or else from the one holding the most of it among the
  stores holding all of it. Ship It makes the order's delivery note from that store and submits
  it, with the carrier and tracking number. An order is shipped from a store once, and never also
  collected in one.
- **The endless aisle.** Stock by Store says where an item is, the stores first. A store sells
  what it lacks: shipped from another store, or brought to it for collection (Bring It From
  Another Store asks for what the collecting store lacks, as a transfer).
- **Returns to any store.** Goods come back to whichever store against the invoice that sold them,
  never more of an item than it sold less what came back already, whoever took it back (Commerce's
  returns and ERP's credit notes count too). Take It Back credits the invoice through ERP at what
  was paid, and puts the goods in that store's stock at the cost they left at: by the credit note
  when the invoice delivered them, or else by a return delivery note against each delivery note
  that did.
- **Between stores.** A store asks another for stock. The giving store sends only what it holds:
  the stock leaves it for the goods in transit, and the store that asked takes it in. Without a
  transit warehouse in the settings, what is sent arrives at once. Each move is an ERP stock
  entry, submitted. It may arrive in parts, item by item; what never arrives is written off out of
  the goods in transit, saying what happened, and the transfer is closed short. A transfer may be
  turned down, saying why, or cancelled. Once sent, it is kept as it was.
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
cargo test                       # needs FERRUM_RETAIL_TEST_DATABASE_URL's and FERRUM_RETAIL_OMNI_TEST_DATABASE_URL's databases
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

`tests/omnichannel_it.rs` walks stores selling as one:
- an order shipped from the store holding all of it, refused from a warehouse that is no store,
  twice, or when it is also collected, and its delivery note made from that store;
- Stock by Store, and a collection's missing goods brought from the centre;
- returns refused for a draft invoice, a warehouse that is no store, more than was sold and an item
  not sold; goods shipped from one store brought back to another by a return delivery note, and
  goods sold at a till brought back by the credit note, each at the cost they left at;
- a transfer taken in item by item and the rest written off; another taken in in two parts.

## Not yet

- Choosing the store nearest the customer: a store is chosen by what it holds, not where it is.
  Carriers' labels and tracking are kept as given, not booked with the carrier.
- Refunds at the store: a return credits the invoice, and the money goes back as ERP pays it (or
  as Commerce refunds a web order). Serial-numbered goods come back through ERP itself.
- Store-level pricing and promotions (ERP's pricing rules and POS profiles cover a store's prices
  today).
- Audits of what franchisees report.

## Changelog

| Version | Highlights |
|---|---|
| **0.3.0** | Ship from store, returns to any store, Stock by Store and goods a collection lacks brought from another store; transfers taken in parts, what never arrived written off. |
| **0.2.2** | Rebuilt with Ferrum-Frontend 24c3951: times on the workspace's clock; drafts headed as drafts. |
| **0.2.1** | Rebuilt with Ferrum-Frontend bb60115: empty numbers read as the server reads them; settings open as their record. |
| **0.2.0** | Transfers between stores through the goods in transit; franchisees reporting their sales on the portal. |
| **0.1.3** | Rebuilt with Ferrum-Frontend 00dea07: a workflow state's kept fields are read-only, its needed ones required. |
| **0.1.2** | Rebuilt with Ferrum-Frontend daeadda: actions say what they did. |
| **0.1.1** | Rebuilt with Ferrum-Frontend 80ff179: labels name their fields, and linked titles show their titles. |
| **0.1.0** | Click and collect with a pickup code from the store an order ships from, and franchise royalties invoiced through ERP (FERRUM-INDUSTRY-003). |

## License

AGPL-3.0-only. See [LICENSE](LICENSE).
