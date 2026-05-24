# M7F portable bundle directory validation — 2026-05-24

Goal: reduce portable handoff friction by making context and media move as one directory artifact instead of separate command outputs.

## Added CLI surface

`pack bundle <dir>` creates:

- `context.jsonl` — deterministic import/roundtrip format.
- `context.md` — human-readable Markdown bundle.
- `mcp-context.json` — agent/MCP-friendly context blocks.
- `bundle.json` — lightweight manifest with counts and relative filenames.
- `assets/...` — copied original/derived media using preserved asset paths.

`pack import <bundle-dir>` now detects a bundle directory and imports `context.jsonl` with the bundle directory as the asset root.

## Validation

- RED: `cargo test -p pack-cli bundle_directory_imports_as_one_portable_artifact` failed because `pack bundle` did not exist.
- GREEN: the same test passed after adding bundle creation and directory import detection.
- `scripts/real-test.sh` now creates a realistic bundle, checks bundle files/assets exist, and imports the bundle into a fresh pack.

## Known gaps

- Bundle output is a directory artifact, not a compressed zip/tar file.
- Bundle import rebuilds indexes after restore; derived SQLite files are intentionally not shipped.
