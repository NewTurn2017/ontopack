# M5 media timeline citation validation

Date: 2026-05-24

## Goal

Make video/audio transcript search results citation-ready by surfacing the timestamp attached to the matching snippet.

## Acceptance criteria

- Search cards keep existing asset metadata.
- Video/audio hits whose snippet contains `[MM:SS]` or `[HH:MM:SS]` expose `media_citation` with normalized time, seconds, and a media-fragment asset URL.
- Image/file hits never receive a false media timeline citation.
- Viewer cards render the timestamp chip and seek the selected detail video/audio when opened from a timestamped result.
- Real smoke covers the API contract against a realistic enriched video sidecar.

## Validation log

- `cargo test -p pack-server search_api_returns_timeline_media_citation_for_transcript_hits` — passed.
- `cargo test -p pack-server search_api_does_not_add_media_citation_to_images` — passed.
- `cargo test -p pack-server viewer_assets_render_media_previews` — passed.
- `cargo test -p pack-server` — passed (`33 passed`).
- `scripts/real-test.sh` — passed with realistic enriched video sidecar media citation assertion.
- `cargo test` — passed across workspace (`pack-cli` 34, `pack-core` 50, `pack-mcp` 10, `pack-server` 33).
- `cargo clippy --all-targets -- -D warnings` — passed.
- `cargo check -p pack-cli --features real-embed` — passed.
- `git diff --check` — passed.

## Known gaps

- This slice detects timestamp text already present in transcript/enrichment; it does not generate transcripts or cut thumbnails by itself.
- Media fragment seeking is browser/player dependent; the API remains useful to external agents even when a browser cannot seek a fake or unsupported video fixture.
