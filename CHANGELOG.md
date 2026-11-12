# Changelog

## 0.3.13

Rebuilt with Ferrum-Frontend 45dad7f, the design system as tuned after the landed build was reviewed. The left menu's icons are chosen so that none repeats on more than three rows and each fits its row; their drawings load beside the first page instead of inside it, so the page starts lighter; a row that scrolls sideways fades at the edge where there is more, in every browser and with reduced motion; a printed record leaves out empty fields and keeps its totals with the table; menus and popovers sit inside the page's landmarks for assistive technology; a selection bar folds what does not fit into More on a narrow screen. Nothing else changes.

## 0.3.12

The interface follows the upgraded design system, rebuilt with Ferrum-Frontend 0a47157. Text uses the kit's type roles (nothing under 12px) and controls its height tokens; loading shows skeletons shaped like what is coming; empty, no matches, denied, offline and failed are told apart; status colours come from stored state ids. The interface check now runs the UI lint against an empty baseline. The home is rebuilt on the kit's work-home pattern: one page title, the main action as a button, queues with their counts, status at its natural width; its other own pages follow the same anatomy.

## 0.3.11

The reports it defines in code (their titles and parameters) have names in Arabic and Hindi too, with its models' and manifest's (133 each in ui/src/locales/names). Translated by the model, to be reviewed by native speakers.

## 0.3.10

Rebuilt with Ferrum-Frontend c59c9a2: the names of its models, fields and manifest in Arabic and Hindi (132 each in ui/src/locales/names, which the platform serves to every page of the workspace, so a record's labels, the launcher and the bar read in the person's language), and the sentences its pages still showed in English (split around a link or a figure, built in code from a template or word by word) whole and translated (27 words each in ui/src/locales). Translated by the model, to be reviewed by native speakers.

## 0.3.9

The assistant knows what its records mean in its pages (contributes.shell.assistant): their states, what counts and where figures come from, with questions it suggests asking there.

## 0.3.8

Rebuilt with Ferrum-Frontend 5b22e7b: its own words carry built-in Arabic and Hindi (26 words each, ui/src/locales, translated by the model and to be reviewed by native speakers), fetched for the page's language only. The labels it keeps in objects, its dialogs, its page titles and the words it keeps in variables go through t() as well, and the names and sizes it shows as they are keep their order right to left. Its pages are drawn once their words are in, and its entry carries only the parts of the kit it uses.

## 0.3.7

Rebuilt with Ferrum-Frontend 6537d65: an action's link, code or token shown once is kept in a dialog to copy, and on a page that reads right to left, English left untranslated keeps its own order. Its own words go through the kit's t() (28 places), so a workspace's own words for its language reach them, and Admin, Interface words lists them under it (dist/words.json).

## 0.3.6

Rebuilt with Ferrum-Frontend d5d98f2: a workspace's own words for its interface (Admin, Interface words) reach its pages as they load; an attachment field that holds a picture shows it; and a value another record already has is said so, where the form said the record had been changed by someone else.

## 0.3.5

Rebuilt with Ferrum-Frontend 63280d3: right to left, its pages mirror as they read (icons that point along the line, margins and alignment by the line's start and end, arrow keys along it), and the records pages' words are in Arabic and Hindi.

## 0.3.4

Rebuilt with Ferrum-Frontend a64416a: a new record's box whose default is 1 shows ticked, as it is saved.

## 0.3.3

Found walking the franchise portal on 7 November: Your franchise shows a report's sales and fees as
money ("sales 1,00,000.00 · fees 7,000.00"), not as raw numbers. Rebuilt with Ferrum-Frontend 922fcfb:
an action's dialog says every refusal, and an action with parameters and a confirmation asks once.

## 0.3.2

Ship from store works for an order that names no warehouse, as an order from the web does. Its
delivery note was made by ERP's "Create Delivery Note", which copies the order's warehouse, and
ERP refused a note whose lines had none. It is now mapped from the order the same way, every
line from the store. Found walking it in a browser on 6 November.

A collect order for an order already collected, or being shipped from a store, says so first,
whichever store it names.

## 0.3.1

Rebuilt with Ferrum-Frontend a76295e: its pages show times on a person's own clock when they chose one on their Account page, while today and due dates stay on the workspace's day. Its pages count today on the workspace's day, not UTC's, so the day's figures are right in the evening west of Greenwich and the early morning east of it.

## 0.3.0

Stores selling as one (FERRUM-INDUSTRY-003).

- **Stores:** a warehouse marked Is A Store (on the warehouse, added by Retail) is a shop.
- **Ship From Store:** an online order shipped to the customer from a store's stock. Left empty,
  its store is the one holding the most of it among the stores holding all of it; a named one must
  be a store of the order's company. Ship It makes the order's delivery note from that store and
  submits it, keeping the carrier and tracking number. An order is shipped from a store once, and
  never also collected in one.
- **The endless aisle:** the Stock by Store report says where an item is, the stores first and the
  most first. Bring It From Another Store, on a click and collect, asks a store for what the
  collecting store lacks, as a store transfer.
- **Store Return:** goods bought online or in another store, brought back to any store against
  their invoice, never more of an item than it sold less what came back already. Take It Back
  credits the invoice for them through ERP, at what was paid, and puts them in that store's stock
  at the cost they left at: by the credit note when the invoice delivered them, or else by a return
  delivery note against each note that did.
- **Transfers in parts:** Some Of It Arrived takes in what arrived of an item; It Arrived takes the
  rest. The Rest Is Lost writes off what never arrived out of the goods in transit, saying what
  happened, and closes the transfer short. Each line keeps what arrived and what was lost.
- **Click and collect counts what is left to deliver** of the order's stock items, in stock units:
  a service on the order no longer holds it back.
- The stores page lists orders to ship from a store and returns to take back, and links Stock by
  Store.

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
