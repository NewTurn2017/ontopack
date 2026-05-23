# M7F portable context export validation — 2026-05-24

Goal: make stored OntoPack knowledge usable without the viewer by exporting citation-ready context for Claude/Codex, another app, or a lecture/demo bundle.

## Added CLI surface

`pack export` now supports:

- `--format markdown-bundle`: one readable Markdown bundle with `note:<id>` citations and asset paths.
- `--format jsonl`: one JSON object per note for pipelines and batch tools.
- `--format mcp-context`: one JSON document with `context_blocks` that external LLM/MCP consumers can use directly.
- `--output <file>`: writes the export to disk; without it, output goes to stdout.

## Contract

Every format preserves the durable source references needed outside the UI:

- note id citation, e.g. `note:lecture-outline`
- note path under `notes/`
- asset path when present, e.g. `assets/evidence.png`
- tags/type/created/related metadata
- source body text

## Validation

- RED: `cargo test -p pack-cli export_` failed because `pack export` did not exist.
- GREEN: `cargo test -p pack-cli export_` passed after implementation.
- `scripts/real-test.sh` now checks all three export formats against a realistic pack.

## Known gaps

- `markdown-bundle` is a single concatenated Markdown file, not a zip/tar archive.
- Exports include source asset paths, not copied asset binaries; packaging/copy mode can be a later distribution slice.
