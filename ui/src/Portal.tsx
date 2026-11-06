// Retail's page of the workspace's portal (Ferrum docs/plan/apps-ui.md §9.1) at /portal/franchise/:
// a franchisee's agreements with the sales they reported and the fees on them (each invoice opens on
// ERP's "Your invoices" once issued), and a new report of a period's sales. Everything comes from
// /api/app/retail/portal/…, which answers by the address the person signed in with.

import { useQuery, useQueryClient } from "@tanstack/react-query";
import { type FormEvent, useState } from "react";
import { Link, useNavigate } from "react-router";
import { api, errorMessage, formatValue, todayIso } from "@ferrum/client";
import { Badge, Button, EmptyState, ErrorState, Field, PageLoading, statusTone, toast, useShellTitle } from "@ferrum/kit";

const BASE = "/api/app/retail/portal";

export interface PortalReport {
  id: string;
  number?: string | null;
  period_from?: string | null;
  period_to?: string | null;
  gross_sales?: string | null;
  total?: string | null;
  status?: string | null;
  invoice?: string | null;
  issued?: boolean;
}

export interface Agreement {
  id: string;
  territory?: string | null;
  royalty_percent?: string | null;
  marketing_percent?: string | null;
  starts_on?: string | null;
  ends_on?: string | null;
  reports: PortalReport[];
}

/** An amount as money: digits grouped, two decimals. */
const money = (v: string | null | undefined) => formatValue({ type: "currency" }, v);

/** The agreements a franchisee may report on `today` (YYYY-MM-DD): those in force or ended, never ones not begun. */
export function reportable(agreements: Agreement[], today: string): Agreement[] {
  return agreements.filter((a) => (a.starts_on ?? "") <= today);
}

const useFranchise = () =>
  useQuery({ queryKey: ["retail", "portal", "franchise"], queryFn: ({ signal }) => api.get<{ agreements: Agreement[] }>(`${BASE}/franchise`, {}, signal) });

export function FranchiseHome() {
  useShellTitle("Your franchise");
  const q = useFranchise();
  if (q.error) return <ErrorState error={q.error} onRetry={() => void q.refetch()} />;
  if (!q.data) return <PageLoading />;
  const { agreements } = q.data;
  const today = todayIso();
  return (
    <div className="stack np-page">
      <div className="page-head">
        <div>
          <h1 className="page-title">Your franchise</h1>
          <p className="page-sub">Your agreements, the sales you reported, and the fees on them.</p>
        </div>
        {reportable(agreements, today).length > 0 && (
          <Link className="btn btn-primary btn-md" to="/new">
            Report your sales
          </Link>
        )}
      </div>
      {!agreements.length ? (
        <EmptyState title="No franchise here yet" icon="store">
          <p className="muted">When an agreement is made out to you, it is here with what you report under it.</p>
        </EmptyState>
      ) : (
        agreements.map((a) => (
          <section key={a.id} className="card card-pad stack" aria-label={`Agreement ${a.territory ?? ""}`}>
            <header className="row" style={{ justifyContent: "space-between" }}>
              <h3>{a.territory || "Your agreement"}</h3>
              <span className="faint">
                {a.starts_on} to {a.ends_on} · royalty {a.royalty_percent}% · marketing {a.marketing_percent ?? 0}%
              </span>
            </header>
            {a.reports.length === 0 ? (
              <p className="muted">Nothing reported yet.</p>
            ) : (
              <ul className="stack">
                {a.reports.map((r) => (
                  <li key={r.id}>
                    {r.number} · {r.period_from} to {r.period_to} · sales {money(r.gross_sales)} · fees{" "}
                    {r.invoice ? <a href={`/portal/invoices/${r.invoice}`}>{money(r.total)}</a> : money(r.total)} <Badge tone={statusTone(r.status)}>{r.status}</Badge>
                    {r.status === "Invoiced" && !r.issued && <span className="muted"> · not issued yet</span>}
                  </li>
                ))}
              </ul>
            )}
          </section>
        ))
      )}
    </div>
  );
}

export function FranchiseReport() {
  useShellTitle("Report your sales");
  const navigate = useNavigate();
  const qc = useQueryClient();
  const q = useFranchise();
  const [agreement, setAgreement] = useState("");
  const [from, setFrom] = useState("");
  const [to, setTo] = useState("");
  const [sales, setSales] = useState("");
  const [busy, setBusy] = useState(false);
  if (q.error) return <ErrorState error={q.error} onRetry={() => void q.refetch()} />;
  if (!q.data) return <PageLoading />;
  const open = reportable(q.data.agreements, todayIso());
  const chosen = agreement || open[0]?.id || "";
  const submit = async (e: FormEvent) => {
    e.preventDefault();
    setBusy(true);
    try {
      await api.post(`${BASE}/reports`, { franchisee: chosen, period_from: from, period_to: to, gross_sales: sales.trim() });
      await qc.invalidateQueries({ queryKey: ["retail", "portal", "franchise"] });
      toast("Your sales are reported", "success");
      navigate("/");
    } catch (err) {
      toast(errorMessage(err), "error");
    } finally {
      setBusy(false);
    }
  };
  return (
    <form className="card card-pad stack np-page" onSubmit={(e) => void submit(e)}>
      <h1 className="page-title">Report your sales</h1>
      <Field label="Agreement">
        <select className="input" value={chosen} onChange={(e) => setAgreement(e.target.value)}>
          {open.map((a) => (
            <option key={a.id} value={a.id}>
              {a.territory || "Your agreement"} · {a.starts_on} to {a.ends_on}
            </option>
          ))}
        </select>
      </Field>
      <Field label="From">
        <input className="input" type="date" value={from} required onChange={(e) => setFrom(e.target.value)} />
      </Field>
      <Field label="To">
        <input className="input" type="date" value={to} required onChange={(e) => setTo(e.target.value)} />
      </Field>
      <Field label="Gross sales">
        <input className="input" inputMode="decimal" value={sales} required onChange={(e) => setSales(e.target.value)} />
      </Field>
      <div className="row">
        <Button type="submit" variant="primary" loading={busy} disabled={!chosen || !from || !to || !sales.trim()}>
          Report
        </Button>
        <Link to="/">Back</Link>
      </div>
    </form>
  );
}
