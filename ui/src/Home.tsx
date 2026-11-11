// What waits on the stores: click and collect orders to get ready and to hand over, orders to ship
// from a store, returns to take back, transfers asked for and on the way, and royalty reports to
// invoice. Each opens its record; Stock by Store says where an item is, for what a store lacks.

import { useQuery } from "@tanstack/react-query";
import { Link } from "react-router";
import { api, docPath, t as tr, isolate } from "@ferrum/client";
import { ErrorState, ListSkeleton, useShellTitle } from "@ferrum/kit";

export interface Row {
  id: string;
  [k: string]: unknown;
}

const rows = (entity: string, filters: unknown[], extra: Record<string, string | number> = {}) =>
  api.get<{ rows: Row[] }>(docPath(entity), { filters: JSON.stringify(filters), limit: 500, ...extra }).then((r) => r.rows);

/** Collections by where they stand: those to get ready, and those ready to hand over. */
export function collections(list: Row[]): { ordered: Row[]; ready: Row[] } {
  return { ordered: list.filter((c) => c.status === "Ordered"), ready: list.filter((c) => c.status === "Ready") };
}

/** Transfers by where they stand: asked for and not yet sent, and on the way (some may have arrived). */
export function transfers(list: Row[]): { asked: Row[]; onTheWay: Row[] } {
  return { asked: list.filter((t) => t.status === "Requested"), onTheWay: list.filter((t) => t.status === "Sent" || t.status === "Part Received") };
}

function List({ title, list, entity, label }: { title: string; list: Row[]; entity: string; label: (r: Row) => string }) {
  return (
    <section className="card card-pad stack" aria-label={title}>
      <h3>
        {isolate(title)} ({list.length})
      </h3>
      <ul className="stack">
        {list.map((r) => (
          <li key={r.id}>
            <Link to={`/e/${entity}/${r.id}`}>{isolate(label(r))}</Link>
          </li>
        ))}
      </ul>
    </section>
  );
}

export function Home() {
  useShellTitle(tr("Retail"));
  const q = useQuery({
    queryKey: ["retail", "home"],
    queryFn: async () => {
      const [collect, toShip, toTakeBack, moving, reports] = await Promise.all([
        rows("CollectOrder", [["status", "in", ["Ordered", "Ready"]]]),
        rows("StoreShipment", [["status", "eq", "To Ship"]]),
        rows("StoreReturn", [["status", "eq", "Draft"]]),
        rows("StoreTransfer", [["status", "in", ["Requested", "Sent", "Part Received"]]]),
        // Store associates do not see royalties.
        rows("RoyaltyReport", [["status", "eq", "Draft"]]).catch(() => [] as Row[]),
      ]);
      return { ...collections(collect), toShip, toTakeBack, ...transfers(moving), reports };
    },
    refetchInterval: 30_000,
  });
  if (q.isPending) return <ListSkeleton className="np-page" />;
  if (q.isError) return <ErrorState error={q.error} onRetry={() => void q.refetch()} />;
  const num = (r: Row) => String(r.number ?? "");
  return (
    <div className="np-page">
      <header className="row" style={{ justifyContent: "space-between" }}>
        <h2>{tr("The stores")}</h2>
        <Link to="/r/retail_stock_by_store">{tr("Stock by store")}</Link>
      </header>
      <div className="np-columns">
        <List title={tr("To get ready")} list={q.data.ordered} entity="CollectOrder" label={num} />
        <List title={tr("Ready to hand over")} list={q.data.ready} entity="CollectOrder" label={num} />
        <List title={tr("To ship from a store")} list={q.data.toShip} entity="StoreShipment" label={num} />
        <List title={tr("Returns to take back")} list={q.data.toTakeBack} entity="StoreReturn" label={num} />
        <List title={tr("Asked of another store")} list={q.data.asked} entity="StoreTransfer" label={num} />
        <List title={tr("On the way")} list={q.data.onTheWay} entity="StoreTransfer" label={num} />
        <List title={tr("Royalties to invoice")} list={q.data.reports} entity="RoyaltyReport" label={(r) => `${num(r)} · ${String(r.total ?? "")}`} />
      </div>
    </div>
  );
}
