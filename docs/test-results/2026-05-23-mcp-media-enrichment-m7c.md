# M7C MCP media enrichment worker contract validation

Date: 2026-05-23

## Goal

Expose the durable media-enrichment loop through MCP so Claude/Codex or another external AI worker can operate on OntoPack without knowing storage internals.

## Acceptance criteria

- `tools/list` includes `media/list_pending`, `media/read_note`, `media/write_enrichment`, and `index/rebuild`.
- `media/list_pending` returns asset sidecar notes whose enrichment status is `pending`.
- `media/read_note` returns sidecar metadata, body/raw Markdown, and local asset paths for a worker.
- `media/write_enrichment` writes caption/tags/transcript/summary/keyframes through the managed enrichment block and preserves human-authored sidecar text.
- `index/rebuild` rebuilds the derived SQLite search index after enrichment writes.
- A term written by MCP enrichment becomes searchable through MCP `search` after rebuild.

## Validation log

All checks below were run locally on 2026-05-23 after the M7C implementation.

- `cargo fmt --check` — passed.
- `cargo test -p pack-mcp` — passed (`10 passed`).
- `scripts/real-test.sh` — passed with realistic pack + CLI + MCP media enrichment + viewer APIs + filter stress + open URL.
- `cargo clippy --all-targets -- -D warnings` — passed.
- `cargo test` — passed across the workspace:
  - `pack-cli`: 18 passed
  - `pack-core`: 49 passed
  - `pack-mcp`: 10 passed
  - `pack-server`: 28 passed
  - doctests: 0
- `cargo check -p pack-cli --features real-embed` — passed.

## Real-test MCP coverage added in this slice

`scripts/real-test.sh` now exercises a stdio MCP worker sequence against a realistic pack:

1. `media/list_pending` surfaces pending media sidecars.
2. `media/read_note` reads the `diagram-image` sidecar and asset path.
3. `media/write_enrichment` writes `MCP generated graph lattice caption` through the managed block.
4. `index/rebuild` refreshes the derived search index.
5. MCP `search` finds `diagram-image` by the newly written `lattice` term.

## Known gap

This slice still uses deterministic test payloads instead of calling a real vision/OCR/STT model. The storage and MCP worker contract is now ready for a provider-backed agent loop.
