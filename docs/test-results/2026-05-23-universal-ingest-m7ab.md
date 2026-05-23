# M7A/B universal ingest status + safe enrichment validation

Date: 2026-05-23

## Goal

Move the next OntoPack phase away from UI polish and toward reliable local-first storage/retrieval: every stored object should be introspectable, and agent-generated enrichment should be written safely into sidecar notes without overwriting human content.

## Acceptance criteria

- `pack status` summarizes notes/assets/index/enrichment state.
- `pack status` refreshes `.pack/objects.jsonl` from source-of-truth notes/assets.
- `pack list --pending-enrichment` lists imported media sidecars that need AI work.
- `pack enrich-note` writes caption/tags/transcript into a managed Markdown block.
- Human-authored text outside the managed enrichment block is preserved.
- Rebuilding the index makes generated enrichment searchable.

## Validation log

All checks below were run locally on 2026-05-23 after the M7A/B implementation.

- `cargo fmt --check` — passed.
- `cargo test -p pack-core enrichment` — passed.
- `cargo test -p pack-cli enrichment` — passed.
- `cargo test -p pack-cli enrich_note` — passed.
- `cargo test` — passed across the workspace:
  - `pack-cli`: 18 passed
  - `pack-core`: 49 passed
  - `pack-mcp`: 9 passed
  - `pack-server`: 28 passed
  - doctests: 0
- `cargo check -p pack-cli --features real-embed` — passed.
- `cargo clippy --all-targets -- -D warnings` — passed.
- `scripts/real-test.sh` — passed with realistic pack + CLI + MCP + viewer APIs + filter stress + open URL.

## Real-test coverage added in this slice

`scripts/real-test.sh` now verifies the new storage/enrichment loop:

1. `pack status` reports enrichment counters and refreshes the object manifest.
2. `pack list --pending-enrichment` surfaces the imported demo video sidecar.
3. `pack enrich-note demo-video ...` writes deterministic caption/tag metadata through the safe managed block.
4. Rebuilding the index makes the generated caption searchable (`cockpit` returns `demo-video#0000`).

## Known gap

This slice intentionally does not call real Claude/Codex/MCP media-enrichment tools or production vision/STT providers. It creates the durable contract those workers should use next: list pending media, read the sidecar, write managed enrichment, rebuild/search.
