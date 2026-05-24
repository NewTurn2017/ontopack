# M7F Windows portability notes — 2026-05-24

Goal: make the current platform boundary explicit after bundle/import/archive work.

## Updated docs

- `README.md` now calls out that bundle/import storage paths are OS-neutral but current runtime proof is macOS-based.
- `docs/real-test.md` documents forward-slash pack-relative paths, pure-Rust `.tar.gz` archive support, and Windows verification targets.
- `docs/providers.md` labels Windows provider support as documented but not live-verified, and lists PATH/executable expectations for ffmpeg/ffprobe, tesseract, ollama, whisper, and Python provider wrappers.

## Current support statement

- Storage format: intended OS-neutral.
- Bundle directory and `.tar.gz` archive: pure Rust implementation, no system tar/zip dependency.
- Provider runtime: macOS verified; Windows setup remains unverified until a Windows runner or manual proof is added.

## Validation

- Documentation-only change; checked with `git diff --check`.
