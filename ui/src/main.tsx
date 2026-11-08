// Retail's own interface at /retail/: collections waiting, transfers between stores and royalties
// due, with every record in the records frame's lists and forms. The same release draws its page of
// the workspace's portal, "Your franchise" at /portal/franchise/ (Ferrum docs/plan/apps-ui.md §9.1).

import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { StrictMode, useEffect } from "react";
import { createRoot } from "react-dom/client";
import { createBrowserRouter, Link, Outlet, RouterProvider, useRouteError } from "react-router";
import { addCatalogs, bootstrap, isolate, routeBase, shell, t, wordsReady } from "@ferrum/client";
import { ConfirmHost, EmptyState, ErrorState, Toaster } from "@ferrum/kit";
import { RequireAuth, setFrameMode } from "@ferrum/records/frame";
import "./styles.css";

// Its own words in the page's language (src/locales), fetched as the page starts; it is drawn once they are in.
void addCatalogs({ ar: () => import("./locales/ar.json"), hi: () => import("./locales/hi.json") });

const page = (name: "FormView" | "ListView" | "ReportView" | "AppSettingsPage" | "ScreenPage") => () => import("./records").then((m) => ({ Component: m[name] }));

const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false, refetchOnWindowFocus: false, staleTime: 5_000 } } });

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
      { path: "apps/:app/settings", lazy: page("AppSettingsPage") },
      { path: "x/:app/:page", lazy: page("ScreenPage") },
      { path: "*", element: <NotFound /> },
    ],
  },
];

const router = onPortal
  ? createBrowserRouter(portalRoutes, { basename: routeBase(PORTAL) })
  : createBrowserRouter(staffRoutes, { basename: routeBase("retail") });

// Drawn once the page's words are in (its language's catalogs, fetched as the page starts).
void wordsReady().then(() =>
  createRoot(document.getElementById("root")!).render(
    <StrictMode>
      <QueryClientProvider client={queryClient}>
        <RouterProvider router={router} />
        <Toaster />
        <ConfirmHost />
      </QueryClientProvider>
    </StrictMode>,
  ),
);
