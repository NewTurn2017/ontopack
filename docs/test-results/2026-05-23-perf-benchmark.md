# OntoPack synthetic performance benchmark — 2026-05-23

## Fixture

- Notes: 1,200
- Media sidecars: 120
- Warmup requests per endpoint: 1
- Measured requests per endpoint: 5
- Baseline raw JSON: `output/perf/ontopack-perf-20260523-m5f.json`
- M5G raw JSON: `output/perf/ontopack-perf-20260523-m5g.json`

## M5G result

- Pack: `/var/folders/6w/ryvjgm214g361w38k2x2dcch0000gn/T//ontopack-perf.zo5h6r`
- Server: `http://127.0.0.1:51743`

| Endpoint | Path | wall p50 ms | wall p95 ms | wall max ms | payload elapsed p50 ms | max bytes |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| `capabilities` | `/api/capabilities` | 0.255 | 0.303 | 0.303 | None | 598 |
| `dashboard_all` | `/api/dashboard?gallery_k=12&timeline_k=10&graph_limit=80` | 5.589 | 6.094 | 6.094 | 4 | 29748 |
| `dashboard_image` | `/api/dashboard?type=image&gallery_k=12&timeline_k=10&graph_limit=80` | 5.661 | 5.991 | 5.991 | 4 | 29739 |
| `search_needle` | `/api/search?q=%EC%84%B1%EB%8A%A5%20%EB%8B%88%EB%93%A4&type=prompt&tag=needle&k=12` | 1.024 | 1.100 | 1.100 | 0 | 9130 |
| `search_common` | `/api/search?q=%EA%B3%B5%ED%86%B5%EC%A7%88%EB%AC%B8&k=12` | 3.403 | 3.923 | 3.923 | 2 | 17586 |
| `gallery` | `/api/gallery?k=24` | 1.408 | 1.491 | 1.491 | None | 34890 |
| `timeline` | `/api/timeline?from=2026-05-01&to=2026-05-28&k=24` | 0.628 | 0.646 | 0.646 | None | 4815 |
| `graph` | `/api/graph?limit=160` | 1.794 | 2.054 | 2.054 | None | 19597 |
| `note_detail` | `/api/notes/bench-0000` | 0.430 | 0.525 | 0.525 | None | 1566 |
| `related` | `/api/related/bench-0100?depth=1` | 0.393 | 0.592 | 0.592 | None | 222 |

## Before/after p50 comparison

| Endpoint | M5F p50 ms | M5G p50 ms | Speedup |
| --- | ---: | ---: | ---: |
| `dashboard_all` | 37.179 | 5.589 | 6.65x |
| `dashboard_image` | 36.083 | 5.661 | 6.37x |
| `gallery` | 9.269 | 1.408 | 6.58x |
| `timeline` | 9.372 | 0.628 | 14.92x |
| `graph` | 9.380 | 1.794 | 5.23x |
| `note_detail` | 8.896 | 0.430 | 20.69x |
| `related` | 9.215 | 0.393 | 23.45x |

Search was already fast in M5F and remains sub-4ms p95 on the same fixture.

## Benchmark-driven fix from M5F

The first persistent-server run exposed a request-reader issue: `read_http_request` did not stop at the `\r\n\r\n` header terminator and could wait for EOF/read-timeout on normal HTTP clients. The HTTP reader now stops on `\r\n\r\n` or bare `\n\n`, and `read_http_request_stops_at_header_terminator_without_waiting_for_eof` locks the behavior.

## Decision

M5G validates the endpoint-specific SQLite read strategy. The main viewer APIs are now below ~6.1ms p95 on the 1,200-note fixture. The next speed work should be either:

1. larger 10k-note benchmark to find the next scaling limit, or
2. dashboard/facets payload specialization if startup still feels heavy with very large tag sets.
