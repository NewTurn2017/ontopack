# OntoPack synthetic performance benchmark — 2026-05-24

## Fixture

- Notes: 10000
- Media sidecars: 500
- Warmup requests per endpoint: 1
- Measured requests per endpoint: 3
- Pack: `/var/folders/6w/ryvjgm214g361w38k2x2dcch0000gn/T//ontopack-perf.pvtVpX`
- Server: `http://127.0.0.1:49395`
- Raw JSON: `output/perf/ontopack-perf-20260524-m7-10k.json`

## Results

| Endpoint | Path | wall p50 ms | wall p95 ms | wall max ms | payload elapsed p50 ms | max bytes |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| `capabilities` | `/api/capabilities` | 0.197 | 0.241 | 0.241 | None | 628 |
| `dashboard_all` | `/api/dashboard?gallery_k=12&timeline_k=10&graph_limit=80` | 26.570 | 26.623 | 26.623 | 25 | 29920 |
| `dashboard_image` | `/api/dashboard?type=image&gallery_k=12&timeline_k=10&graph_limit=80` | 27.772 | 28.295 | 28.295 | 26 | 29920 |
| `search_needle` | `/api/search?q=%EC%84%B1%EB%8A%A5%20%EB%8B%88%EB%93%A4&type=prompt&tag=needle&k=12` | 2.273 | 2.507 | 2.507 | 1 | 18436 |
| `search_common` | `/api/search?q=%EA%B3%B5%ED%86%B5%EC%A7%88%EB%AC%B8&k=12` | 20.638 | 20.726 | 20.726 | 19 | 17753 |
| `gallery` | `/api/gallery?k=24` | 1.364 | 1.526 | 1.526 | None | 35250 |
| `timeline` | `/api/timeline?from=2026-05-01&to=2026-05-28&k=24` | 1.172 | 1.208 | 1.208 | None | 4795 |
| `graph` | `/api/graph?limit=160` | 7.376 | 7.466 | 7.466 | None | 19541 |
| `note_detail` | `/api/notes/bench-0000` | 0.564 | 0.581 | 0.581 | None | 1581 |
| `related` | `/api/related/bench-0100?depth=1` | 0.440 | 0.519 | 0.519 | None | 222 |

## Interpretation

- This benchmark measures a persistent local server, not `pack serve --once` startup overhead.
- `wall_ms` includes localhost HTTP and JSON transfer; `payload_elapsed_ms` is server handler timing when the endpoint exposes it.
- If dashboard/timeline/gallery p95 grows faster than search p95 as note count increases, the next optimization should be endpoint-specific SQLite queries instead of materializing all indexed notes per request.
