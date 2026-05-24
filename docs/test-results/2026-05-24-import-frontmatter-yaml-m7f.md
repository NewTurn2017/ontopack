# M7F import frontmatter readability validation — 2026-05-24

Goal: make restored notes readable after `pack import` by writing YAML frontmatter instead of compact JSON-object frontmatter.

## Change

`pack import <context.jsonl>` and `pack import <bundle-dir>` now serialize restored note frontmatter with `serde_yaml`, producing human-readable fields such as `type: image`, `title: ...`, `tags:`, and `asset: ...`.

The JSONL import contract and bundle directory contract remain unchanged; only the restored note source-of-truth Markdown format became easier for humans to inspect and edit.

## Validation

- RED: `cargo test -p pack-cli import_jsonl_roundtrips_exported_context_and_assets -- --exact` failed when the restored note still started with compact JSON object frontmatter.
- GREEN: the same test passed after switching import writes to YAML frontmatter and asserting readable YAML fields.

## Known gaps

- YAML key ordering follows map serialization order, so tests assert field readability rather than a hand-authored key order.
