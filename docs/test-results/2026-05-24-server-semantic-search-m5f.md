# M5F server semantic search capability validation

Date: 2026-05-24

## Goal

Connect the already-verified CLI/core vector/hybrid retrieval path to `pack-server`/viewer capabilities without making the default local viewer download models.

## Acceptance criteria

- Default `pack serve` / `pack open` remains keyword-only and model-download-free.
- `pack serve --semantic` / `pack open --semantic` are explicit opt-ins.
- Non-`real-embed` builds fail honestly when `--semantic` is requested.
- `real-embed` builds can compile a server state that loads one process-level BGE-M3 embedder.
- `/api/capabilities` reports vector/hybrid as available only when the server state has an embedder.
- `/api/search?mode=vector|hybrid` routes through sqlite-vec/RRF when an embedder is available, and still rejects semantic modes when unavailable.
- Semantic-mode API filtering keeps type/tag/date constraints intact.

## Validation log

- `cargo test -p pack-server api_search_uses_state_embedder_for_vector_mode` — passed.
- `cargo test -p pack-server api_search_state_embedder_supports_hybrid_mode_with_filters` — passed.
- `cargo test -p pack-server` — passed (`31 passed`).
- `cargo test -p pack-cli serve_once_prints_local_url_and_handles_one_request` — passed.
- `cargo test -p pack-cli serve_semantic_requires_real_embed_build` — passed.
- `cargo check -p pack-cli --features real-embed` — passed.
- `cargo clippy --all-targets -- -D warnings` — passed.
- `cargo test` — passed across workspace (`pack-cli` 34, `pack-core` 50, `pack-mcp` 10, `pack-server` 31).
- `scripts/real-test.sh` — passed with realistic pack + CLI + exports + MCP + viewer APIs + filter stress + open URL.
- `git diff --check` — passed.

## Known gaps

- No live BGE-M3 server request was executed in this slice; the compile gate verifies the `real-embed` path and unit tests use a deterministic in-process fake embedder.
- The server still depends on existing chunk embeddings; users should run `pack embed` before starting `pack serve --semantic`.
