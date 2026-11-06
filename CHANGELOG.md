# Changelog

## 0.2.2

Rebuilt with Ferrum-Frontend 24c3951: its pages show times on the workspace's clock, naming the zone when it is not the browser's, a draft numbered on submit is headed as a draft, and a record of an app that is not installed says so.

## 0.2.1

Rebuilt with Ferrum-Frontend bb60115: a record page reads an empty number or check as the server does, so an action whose condition needs one shows, and a workspace's settings open as their one record.

## 0.2.0

Transfers between stores, and franchisees on the portal (FERRUM-INDUSTRY-003).

- **Store transfers:** a store asks another for stock. Send It moves only what the giving store
  holds, as a submitted ERP stock entry into the settings' goods in transit. It Arrived takes it
  into the store that asked. Without a transit warehouse, it arrives as it is sent. A transfer
  may be turned down, saying why, or cancelled. Once sent, it is kept as it was. The stores page
  lists transfers asked for and on the way.
- **Your franchise:** a page of the workspace's portal, at /portal/franchise/. A franchisee sees
  their agreements, the sales they reported and the fees on them; each invoice opens on Your
  invoices once issued. They report a period's sales under their own agreements, by the office's
  rules. The report keeps who made it, and personal-data requests reach it.
- Rebuilt with Ferrum-Frontend 4a3681c.

## 0.1.3

Rebuilt with Ferrum-Frontend 00dea07: in a workflow state that names what may change, the other fields show read-only, and the fields the state needs show required.

## 0.1.2

Rebuilt with Ferrum-Frontend daeadda: after an action runs, a record page shows what the action said it did rather than only that it is done.

## 0.1.1

Rebuilt with Ferrum-Frontend 80ff179: a field's label names its control for assistive technology, and a record whose title is a link shows that record's title, not its id.

## 0.1.0

Retail and distribution (FERRUM-INDUSTRY-003).

- Click and collect: an order collected from the store it ships from, one collection to an order,
  ready once the store holds it, handed over against its six-digit code, its delivery note then
  made and submitted.
- Franchisees with their royalty and marketing percentages and agreement; sales reported for
  periods within it, never two for a day; the fees invoiced through ERP once.
- Its own interface at `/retail/`: collections waiting, and royalties to invoice.
- Store replenishment is left to ERP Procurement's auto-reorder levels.
