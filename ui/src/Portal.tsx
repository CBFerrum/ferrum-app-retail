// Retail's page of the workspace's portal (Ferrum docs/plan/apps-ui.md §9.1) at /portal/franchise/:
// a franchisee's agreements with the sales they reported and the fees on them (each invoice opens on
// ERP's "Your invoices" once issued), and a new report of a period's sales. Everything comes from
// /api/app/retail/portal/…, which answers by the address the person signed in with.

import { useQuery, useQueryClient } from "@tanstack/react-query";
import { type FormEvent, useState } from "react";
import { useNavigate } from "react-router";
import { api, errorMessage, formatValue, todayIso, t, isolate } from "@ferrum/client";
import { Button, ButtonLink, Field, HomeSection, HomeSkeleton, LoadFailed, PageHead, PageStack, StatePanel, StatusBadge, toast } from "@ferrum/kit";

// RoyaltyReport.status as stored (packages/retail/entities/retail.yaml).
const REPORT_TONES = { Draft: "neutral", Invoiced: "green" } as const;

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
  const q = useFranchise();
  const title = t("Your franchise");
  const sub = t("Your agreements, the sales you reported, and the fees on them.");
  if (q.error) return <PageStack><PageHead title={title} description={sub} /><LoadFailed error={q.error} onRetry={() => void q.refetch()} /></PageStack>;
  if (!q.data) return <PageStack><PageHead title={title} description={sub} /><HomeSkeleton queues={2} /></PageStack>;
  const { agreements } = q.data;
  const today = todayIso();
  return (
    <PageStack>
      <PageHead
        title={title}
        description={sub}
        actions={reportable(agreements, today).length > 0 ? <ButtonLink to="/new" variant="primary" icon="plus">{t("Report your sales")}</ButtonLink> : undefined}
      />
      {!agreements.length ? (
        <StatePanel kind="empty" title={t("No franchise here yet")}>
          {t("When an agreement is made out to you, it is here with what you report under it.")}
        </StatePanel>
      ) : (
        agreements.map((a) => (
          <HomeSection key={a.id} title={a.territory || t("Your agreement")}>
          <div className="grid gap-2">
            <p className="text-xs text-muted-foreground">
              {isolate(t("{starts_on} to {ends_on} · royalty {royalty_percent}% · marketing", { starts_on: a.starts_on ?? "", ends_on: a.ends_on ?? "", royalty_percent: a.royalty_percent ?? "" }))} {isolate(`${a.marketing_percent ?? 0}%`)}
            </p>
            {a.reports.length === 0 ? (
              <p className="text-sm text-muted-foreground">{t("Nothing reported yet.")}</p>
            ) : (
              <div className="table-wrap" tabIndex={0} role="region" aria-label={t("Agreement {value}", { value: a.territory ?? "" })}>
                  <table className="table">
                    <thead>
                      <tr>
                        <th scope="col">{t("Report")}</th>
                        <th scope="col">{t("Period")}</th>
                        <th scope="col" className="num">{t("Sales")}</th>
                        <th scope="col" className="num">{t("Fees")}</th>
                        <th scope="col">{t("Status")}</th>
                      </tr>
                    </thead>
                    <tbody>
                      {a.reports.map((r) => (
                        <tr key={r.id}>
                          <td className="nowrap">{isolate(r.number ?? "")}</td>
                          <td className="nowrap">{t("{period_from} to {period_to}", { period_from: r.period_from ?? "", period_to: r.period_to ?? "" })}</td>
                          <td className="num">{isolate(money(r.gross_sales))}</td>
                          <td className="num">{r.invoice ? <a href={`/portal/invoices/${r.invoice}`}>{isolate(money(r.total))}</a> : isolate(money(r.total))}</td>
                          <td>
                            <StatusBadge value={r.status} tones={REPORT_TONES} label={isolate(r.status ?? "")} />
                            {r.status === "Invoiced" && !r.issued && <span className="text-xs text-muted-foreground"> {t("· not issued yet")}</span>}
                          </td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
              </div>
            )}
          </div>
          </HomeSection>
        ))
      )}
    </PageStack>
  );
}

export function FranchiseReport() {
  const navigate = useNavigate();
  const qc = useQueryClient();
  const q = useFranchise();
  const [agreement, setAgreement] = useState("");
  const [from, setFrom] = useState("");
  const [to, setTo] = useState("");
  const [sales, setSales] = useState("");
  const [busy, setBusy] = useState(false);
  const head = <PageHead title={t("Report your sales")} back={{ to: "/", label: t("Your franchise") }} />;
  if (q.error) return <PageStack>{head}<LoadFailed error={q.error} onRetry={() => void q.refetch()} /></PageStack>;
  if (!q.data) return <PageStack>{head}<HomeSkeleton queues={1} /></PageStack>;
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
    <PageStack>
    {head}
    <form className="card card-pad stack retail-form" onSubmit={(e) => void submit(e)}>
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
        <ButtonLink to="/" variant="ghost">{t("Back")}</ButtonLink>
      </div>
    </form>
    </PageStack>
  );
}
