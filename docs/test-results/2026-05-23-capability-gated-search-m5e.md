# OntoPack M5E capability-gated search QA — 2026-05-23

## Scope

Verify that the viewer exposes semantic/vector search honestly: keyword is available, vector/hybrid are visible but locked until the server actually supports real embeddings.

## Implementation evidence

- Added `/api/capabilities` with `keyword` available and `vector`/`hybrid` unavailable.
- Added `mode` and `source` to `/api/search` responses.
- `/api/search?mode=vector|hybrid` returns a JSON 400 instead of pretending to run semantic search.
- Viewer search mode selector is populated from `/api/capabilities` and disables locked modes.

## Regression tests

- `api_capabilities_reports_keyword_only_server_modes` verifies the capability payload.
- `api_search_rejects_unavailable_vector_mode` verifies honest rejection.
- Viewer shell tests assert `mode-filter` and `/api/capabilities` are embedded.
- `scripts/real-test.sh` checks search mode/source and capability-gated vector rejection against a realistic pack.

## Remaining work

- Add a real server-side embedder lifecycle when pack-server gains a `real-embed` feature.
- Enable vector/hybrid only after embeddings exist and the embedder is loaded once per server process.

## Browser QA

- Pack: `/var/folders/6w/ryvjgm214g361w38k2x2dcch0000gn/T//ontopack-real-test.7T5Vvg`
- URL: `http://127.0.0.1:59414`
- Screenshot: `output/playwright/ontopack-capability-gated-search-m5e-20260523.png`
- Mode selector: `keyword` enabled; `vector` and `hybrid` disabled with locked explanation.
- Search smoke: `다이어그램` returned `1 SOURCE CARD` and updated summary to `SEARCH MODE: KEYWORD · sqlite_fts`.
- Browser console: 0 errors/warnings.
