# OntoPack M5B indexed viewer API QA — 2026-05-23

## Scope

Verify the first M5B performance slice: viewer APIs prefer the SQLite derived index instead of reparsing markdown files on normal built packs.

## Implementation evidence

- Added `Index::all_notes()`.
- Added `Pack::indexed_notes_or_scan()`.
- Changed server API handlers for note detail, related, timeline, graph, facets, and gallery to use indexed rows when `.pack/index.db` exists.
- Kept source markdown scanning as fallback for packs that have not run `pack build` yet.

## Regression tests

New tests:

- `note_api_reads_from_index_after_source_file_is_removed`
- `gallery_api_reads_from_index_after_source_file_is_removed`

These tests build the index, remove the source markdown note, and verify the API still returns the indexed note/media metadata.

## Validation

- `cargo test -q`: passed during implementation.
- Full gate should include `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `scripts/real-test.sh`, and `git diff --check` before commit.

## Remaining performance work

- Add endpoint-specific SQL reads instead of materializing all indexed notes per request.
- Add `/api/dashboard` batching to reduce viewer startup fan-out.
- Add timing metrics and a large synthetic pack benchmark.
