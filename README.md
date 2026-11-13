<div align="center">

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/assets/banner-dark.svg">
  <img alt="Ferrum Retail: stores that sell and serve as one" src="docs/assets/banner-light.svg" width="100%">
</picture>

<br>

[![Version](https://img.shields.io/badge/version-0.3.13-be185d?style=flat-square)](CHANGELOG.md)
[![Licence: AGPL-3.0](https://img.shields.io/badge/licence-AGPL--3.0-blue?style=flat-square)](LICENSE)
[![Ferrum platform](https://img.shields.io/badge/runs%20on-Ferrum-2f5bea?style=flat-square)](https://github.com/CBFerrum/Ferrum)
![Rust](https://img.shields.io/badge/Rust-server-b7410e?style=flat-square&logo=rust)
![PostgreSQL](https://img.shields.io/badge/PostgreSQL-data-336791?style=flat-square&logo=postgresql&logoColor=white)
![React](https://img.shields.io/badge/React-TypeScript-087ea4?style=flat-square&logo=react&logoColor=white)
![Tests](https://img.shields.io/badge/tests-4%20Rust%20%C2%B7%203%20UI-16a34a?style=flat-square)

### Stores that sell and serve as one: collect, ship, return and transfer across every shop.

Ferrum Retail is for chains of shops and the franchisors behind them. It adds click and collect, shipping from a store, returns at any store, stock by store, transfers between stores and franchise royalties on top of the stock, orders and invoices that ERP already keeps. It is the omnichannel layer that Shopify, Lightspeed, Dynamics 365 Commerce or Odoo give a retailer, on a self-hosted system. The till itself is the [Point of Sale](https://github.com/CBFerrum/ferrum-app-pos) app.

[💡 Why](#-why-retail) · [📸 Screenshots](#-screenshots) · [✨ Features](#-features) · [📊 Compare](#-how-it-compares) · [📍 Roadmap](#-roadmap) · [🚀 Install](#-install) · [🔧 Develop](#-develop)

</div>

## 💡 Why Retail

- 🛍️ **Click and collect with a pickup code.** An order bought online is collected from the store it ships from. It is ready only when the store holds every line, and it is handed over only against the code.
- 📦 **Ship from the store that has it.** An online order can ship from a store instead of the distribution centre, and the store's stock goes down with the delivery note.
- ↩️ **Returns at any store.** Goods come back to whichever store the customer visits, against the invoice that sold them, and never more than was sold.
- 🚚 **Stock moves between stores with its paperwork.** A transfer goes through the goods in transit, arrives in parts, and what never arrives is written off with a reason.
- 🤝 **Franchises on the same books.** Franchisees report their sales on the workspace's portal, and the royalty and marketing fee are invoiced through ERP.
- 🏠 **Self-hosted and open source.** AGPL-3.0, your server and your data, with no per-store fee.
- 🧩 **Extendable in Studio, open to AI agents.** Add fields and rules to any record, and use the same records over REST and MCP under the same permissions.

## 🎯 At a glance

| | |
|---|---|
| **Version** | 0.3.13 |
| **Kind** | Its own interface at `/retail/` (what waits on the stores), the franchisee's page of the portal at `/portal/franchise/`, and the platform's records screens for the rest |
| **What it adds** | 9 entities (7 you work with, 2 that sit inside them), 2 roles, 1 report (Stock by Store), 1 menu module, 1 field on ERP's warehouse (Is A Store), 1 package. It has no print formats of its own |
| **Depends on** | Ferrum platform `>=0.3.2, <0.4` and [ERP](https://github.com/CBFerrum/ferrum-app-erp) `>=0.1` (selling, inventory and accounting) |
| **Works with** | [Point of Sale](https://github.com/CBFerrum/ferrum-app-pos) (goods sold at a till can be brought back to any store), Commerce (its returns count too), ERP Procurement (replenishing stores) |
| **Languages** | English, Arabic (right to left), Hindi |
| **Tests** | 4 Rust tests (two end-to-end walks of a chain of shops), 3 interface tests |
| **Licence** | AGPL-3.0-only |

## 📸 Screenshots

<p align="center"><picture><source media="(prefers-color-scheme: dark)" srcset="docs/assets/screens/retail-home-dark.webp"><img alt="Retail — home" src="docs/assets/screens/retail-home-light.webp"></picture><br><b>🏬 The stores</b><br><sub>See what waits on the stores: collections to get ready, orders to ship, returns to take back, transfers and royalties to invoice.</sub></p>

<table>
<tr>
<td width="50%" valign="top">
<picture><source media="(prefers-color-scheme: dark)" srcset="docs/assets/screens/retail-franchisee-list-dark.webp"><img alt="Franchisees — list" src="docs/assets/screens/retail-franchisee-list-light.webp"></picture>
<p><b>🤝 Franchisees</b><br><sub>Each franchisee with its customer, territory, royalty and marketing percentages.</sub></p>
</td>
<td width="50%" valign="top">
<picture><source media="(prefers-color-scheme: dark)" srcset="docs/assets/screens/retail-collect-order-list-dark.webp"><img alt="Click and collect — list" src="docs/assets/screens/retail-collect-order-list-light.webp"></picture>
<p><b>🛍️ Click and collect</b><br><sub>Orders customers collect in a store, from getting ready to handing over, as a list, a board or a calendar.</sub></p>
</td>
</tr>
<tr>
<td width="50%" valign="top">
<picture><source media="(prefers-color-scheme: dark)" srcset="docs/assets/screens/retail-store-return-list-dark.webp"><img alt="Store returns — list" src="docs/assets/screens/retail-store-return-list-light.webp"></picture>
<p><b>↩️ Store returns</b><br><sub>Goods taken back at any store against the invoice that sold them.</sub></p>
</td>
<td width="50%" valign="top">
<picture><source media="(prefers-color-scheme: dark)" srcset="docs/assets/screens/retail-franchisee-detail-dark.webp"><img alt="Franchisee — open record" src="docs/assets/screens/retail-franchisee-detail-light.webp"></picture>
<p><b>📄 A franchisee</b><br><sub>Open one franchisee: customer, territory, royalty and marketing percentages, and the dates of the agreement.</sub></p>
</td>
</tr>
<tr>
<td width="50%" valign="top">
<picture><source media="(prefers-color-scheme: dark)" srcset="docs/assets/screens/retail-report-stock-by-store-dark.webp"><img alt="Retail — Stock by store" src="docs/assets/screens/retail-report-stock-by-store-light.webp"></picture>
<p><b>📦 Stock by store</b><br><sub>Where an item is: each store and warehouse holding it, the stores first.</sub></p>
</td>
<td width="50%" valign="top">
<picture><source media="(prefers-color-scheme: dark)" srcset="docs/assets/screens/retail-transfer-list-dark.webp"><img alt="Retail — store transfers" src="docs/assets/screens/retail-transfer-list-light.webp"></picture>
<p><b>🚚 Store transfers</b><br><sub>Stock asked for, on its way and arrived between stores.</sub></p>
</td>
</tr>
</table>

<p align="center">
<span><picture><source media="(prefers-color-scheme: dark)" srcset="docs/assets/screens/retail-home-phone-dark.webp"><img alt="Retail — home" src="docs/assets/screens/retail-home-phone-light.webp" width="260"></picture><br><sub><b>🏬 The stores on a phone</b></sub></span>&nbsp;
<span><picture><source media="(prefers-color-scheme: dark)" srcset="docs/assets/screens/retail-franchisee-list-phone-dark.webp"><img alt="Franchisees — list" src="docs/assets/screens/retail-franchisee-list-phone-light.webp" width="260"></picture><br><sub><b>🤝 Franchisees on a phone</b></sub></span>&nbsp;
</p>

## ✨ Features

### 🏬 Stores

- **A store is a warehouse marked Is A Store.** Retail adds that check to ERP's warehouse. A store ships online orders, takes returns, and is counted in Stock by Store.
- **Retail Settings.** The company franchise fees are invoiced from, the item the royalty is invoiced as, the item the marketing fee is invoiced as, and a **Goods in Transit** warehouse for stock sent between stores.
- **Quick create.** New Click and Collect, Store Transfer, Ship From Store and Store Return are one click from the create menu.
- **What waits on the stores.** The home page lists collections to get ready and to hand over, orders to ship from a store, returns to take back, transfers asked for and on the way, and royalty reports to invoice. Each opens its record.

### 🛍️ Click and collect

| State | Meaning |
|---|---|
| **Ordered** | An order to collect in a store, with a six-digit pickup code |
| **Ready** | The store holds every line |
| **Collected** | Handed over against the code |
| **Cancelled** | The order is free for another collection |

- **One collection to an order.** An order is collected in one store once, and an order shipped from a store is never also collected.
- **It Is Ready** only when the store holds every line. Otherwise it says what is short, and how many the store has.
- **Bring It From Another Store** asks for what the collecting store lacks, as a transfer, and links it to the collection.
- **Hand It Over** asks for the pickup code. It makes and submits the order's delivery note, so the goods leave the store's stock.
- **Cancel It** frees the order for another collection.

### 📦 Ship from store

- **Choose the store, or let the app choose.** From a store named, or else from the store holding the most of it among the stores holding all of it.
- **Ship It** makes the order's delivery note from that store, for every line, whatever warehouse the order named (a web order names none), and submits it. The carrier and tracking number are kept as given.
- **Once only.** An order is shipped from a store once, and never also collected in one. A warehouse that is not a store is refused.
- **States.** To Ship, Shipped, Cancelled.

### ↩️ Returns to any store

- **Against the invoice that sold the goods.** Goods bought online or in another store come back to whichever store the customer brings them to.
- **Never more than was sold.** A return is limited to what the invoice sold, less what came back already, whoever took it back (Commerce's returns and ERP's credit notes count too). A draft invoice, a warehouse that is not a store and an item that was not sold are refused.
- **Take It Back** credits the invoice through ERP at what was paid and puts the goods into that store's stock at the cost they left at. It does so by the credit note when the invoice delivered them, or else by a return delivery note against each delivery note that did. It works for goods sold at a till as well as goods shipped from another store.
- **A reason per line:** Changed Mind, Damaged, Faulty, Wrong Item, Other.
- **States.** Draft, Taken Back, Cancelled. The person who took it back is kept.
- **The money goes back as ERP pays it** (or as Commerce refunds a web order). A cash refund at the store's till is not built yet.

### 🚚 Transfers between stores

| State | Meaning |
|---|---|
| **Requested** | A store asks another for stock |
| **Sent** | The giving store sent what it holds. Stock leaves it for the goods in transit |
| **Part Received** | Some items arrived |
| **Received** | Everything arrived |
| **Closed Short** | What never arrived was written off, saying what happened |
| **Rejected** | The giving store turned it down, saying why |
| **Cancelled** | Cancelled before it was sent |

- **The giving store sends only what it holds.**
- **Goods in transit.** With a transit warehouse in the settings, the stock sits there until the asking store takes it in. Without one, what is sent arrives at once.
- **Each move is an ERP stock entry,** submitted: the one that sent it, the ones that took it in, and the one that wrote the rest off.
- **It Arrived** takes it all in. **Some Of It Arrived** takes an item in, with a quantity, and the transfer is Part Received.
- **The Rest Is Lost** writes off what never arrived out of the goods in transit, with the reason.
- **Once sent, it is kept as it was.**

### 🌐 The endless aisle

- **Stock by Store** is a report that says where an item is: each store and warehouse holding it, the stores first and the most first. It can be run for one item or all.
- **A store sells what it lacks,** shipped to the customer from another store, or brought to it for collection.
- **Replenishing stores is ERP's.** Give each item an auto-reorder level for each store's warehouse in ERP Procurement, as a material transfer from the distribution centre, and Procurement raises the transfers when a store's projected stock falls short. Retail adds nothing there.

### 🤝 Franchises

- **Franchisee.** A customer with a territory, a royalty percentage, a marketing percentage and the dates of the agreement.
- **Royalty report.** A franchisee's gross sales for a period, within the agreement, and never two reports for one day. The royalty and the marketing fee are worked out from the percentages.
- **Invoice It** makes one ERP invoice to the franchisee's customer with the royalty and the marketing fee each on its own line. An invoiced report is not changed.
- **Numbering.** `ROY-{year}-{5 digits}`.
- **Your franchise, on the portal.** A franchisee signs in to the workspace's portal and sees their agreements, the sales they reported and the fees on them, as money. Each invoice opens on ERP's Your invoices once issued.
- **They report under their own agreements only,** by the same rules as a report made in the office, and the report keeps who made it.

### 🔐 Roles, privacy and APIs

- **Two roles,** below. Row-level security applies to every table.
- **Personal data.** A request that names a person by address finds the royalty reports they made, the transfers they asked for and the returns they took, and erasure takes the address off while the records stay.
- **Records for free.** Lists, boards, calendars, forms, search, exports, audit trail, REST and MCP.
- **The assistant** knows what the records mean and suggests questions such as which collect orders are ready but not collected and which franchisees have not reported this month.

### 🌍 Languages

- **English, Arabic and Hindi** for the app's words and the names of its records. Pages mirror right to left. The Arabic and Hindi text was translated by a model and is waiting for review by native speakers.

## 🧭 How it works

```mermaid
flowchart LR
    O["Online order (ERP sales order)"] --> C["Click and Collect"]
    O --> S["Ship From Store"]
    C -->|"store holds every line"| R["Ready"]
    R -->|"pickup code"| H["Handed over: delivery note"]
    S --> D["Shipped: delivery note from the store"]
    C -->|"store lacks some"| T["Transfer from another store"]
    T --> C
```

One order has one fulfilment: collected in a store, or shipped from one. Both end in an ERP delivery note from the store's stock.

```mermaid
flowchart LR
    ST["Store A asks Store B"] --> RQ["Requested"]
    RQ --> SN["Sent: stock to goods in transit"]
    SN --> PR["Part Received"]
    SN --> RC["Received"]
    PR --> RC
    PR --> CS["Closed Short: rest written off"]
    RQ --> RJ["Rejected or Cancelled"]
```

Transfers are ERP stock entries. The stock is always somewhere: in a store, or in the goods in transit.

## 🧱 Data model

| Entity | What it is | Key fields or states |
|---|---|---|
| **Click and Collect** | An online order collected in a store | Sales order, store, status (Ordered, Ready, Collected, Cancelled), pickup code, delivery note, transfer |
| **Store Shipment** | An online order shipped from a store | Sales order, store, status (To Ship, Shipped, Cancelled), carrier, tracking number, delivery note |
| **Store Return** | Goods taken back at a store | Sales invoice, customer, store, items with reasons, status (Draft, Taken Back, Cancelled), credit note |
| **Store Transfer** | Stock sent between stores | From, to, items with arrived and lost quantities, status, stock entries |
| **Franchisee** | A franchise agreement | Customer, territory, royalty percent, marketing percent, start and end |
| **Royalty Report** | A franchisee's sales for a period | Period, gross sales, royalty, marketing, total, status (Draft, Invoiced), sales invoice, who reported it |
| **Retail Settings** | One per workspace | Company, royalty item, marketing item, goods in transit warehouse |
| **Store Transfer Item, Store Return Item** | Rows inside the records above | Item, quantity, arrived, lost, reason |

| Role | What it can do |
|---|---|
| **Retail Manager** | Runs click and collect, shipping from stores and returns across the stores, and franchises and their royalties. Edits the settings |
| **Store Associate** | Gets orders ready and hands them over against their code, ships orders from the store, takes returns, and asks other stores for stock, sends it and takes it in |

Report: Stock by Store.

## 🔌 Integrations

| With | How | Needs |
|---|---|---|
| **ERP** | Sales orders, delivery notes, sales invoices and credit notes, stock entries, warehouses, customers | ERP installed (done for you) |
| **ERP Procurement** | Reorder levels raise the transfers that replenish stores | Auto-reorder levels on items |
| **Point of Sale** | Goods sold at a till come back to any store through the credit note | Point of Sale installed |
| **Commerce** | Web orders are collected or shipped from a store, and Commerce's returns count against what was sold | Commerce installed |
| **The portal** | A franchisee reports sales and sees fees | A portal account for the franchisee |
| **Webhooks and the outbox** | Record events from the platform | An endpoint |
| **REST and MCP** | Every record and action, within each person's permissions | An API token |

Carrier labels and tracking, a shop-floor stock count by scanning, and online marketplaces are not connected yet.

## 📊 How it compares

Leaders: Lightspeed Retail, Shopify POS, Oracle Retail (Xstore), Microsoft Dynamics 365 Commerce and Odoo (open source).

| Capability | Ferrum Retail | Lightspeed | Shopify | Oracle Retail | Dynamics 365 Commerce | Odoo |
|---|:-:|:-:|:-:|:-:|:-:|:-:|
| Self-hosted, you hold the data | ✅ | ❌ | ❌ | ❌ | ❌ | ✅ |
| Open-source licence | ✅ | ❌ | ❌ | ❌ | ❌ | 🟡¹ |
| No per-store or per-register fee | ✅ | ❌ | ❌ | ❌ | ❌ | 🟡² |
| Sales, stock and accounts are one data set | ✅ | 🟡³ | 🟡³ | 🟡³ | ✅ | ✅ |
| Click and collect | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| Ship from store | ✅ | 🟡 | ✅ | ✅ | ✅ | 🟡 |
| Choose the store nearest the customer | ❌ | 🟡 | ✅ | ✅ | ✅ | ❌ |
| Endless aisle: see other stores' stock | 🟡⁴ | ✅ | ✅ | ✅ | ✅ | 🟡 |
| Returns at any store | 🟡⁵ | ✅ | ✅ | ✅ | ✅ | 🟡 |
| Transfers between stores, partial receipt, write-off | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| Franchise royalties reported on a portal and invoiced | ✅ | ❌ | ❌ | 🟡⁶ | 🟡⁶ | ❌ |
| A till with offline selling | 🟡⁷ | ✅ | ✅ | ✅ | ✅ | ✅ |
| Gift cards and store credit | ✅⁷ | ✅ | ✅ | ✅ | ✅ | ✅ |
| Loyalty points and tiers | ✅⁷ | 🟡 | 🟡 | ✅ | ✅ | ✅ |
| Store-level prices and promotions | ❌ | 🟡 | 🟡 | ✅ | ✅ | ✅ |
| Cart discounts and buy-one-get-one | 🟡⁷ | ✅ | ✅ | ✅ | ✅ | ✅ |
| Size and colour variants | 🟡⁸ | ✅ | ✅ | ✅ | ✅ | ✅ |
| Replenishment from reorder levels | 🟡⁸ | ✅ | ✅ | ✅ | ✅ | ✅ |
| Stock counts by scanning on a handheld | ❌ | ✅ | ✅ | ✅ | ✅ | ✅ |
| Barcode and shelf-label printing | ❌ | ✅ | 🟡 | ✅ | ✅ | ✅ |
| Carrier labels and tracking booked with the carrier | ❌ | 🟡 | ✅ | ✅ | ✅ | ✅ |
| An online store | 🟡⁹ | ✅ | ✅ | ✅ | ✅ | ✅ |
| Open API and webhooks | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |

<sub>¹ Odoo's Community edition is open source; some features are Enterprise only. ² Community is free; Enterprise is paid per user. ³ By accounting integrations, not one data set. ⁴ Stock by Store is a report; it is not on the sale screen of the till. ⁵ The invoice is credited through ERP and the goods go into that store's stock; a cash refund at that store's till is not built. ⁶ Through franchise or partner modules, not out of the box. ⁷ In the Point of Sale app (stored value, loyalty, promotions and the offline till), not in Retail itself. ⁸ By ERP and its Procurement; the till lists variants as plain items. ⁹ By the Commerce app.</sub>

### 🏆 Where Retail stands out

- 🔁 Stock, orders, invoices and credit notes are ERP records, so a transfer, a hand-over and a return always add up with the books.
- 🤝 Franchise reporting and royalty invoicing are built in, with a portal for the franchisee.
- 📦 Transfers that arrive in parts and write off what is lost, with the goods in transit kept as a place.
- 🔐 A pickup code is checked at the hand-over, and an order is never both collected and shipped.
- 🏠 Self-hosted, open source, no per-store fee, with Arabic and Hindi.

### 🚧 Where the leaders are still ahead

- 📍 **Choosing the nearest store** to a customer, and store-level prices and promotions.
- 🏷️ **Hardware and handheld flows:** shelf labels, counting and receiving by scanning.
- 🚛 **Carriers:** labels and tracking booked with the carrier.
- 🛒 **Sales channel breadth:** marketplaces and a built-in storefront.
- 💳 **Refunds at the store** in cash, and special orders and layaway.
- 📊 **Cross-store analytics** such as sales by store and store comparison.

> [!NOTE]
> Leader columns are from public product information as of November 2026 and may change. Features marked 🟡 for a leader depend on plan, region or add-on.

## 📍 Roadmap

### ✅ Recently shipped

- [x] **0.3.13:** the interface on the tuned design system, with better menu icons and cleaner print.
- [x] **0.3.12:** the redesigned interface, with the stores' home on the work-home pattern.
- [x] **0.3.11:** reports named in Arabic and Hindi.
- [x] **0.3.3:** the franchise portal shows sales and fees as money.
- [x] **0.3.2:** ship from store works for an order that names no warehouse, as web orders do.
- [x] **0.3.0:** ship from store, returns to any store, Stock by Store and goods a collection lacks brought from another store, and transfers taken in parts with the lost rest written off.

### 🔜 Next up

**Fulfilment**
- [ ] Choose the store nearest the customer, not only the one holding the most.
- [ ] Book labels and tracking with the carrier.
- [ ] Refunds at the store: give the money back at that store's till and drawer.

**Prices and stock**
- [ ] Store-level prices and promotions.
- [ ] Stock on hand and other stores' stock on the till's sale screen.
- [ ] Counting and receiving stock by scanning on a handheld.

**Franchises**
- [ ] Audits of what franchisees report.

### 🔭 Later

- [ ] Special orders and layaway: a deposit now, the balance later, stock reserved.
- [ ] Shelf-label and barcode printing, in bulk by receipt or purchase order.
- [ ] Reorder by sales velocity and min and max per store.
- [ ] Sales by store and a store comparison dashboard.
- [ ] Sync with an online shop such as Shopify or WooCommerce, and marketplaces.
- [ ] Serial-numbered goods taken back through the app, not through ERP.

## 🚀 Install

Retail is a native Ferrum app. It ships in the official Ferrum server image, and a workspace administrator installs it from **Apps** (`/store/`). It requires ERP `>=0.1`, and installing it in a workspace without ERP installs ERP whole. Self-hosters who build their own image enable the `retail` feature of the server.

First steps after installing:

1. Give people a role: **Retail Manager** or **Store Associate**.
2. In ERP, tick **Is A Store** on each warehouse that is a shop.
3. In **Retail Settings**, choose the company, the items royalties and marketing fees are invoiced as, and a Goods in Transit warehouse if stock should travel between stores.
4. For replenishment, give each item an auto-reorder level for each store's warehouse in ERP Procurement, as a material transfer from the distribution centre.
5. For franchises, add a **Franchisee** for each agreement, and give each franchisee's people a portal account so they can report their sales at `/portal/franchise/`.

## 🔧 Develop

Clone it next to the platform:

```text
work/
├── ferrum/                  https://github.com/CBFerrum/Ferrum (with Ferrum-Frontend cloned into ferrum/web)
└── ferrum-app-retail/
```

Point the platform dependencies at your checkout (this file is git-ignored):

```toml
# .cargo/config.toml
[patch."https://github.com/CBFerrum/Ferrum"]
ferrum-domain = { path = "../ferrum/crates/domain" }
ferrum-kernel = { path = "../ferrum/crates/kernel" }
ferrum-meta = { path = "../ferrum/crates/meta" }
```

The tests need PostgreSQL with an ordinary (non-superuser) role, so row-level security applies. The two test files use their own databases:

```bash
createuser ferrum --pwprompt
createdb -O ferrum ferrum_retail_test && createdb -O ferrum ferrum_retail_omni_test
export FERRUM_RETAIL_TEST_DATABASE_URL=postgres://ferrum:ferrum@localhost:5432/ferrum_retail_test
export FERRUM_RETAIL_OMNI_TEST_DATABASE_URL=postgres://ferrum:ferrum@localhost:5432/ferrum_retail_omni_test
cargo fmt -- --check
cargo clippy --all-targets -- -D warnings
cargo test
```

Its tests also build on [ferrum-app-erp](https://github.com/CBFerrum/ferrum-app-erp), which Cargo takes from GitHub. The interface (`ui/`) is built with the platform's packages from the sibling `ferrum/web` checkout, and its built files (`ui/dist`) are committed and embedded in the crate:

```bash
cd ui
npm run build && npm run check && npm run typecheck && npm test
```

`tests/retail_it.rs` walks a chain of shops: an order collected in its store once stock came from the centre, a collection cancelled and made again, a franchisee's reports and their fees invoiced, who may do what, transfers refused, sent at once, sent through the goods in transit and taken in, turned down and cancelled, and a franchisee reporting on the portal.

`tests/omnichannel_it.rs` walks stores selling as one: an order shipped from the store holding all of it, Stock by Store, a collection's missing goods brought from the centre, returns refused for a draft invoice, a warehouse that is no store, more than was sold and an item not sold, goods returned to another store by a return delivery note or by the credit note, and a transfer taken in item by item with the rest written off.

## 📂 Project layout

```text
app.yaml          the manifest: id, version, requirements, what it adds to Ferrum
packages/         metadata: entities, roles, menu, the Is A Store field on warehouses
src/              the logic in Rust: collect, ship, returns, transfers, the aisle, franchises, the portal
ui/               its interface: the stores' home and the franchise portal page (React, TypeScript)
tests/            two end-to-end tests against a real PostgreSQL
docs/PROVENANCE.md  where the design comes from (its own, no code copied)
CHANGELOG.md      what changed in each version
```

## 📜 Changelog

Every version is in [CHANGELOG.md](CHANGELOG.md).

## 🤝 Contributing

Issues and pull requests are welcome at [github.com/CBFerrum/ferrum-app-retail](https://github.com/CBFerrum/ferrum-app-retail/issues). Run the checks in Develop first.

## 📄 License

[AGPL-3.0-only](LICENSE), like the platform.

<div align="center">

<sub>Part of <a href="https://github.com/CBFerrum/Ferrum">Ferrum</a>, the open business platform · Made with 🦀 Rust and 🐘 PostgreSQL</sub>

</div>
