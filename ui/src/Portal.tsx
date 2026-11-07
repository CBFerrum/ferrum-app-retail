// Retail's page of the workspace's portal (Ferrum docs/plan/apps-ui.md §9.1) at /portal/franchise/:
// a franchisee's agreements with the sales they reported and the fees on them (each invoice opens on
// ERP's "Your invoices" once issued), and a new report of a period's sales. Everything comes from
// /api/app/retail/portal/…, which answers by the address the person signed in with.

import { useQuery, useQueryClient } from "@tanstack/react-query";
import { type FormEvent, useState } from "react";
import { Link, useNavigate } from "react-router";
import { api, errorMessage, formatValue, todayIso, t, isolate } from "@ferrum/client";
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
  useShellTitle(t("Your franchise"));
  const q = useFranchise();
  if (q.error) return <ErrorState error={q.error} onRetry={() => void q.refetch()} />;
  if (!q.data) return <PageLoading />;
  const { agreements } = q.data;
  const today = todayIso();
  return (
    <div className="stack np-page">
      <div className="page-head">
        <div>
          <h1 className="page-title">{t("Your franchise")}</h1>
          <p className="page-sub">{t("Your agreements, the sales you reported, and the fees on them.")}</p>
        </div>
        {reportable(agreements, today).length > 0 && (
          <Link className="btn btn-primary btn-md" to="/new">
            {t("Report your sales")}
          </Link>
        )}
      </div>
      {!agreements.length ? (
        <EmptyState title={t("No franchise here yet")} icon="store">
          <p className="muted">{t("When an agreement is made out to you, it is here with what you report under it.")}</p>
        </EmptyState>
      ) : (
        agreements.map((a) => (
          <section key={a.id} className="card card-pad stack" aria-label={t("Agreement {value}", { value: a.territory ?? "" })}>
            <header className="row" style={{ justifyContent: "space-between" }}>
              <h3>{a.territory || t("Your agreement")}</h3>
              <span className="faint">
                {t("{starts_on} to {ends_on} · royalty {royalty_percent}% · marketing", { starts_on: a.starts_on ?? "", ends_on: a.ends_on ?? "", royalty_percent: a.royalty_percent ?? "" })} {a.marketing_percent ?? 0}%
              </span>
            </header>
            {a.reports.length === 0 ? (
              <p className="muted">{t("Nothing reported yet.")}</p>
            ) : (
              <ul className="stack">
                {a.reports.map((r) => (
                  <li key={r.id}>
                    {t("{number} · {period_from} to {period_to} · sales", { number: r.number ?? "", period_from: r.period_from ?? "", period_to: r.period_to ?? "" })} {isolate(money(r.gross_sales))} {t("· fees")}{isolate(" ")}
                    {r.invoice ? <a href={`/portal/invoices/${r.invoice}`}>{isolate(money(r.total))}</a> : money(r.total)} <Badge tone={statusTone(r.status)}>{isolate(r.status ?? "")}</Badge>
                    {r.status === "Invoiced" && !r.issued && <span className="muted"> {t("· not issued yet")}</span>}
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
  useShellTitle(t("Report your sales"));
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
      toast(t("Your sales are reported"), "success");
      navigate("/");
    } catch (err) {
      toast(errorMessage(err), "error");
    } finally {
      setBusy(false);
    }
  };
  return (
    <form className="card card-pad stack np-page" onSubmit={(e) => void submit(e)}>
      <h1 className="page-title">{t("Report your sales")}</h1>
      <Field label={t("Agreement")}>
        <select className="input" value={chosen} onChange={(e) => setAgreement(e.target.value)}>
          {open.map((a) => (
            <option key={a.id} value={a.id}>
              {a.territory || t("Your agreement")} {t("· {starts_on} to {ends_on}", { starts_on: a.starts_on ?? "", ends_on: a.ends_on ?? "" })}
            </option>
          ))}
        </select>
      </Field>
      <Field label={t("From")}>
        <input className="input" type="date" value={from} required onChange={(e) => setFrom(e.target.value)} />
      </Field>
      <Field label={t("To")}>
        <input className="input" type="date" value={to} required onChange={(e) => setTo(e.target.value)} />
      </Field>
      <Field label={t("Gross sales")}>
        <input className="input" inputMode="decimal" value={sales} required onChange={(e) => setSales(e.target.value)} />
      </Field>
      <div className="row">
        <Button type="submit" variant="primary" loading={busy} disabled={!chosen || !from || !to || !sales.trim()}>
          {t("Report")}
        </Button>
        <Link to="/">{t("Back")}</Link>
      </div>
    </form>
  );
}
