# OntoPack M5D fast search QA — 2026-05-23

## Scope

Verify the first M5D speed slice: stale viewer requests are cancelled, typed search is debounced, API timing is exposed, and long-note snippets point at the matching chunk.

## Implementation evidence

- Viewer JS uses `AbortController` for search/dashboard/ask request lanes.
- Search input uses a 180ms debounce for typed searches.
- Result, gallery, timeline, and graph panels get local loading states instead of page-level blocking.
- `/api/search`, `/api/ask`, and `/api/dashboard` include `elapsed_ms`.
- Keyword chunk search chooses a query-containing chunk when possible instead of always `ord = 0`.

## Regression tests

- `keyword_chunk_search_prefers_chunk_containing_query` covers long-note snippet selection.
- HTTP tests assert `elapsed_ms` on search, ask, and dashboard responses.
- Viewer JS tests assert debounce, `AbortController`, and local loading CSS are embedded.
- `scripts/real-test.sh` asserts timing fields on realistic search/ask/dashboard calls.

## Remaining work

- Add an explicit `source` field when server API supports keyword/vector/hybrid modes.
- Add browser timing screenshot/trace if future payload sizes make perceived latency visible.
- Embedded `/app.js` passes `node --check` after being served through `pack serve --once`.
