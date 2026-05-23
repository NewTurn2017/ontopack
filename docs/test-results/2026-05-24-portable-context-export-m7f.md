# M7F portable context export validation — 2026-05-24

Goal: make stored OntoPack knowledge usable without the viewer by exporting citation-ready context for Claude/Codex, another app, or a lecture/demo bundle.

## Added CLI surface

`pack export` now supports:

- `--format markdown-bundle`: one readable Markdown bundle with `note:<id>` citations and asset paths.
- `--format jsonl`: one JSON object per note for pipelines and batch tools.
- `--format mcp-context`: one JSON document with `context_blocks` that external LLM/MCP consumers can use directly.
- `--output <file>`: writes the export to disk; without it, output goes to stdout.
- `--copy-assets <dir>`: copies referenced `assets/...` files into the destination while preserving their relative paths.

## Contract

Every format preserves the durable source references needed outside the UI:

- note id citation, e.g. `note:lecture-outline`
- note path under `notes/`
- asset path when present, e.g. `assets/evidence.png`
- copied asset file when `--copy-assets` is used, e.g. `<dir>/assets/evidence.png`
- tags/type/created/related metadata
- source body text

## Validation

- RED: `cargo test -p pack-cli export_` failed because `pack export` did not exist.
- GREEN: `cargo test -p pack-cli export_` passed after implementation.
- RED: `cargo test -p pack-cli export_can_copy_referenced_assets_for_portable_bundle` failed because `--copy-assets` did not exist.
- GREEN: `cargo test -p pack-cli export_` passed after adding asset copy support for frontmatter assets and derived `assets/...` references in note bodies.
- `scripts/real-test.sh` now checks all three export formats against a realistic pack.
- `scripts/real-test.sh` now checks `--copy-assets` copies `assets/evidence.png`.

## Known gaps

- `markdown-bundle` is a single concatenated Markdown file, not a zip/tar archive.
- Copy mode creates a portable directory tree, not a compressed archive.
