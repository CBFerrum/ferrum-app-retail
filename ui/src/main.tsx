// Retail's own interface at /retail/: collections waiting, transfers between stores and royalties
// due, with every record in the records frame's lists and forms. The same release draws its page of
// the workspace's portal, "Your franchise" at /portal/franchise/ (Ferrum docs/plan/apps-ui.md §9.1).

import { useEffect } from "react";
import { Link, Outlet, useRouteError } from "react-router";
import { bootstrap, isolate, shell, t } from "@ferrum/client";
import { EmptyState, ErrorState } from "@ferrum/kit";
import { mountApp, RequireAuth, setFrameMode } from "@ferrum/records/frame";
import "./styles.css";

const page = (name: "FormView" | "ListView" | "ReportView") => () => import("./records").then((m) => ({ Component: m[name] }));

function RouteError() {
  return <ErrorState error={useRouteError()} onRetry={() => window.location.reload()} />;
}

function NotFound({ back = t("Go to the stores") }: { back?: string }) {
  return (
    <EmptyState title={t("Page not found")}>
      <Link to="/">{isolate(back)}</Link>
    </EmptyState>
  );
}

function Started() {
  useEffect(() => shell().ready(), []);
  return null;
}

/** The portal's page, when the platform serves this release there. */
const PORTAL = "portal/franchise";
const onPortal = bootstrap()?.app?.entry === PORTAL;

if (!onPortal) setFrameMode("app", ["retail"]);

const portalRoutes = [
  {
    path: "/",
    element: (
      <main>
        <Started />
        <Outlet />
      </main>
    ),
    errorElement: <RouteError />,
    children: [
      { index: true, lazy: () => import("./Portal").then((m) => ({ Component: m.FranchiseHome })) },
      { path: "new", lazy: () => import("./Portal").then((m) => ({ Component: m.FranchiseReport })) },
      { path: "*", get element() { return <NotFound back={t("Go to your franchise")} />; } },
    ],
  },
];

const staffRoutes = [
  {
    path: "/",
    element: (
      <>
        <Started />
        <RequireAuth />
      </>
    ),
    errorElement: <RouteError />,
    children: [
      { index: true, lazy: () => import("./Home").then((m) => ({ Component: m.Home })) },
      { path: "e/:entity", lazy: page("ListView") },
      { path: "e/:entity/:id", lazy: page("FormView") },
      { path: "r/:name", lazy: page("ReportView") },
      { path: "*", element: <NotFound /> },
    ],
  },
];

mountApp({
  catalogs: { ar: () => import("./locales/ar.json"), hi: () => import("./locales/hi.json") },
  ...(onPortal ? { route: PORTAL, routes: portalRoutes } : { route: "retail", routes: staffRoutes }),
});
