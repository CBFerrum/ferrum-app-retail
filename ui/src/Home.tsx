// What waits on the stores: click and collect orders to get ready and to hand over, and royalty
// reports to invoice. Each opens its record.

import { useQuery } from "@tanstack/react-query";
import { Link } from "react-router";
import { api, docPath } from "@ferrum/client";
import { ErrorState, PageLoading, useShellTitle } from "@ferrum/kit";

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

function List({ title, list, entity, label }: { title: string; list: Row[]; entity: string; label: (r: Row) => string }) {
  return (
    <section className="card card-pad stack" aria-label={title}>
      <h3>
        {title} ({list.length})
      </h3>
      <ul className="stack">
        {list.map((r) => (
          <li key={r.id}>
            <Link to={`/e/${entity}/${r.id}`}>{label(r)}</Link>
          </li>
        ))}
      </ul>
    </section>
  );
}

export function Home() {
  useShellTitle("Retail");
  const q = useQuery({
    queryKey: ["retail", "home"],
    queryFn: async () => {
      const [collect, reports] = await Promise.all([rows("CollectOrder", [["status", "in", ["Ordered", "Ready"]]]), rows("RoyaltyReport", [["status", "eq", "Draft"]])]);
      return { ...collections(collect), reports };
    },
    refetchInterval: 30_000,
  });
  if (q.isPending) return <PageLoading />;
  if (q.isError) return <ErrorState error={q.error} onRetry={() => void q.refetch()} />;
  const num = (r: Row) => String(r.number ?? "");
  return (
    <div className="np-page">
      <h2>The stores</h2>
      <div className="np-columns">
        <List title="To get ready" list={q.data.ordered} entity="CollectOrder" label={num} />
        <List title="Ready to hand over" list={q.data.ready} entity="CollectOrder" label={num} />
        <List title="Royalties to invoice" list={q.data.reports} entity="RoyaltyReport" label={(r) => `${num(r)} · ${String(r.total ?? "")}`} />
      </div>
    </div>
  );
}
