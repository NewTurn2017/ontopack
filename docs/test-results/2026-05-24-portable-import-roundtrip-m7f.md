# M7F portable import roundtrip validation — 2026-05-24

Goal: prove an OntoPack portable export can be restored into another pack, not only consumed as read-only context.

## Added CLI surface

`pack import <context.jsonl> --format jsonl [--asset-root <dir>] [--overwrite]`

- Reads JSONL produced by `pack export --format jsonl`.
- Recreates Markdown notes under `notes/<note-id>.md`.
- Preserves type/title/tags/created/related/body metadata.
- Copies referenced `assets/...` files from `--asset-root` into the destination pack.
- Refuses to overwrite existing notes/assets unless `--overwrite` is set.

## Validation

- RED: `cargo test -p pack-cli import_jsonl_roundtrips_exported_context_and_assets` failed because `pack import` did not exist.
- GREEN: the same test passed after implementation.
- `scripts/real-test.sh` now exports a realistic pack, imports the JSONL + copied assets into a fresh pack, builds the imported pack, and verifies keyword search works.

## Known gaps

- Import currently targets the project-owned JSONL format only; Markdown bundle and MCP context remain export/consumer formats.
- Import restores referenced assets and note metadata, not derived SQLite indexes; indexes are intentionally rebuilt in the destination pack.
