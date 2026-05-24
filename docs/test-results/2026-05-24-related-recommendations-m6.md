# M6 related note recommendation validation

## Claim

OntoPack now produces a read-only proactive report that recommends unlinked related notes using explicit shared tags.

## Scope

- `pack_core::Pack::related_suggestions(note_id, k)` ranks candidate notes by shared tag count.
- Existing outgoing or incoming wiki links are skipped so the report focuses on missing relationships.
- `pack recommend [note-id] -k N` prints a Korean human-readable report; `pack recommend --json` emits stable JSON for agents and scripts.
- `scripts/real-test.sh` seeds `recommend-a` and `recommend-b` with shared tags and asserts both text and JSON reports recommend the candidate.

## Evidence

- `cargo test -p pack-core related_suggestions` — passed.
- `cargo test -p pack-cli recommend_reports_unlinked_notes_with_shared_tags` — passed.
- `scripts/real-test.sh` — passed with related recommendation assertions on a realistic pack.
- `cargo test` — passed.
- `cargo clippy --all-targets -- -D warnings` — passed.
- `cargo check -p pack-cli --features real-embed` — passed.
- `git diff --check` — passed.

## Notes

This slice intentionally uses explicit tag overlap. Embedding-based similarity recommendations and automatic link-writing remain future, opt-in additions.
