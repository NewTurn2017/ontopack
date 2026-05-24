# M6 orphan note detection validation

## Claim

OntoPack now exposes a read-only maintenance report for notes that are disconnected from the local knowledge graph.

## Scope

- `pack_core::Pack::orphan_notes()` scans source-of-truth markdown files and reports notes with no outgoing wiki links and no incoming wiki links from other existing notes.
- Missing link targets are intentionally treated as future gap-detection data, not as valid incoming/outgoing connectivity for the target.
- `pack orphans` prints a Korean human-readable report; `pack orphans --json` emits stable JSON for agents and scripts.
- `scripts/real-test.sh` seeds a deliberate `orphan-gap` note and asserts both text and JSON reports include it.

## Evidence

- `cargo test -p pack-core orphan_notes` — passed.
- `cargo test -p pack-cli orphans_reports_unlinked_notes` — passed.
- `scripts/real-test.sh` — passed with orphan report assertions on a realistic pack.
- `cargo test` — passed.
- `cargo clippy --all-targets -- -D warnings` — passed.
- `cargo check -p pack-cli --features real-embed` — passed.
- `git diff --check` — passed.

## Notes

This slice is read-only and does not rewrite notes. Broader M6 graph hygiene such as dangling-link reports, cluster/topic-map generation, and fuzzy duplicate detection remains future work.
