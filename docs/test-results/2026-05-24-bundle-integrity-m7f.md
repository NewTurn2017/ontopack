# M7F portable bundle integrity validation — 2026-05-24

Goal: make `pack import <bundle-dir>` fail clearly and before partial restore when a portable bundle is incomplete, unsafe, or would overwrite source-of-truth files.

## Added safeguards

- `pack import <bundle-dir>` now requires and parses `bundle.json` before reading `context.jsonl`.
- Manifest fields are validated for bundle type, version, safe relative paths, and note/asset counts, including explicit mismatch failures.
- Bundle imports preflight all notes and assets before writing anything, so missing assets or overwrite conflicts do not leave partially restored notes.
- Existing notes/assets are refused by default and restored only with `--overwrite`.
- JSONL note IDs and bundle manifest paths reject path traversal.

## Validation

- RED: new CLI tests failed against the previous implementation because directory import ignored `bundle.json`, wrote notes before missing-asset failure, and allowed manifest path tampering to be ignored.
- GREEN: after adding manifest validation and import preflight, the focused tests passed:
  - `cargo test -p pack-cli bundle_import_ -- --nocapture`
  - `cargo test -p pack-cli import_refuses_existing_note_or_asset_unless_overwrite_is_set -- --exact`
  - `cargo test -p pack-cli import_rejects_context_and_manifest_path_traversal -- --exact`

## Known gaps

- Bundle output is still a directory artifact, not a compressed archive.
- Import frontmatter still uses compact JSON object YAML; a readability pass remains separate.
