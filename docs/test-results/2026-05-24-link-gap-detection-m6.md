# M6 link gap detection validation

## Claim

OntoPack now reports dangling wiki links as a separate read-only M6 graph hygiene check.

## Scope

- `pack_core::Pack::link_gaps()` scans source-of-truth markdown notes and compares every related/wiki-link target against existing note ids.
- Existing links are ignored; missing targets are reported once per source note relation.
- `pack gaps` prints a Korean human-readable report; `pack gaps --json` emits stable JSON for agent workflows.
- `scripts/real-test.sh` seeds a deliberate `dangling-gap -> missing-hygiene-target` relation and asserts both text and JSON reports include it.

## Evidence

- `cargo test -p pack-core link_gaps` — passed.
- `cargo test -p pack-cli gaps_reports_missing_wikilink_targets` — passed.
- `scripts/real-test.sh` — passed with dangling-link report assertions on a realistic pack.
- `cargo test` — passed.
- `cargo clippy --all-targets -- -D warnings` — passed.
- `cargo check -p pack-cli --features real-embed` — passed.
- `git diff --check` — passed.

## Notes

This keeps gap detection separate from orphan detection: a note that links to a missing target is not an orphan, and the missing target is not treated as a real node until a note file exists.
