# M6 duplicate note detection validation

Date: 2026-05-24

## Goal

Start the M6 knowledge-intelligence lane with a deterministic maintenance report that finds exact duplicate note bodies without touching source-of-truth files.

## Acceptance criteria

- `pack-core` groups notes with the same whitespace-normalized body.
- Blank bodies are ignored.
- Report output includes note id, title, type, source path, optional asset path, fingerprint, and group size.
- CLI exposes `pack duplicates` and `pack duplicates --json`.
- Real smoke covers the human-readable duplicate report on a realistic pack.

## Validation log

- `cargo test -p pack-core duplicate_notes` — passed.
- `cargo test -p pack-cli duplicates_reports_matching_note_bodies` — passed.
- `scripts/real-test.sh` — passed with duplicate report assertions on a realistic pack.
- `cargo test` — passed across workspace (`pack-cli` 35, `pack-core` 52, `pack-mcp` 10, `pack-server` 33).
- `cargo clippy --all-targets -- -D warnings` — passed.
- `cargo check -p pack-cli --features real-embed` — passed.
- `git diff --check` — passed.

## Known gaps

- This slice detects exact normalized-body duplicates only; near-duplicate/fuzzy similarity and topic-cluster analysis remain future M6 work.
- The command is read-only and does not merge, delete, or rewrite notes.
