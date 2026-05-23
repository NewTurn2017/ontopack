# OntoPack synthetic performance benchmark — 2026-05-23

## Fixture

- Notes: 1200
- Media sidecars: 120
- Warmup requests per endpoint: 1
- Measured requests per endpoint: 5
- Pack: `/var/folders/6w/ryvjgm214g361w38k2x2dcch0000gn/T//ontopack-perf.IzwaIL`
- Server: `http://127.0.0.1:55487`
- Raw JSON: `/Users/genie/dev/ontopack/output/perf/ontopack-perf-20260523-m5f.json`

## Results

| Endpoint | Path | wall p50 ms | wall p95 ms | wall max ms | payload elapsed p50 ms | max bytes |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| `capabilities` | `/api/capabilities` | 0.215 | 0.253 | 0.253 | None | 598 |
| `dashboard_all` | `/api/dashboard?gallery_k=12&timeline_k=10&graph_limit=80` | 37.179 | 44.981 | 44.981 | 35 | 29749 |
| `dashboard_image` | `/api/dashboard?type=image&gallery_k=12&timeline_k=10&graph_limit=80` | 36.083 | 37.402 | 37.402 | 34 | 29740 |
| `search_needle` | `/api/search?q=%EC%84%B1%EB%8A%A5%20%EB%8B%88%EB%93%A4&type=prompt&tag=needle&k=12` | 0.982 | 1.060 | 1.060 | 0 | 9130 |
| `search_common` | `/api/search?q=%EA%B3%B5%ED%86%B5%EC%A7%88%EB%AC%B8&k=12` | 3.001 | 3.097 | 3.097 | 2 | 17586 |
| `gallery` | `/api/gallery?k=24` | 9.269 | 9.393 | 9.393 | None | 34890 |
| `timeline` | `/api/timeline?from=2026-05-01&to=2026-05-28&k=24` | 9.372 | 9.434 | 9.434 | None | 4815 |
| `graph` | `/api/graph?limit=160` | 9.380 | 9.469 | 9.469 | None | 19597 |
| `note_detail` | `/api/notes/bench-0000` | 8.896 | 8.992 | 8.992 | None | 1566 |
| `related` | `/api/related/bench-0100?depth=1` | 9.215 | 9.559 | 9.559 | None | 222 |

## Interpretation

- This benchmark measures a persistent local server, not `pack serve --once` startup overhead.
- `wall_ms` includes localhost HTTP and JSON transfer; `payload_elapsed_ms` is server handler timing when the endpoint exposes it.
- If dashboard/timeline/gallery p95 grows faster than search p95 as note count increases, the next optimization should be endpoint-specific SQLite queries instead of materializing all indexed notes per request.

## Benchmark-driven fix

The first persistent-server run exposed a request-reader issue: `read_http_request` did not stop at the `\r\n\r\n` header terminator and could wait for EOF/read-timeout on normal HTTP clients. The HTTP reader now stops on `\r\n\r\n` or bare `\n\n`, and `read_http_request_stops_at_header_terminator_without_waiting_for_eof` locks the behavior.

## Decision

The next backend speed slice should target endpoint-specific SQLite reads. Search is already sub-3.1ms p95 on the 1,200-note fixture, while dashboard p95 is 44.981ms and the other non-search viewer APIs cluster around 9ms because they still materialize all indexed notes per request.
