# M7D provider command enrichment worker validation

Date: 2026-05-23

## Goal

Make the media enrichment loop runnable as a real local worker workflow, not only as manual CLI calls or individual MCP tools. The worker command must let an external AI/provider process read a pending media sidecar payload, return generated metadata, and have OntoPack persist/search it safely.

## Acceptance criteria

- `pack enrich-pending --provider-command <cmd>` drains pending media sidecars through a JSON stdin/stdout provider command.
- Provider input includes note id, sidecar body/raw Markdown, tags/relations, relative asset path, and absolute local asset path.
- Provider output uses the existing `EnrichmentPatch` contract.
- The worker writes only the managed enrichment block and preserves human sidecar text.
- The worker refreshes `.pack/objects.jsonl` and rebuilds the search index by default.
- Generated provider text is searchable immediately after the command completes.

## TDD evidence

- RED: `cargo test -p pack-cli enrich_pending_runs_provider_command_and_rebuilds_search` failed because `enrich-pending` did not exist.
- GREEN: after implementing the command loop, the same test passed.

## Validation log

All checks below were run locally on 2026-05-23 after the M7D implementation.

- `cargo fmt --check` — passed.
- `cargo test -p pack-cli enrich_pending` — passed.
- `scripts/real-test.sh` — passed with realistic pack + CLI provider worker + MCP media enrichment + viewer APIs + filter stress + open URL.

## Real-test worker coverage added in this slice

`scripts/real-test.sh` now creates a deterministic executable provider command and verifies:

1. `pack enrich-pending --provider-command ... --limit 1` processes one pending media sidecar.
2. The command prints `processed=1` and an `indexed=` result.
3. The generated `vaultworker` caption is immediately searchable through CLI keyword search.

## Known gap

This slice wires a real local worker command contract but still uses a deterministic fixture provider in tests. The next provider-specific slice can call actual vision/OCR/STT models behind this command interface.
