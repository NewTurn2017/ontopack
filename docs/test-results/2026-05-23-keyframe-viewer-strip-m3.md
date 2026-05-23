# M3 keyframe API + viewer strip validation — 2026-05-23

Goal: make locally extracted video keyframe assets visible to humans, not just written into sidecar Markdown.

## What changed

- `NoteDetail` and `GalleryItem` now expose parsed `keyframes[]` from the managed enrichment block.
- Each keyframe includes `time`, `text`, optional `asset`, and optional safe `/assets/...` `asset_url`.
- The embedded viewer renders a compact keyframe strip in gallery cards and selected note detail.
- In selected note detail, Enter/Space/click on a keyframe seeks the source video to that timestamp.
- Viewer tests now assert the keyframe rendering path is present.

## Validation

- `cargo fmt --check`
- `cargo test -p pack-server note_api_returns_enrichment_keyframe_assets`
- `cargo test -p pack-server viewer_assets_render_media_previews`
- `cargo test -p pack-server api_gallery_http_returns_asset_cards`
- `scripts/real-test.sh`
- `cargo clippy --all-targets -- -D warnings`
- `cargo test`
- `cargo check -p pack-cli --features real-embed`
- `git diff --check`

## Known gaps

- Keyframe seek is implemented only for the selected note detail panel, not gallery cards.
- The parser intentionally reads the existing Markdown managed block instead of introducing a second structured sidecar file.
