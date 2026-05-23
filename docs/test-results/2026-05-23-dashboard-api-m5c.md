# OntoPack M5C dashboard aggregate QA — 2026-05-23

## Scope

Verify the first M5C speed slice: initial viewer panel data and filter-driven panel refreshes use `/api/dashboard` instead of separate facets/gallery/timeline/graph requests.

## Implementation evidence

- Added `/api/dashboard` HTTP route.
- Added `DashboardResponse` with `facets`, `gallery`, `timeline`, and `graph` sections.
- Viewer startup now calls `loadDashboard()` directly.
- Filter changes call dashboard once for panels and search only when a query exists.

## Regression tests

- `api_dashboard_http_returns_startup_panels` verifies the aggregate route includes facets, gallery media metadata, timeline notes, and graph nodes.
- `viewer_js_reruns_search_when_filters_change` now checks for `/api/dashboard` and `loadDashboard()`.

## Remaining work

- Add AbortController cancellation and timing metrics in M5D.
- Add media-kind counts and endpoint-specific SQL if dashboard payloads become large.
