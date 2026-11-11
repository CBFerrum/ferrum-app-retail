// What waits on the stores: click and collect orders to get ready and to hand over, orders to ship
// from a store, returns to take back, transfers asked for and on the way, and royalty reports to
// invoice. Each opens its record; Stock by Store says where an item is, for what a store lacks.

import { useQuery } from "@tanstack/react-query";
import { api, docPath, t as tr, isolate } from "@ferrum/client";
import { ButtonLink, HomeSkeleton, LoadFailed, PageHead, Queue, QueueGrid, QueueItem } from "@ferrum/kit";

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

/** The most a queue lists before "and N more" opens the list. */
const LIMIT = 5;

/** One quiet line under a record's number: the names its links carry (the store, or from and to). */
const names = (r: Row, ...keys: string[]) => {
  const t = (r._titles ?? {}) as Record<string, unknown>;
  return keys.map((k) => t[k]).filter((v): v is string => typeof v === "string" && v !== "").join(" → ");
};

function Work({ title, list, entity, label, meta, tone, trailing }: { title: string; list: Row[]; entity: string; label: (r: Row) => string; meta?: (r: Row) => string; tone?: "attention"; trailing?: (r: Row) => React.ReactNode }) {
  return (
    <Queue title={title} count={list.length} tone={tone} more={{ to: `/e/${entity}`, count: list.length - LIMIT }}>
      {list.slice(0, LIMIT).map((r) => (
        <QueueItem key={r.id} to={`/e/${entity}/${r.id}`} title={isolate(label(r))} meta={meta?.(r) ? isolate(meta(r)) : undefined} trailing={trailing?.(r)} />
      ))}
    </Queue>
  );
}

export function Home() {
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
  const head = <PageHead title={tr("The stores")} actions={<ButtonLink to="/r/retail_stock_by_store">{tr("Stock by store")}</ButtonLink>} />;
  if (q.isPending) return <>{head}<HomeSkeleton queues={6} /></>;
  if (q.isError) return <>{head}<LoadFailed error={q.error} onRetry={() => void q.refetch()} /></>;
  const num = (r: Row) => String(r.number ?? "");
  const store = (r: Row) => names(r, "store");
  const route = (r: Row) => names(r, "from_store", "to_store");
  return (
    <>
      {head}
      <QueueGrid>
        {/* Royalties are for managers (a store associate sees none, so an empty queue would only mislead): shown, first, only when there are reports to invoice. */}
        {q.data.reports.length > 0 && (
          <Work title={tr("Royalties to invoice")} list={q.data.reports} entity="RoyaltyReport" label={num} tone="attention" trailing={(r) => <span className="text-sm font-medium tabular-nums">{isolate(String(r.total ?? ""))}</span>} />
        )}
        <Work title={tr("To get ready")} list={q.data.ordered} entity="CollectOrder" label={num} meta={store} />
        <Work title={tr("Ready to hand over")} list={q.data.ready} entity="CollectOrder" label={num} meta={store} />
        <Work title={tr("To ship from a store")} list={q.data.toShip} entity="StoreShipment" label={num} meta={store} />
        <Work title={tr("Returns to take back")} list={q.data.toTakeBack} entity="StoreReturn" label={num} meta={store} />
        <Work title={tr("Asked of another store")} list={q.data.asked} entity="StoreTransfer" label={num} meta={route} />
        <Work title={tr("On the way")} list={q.data.onTheWay} entity="StoreTransfer" label={num} meta={route} />
      </QueueGrid>
    </>
  );
}
