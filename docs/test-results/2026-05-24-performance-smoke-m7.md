# M7 performance smoke validation

## Claim

OntoPack now has a lightweight performance smoke gate on top of the existing synthetic benchmark so productization work can catch obvious viewer/API regressions.

## Scope

- `scripts/perf-benchmark.sh` now supports `SKIP_BUILD=1` so larger test flows can reuse an already-built `pack` binary.
- `scripts/perf-smoke.sh` runs a small synthetic benchmark, verifies required endpoints are present, and fails if any endpoint p95 exceeds a configurable threshold.
- `scripts/real-test.sh` runs the performance smoke with an 80-note / 8-media fixture and a conservative 1000ms p95 ceiling.
- The existing large benchmark remains available through `scripts/perf-benchmark.sh` with configurable `NOTE_COUNT`, `MEDIA_COUNT`, `REPEATS`, and artifact paths.

## Evidence

- `scripts/real-test.sh` — passed with performance smoke assertions.
- `cargo test` — passed.
- `cargo clippy --all-targets -- -D warnings` — passed.
- `cargo check -p pack-cli --features real-embed` — passed.
- `git diff --check` — passed.

## Notes

The smoke threshold is intentionally broad to avoid flaky CI failures while still catching pathological latency or broken endpoint regressions. For release-grade scaling checks, run a larger benchmark manually, for example:

```bash
NOTE_COUNT=10000 MEDIA_COUNT=500 REPEATS=5 WARMUP=1 scripts/perf-benchmark.sh
```
