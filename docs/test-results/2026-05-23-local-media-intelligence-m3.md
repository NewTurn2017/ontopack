# M3 local media intelligence provider slice — 2026-05-23

Goal: move the post-MVP media path beyond static metadata by making video/audio provider output search-oriented transcript and keyframe fields through the existing `EnrichmentPatch` contract.

## What changed

- `scripts/providers/local_media_worker.py` now keeps the Mac-local, no-cloud behavior but expands video/audio handling:
  - ffprobe metadata still populates `summary`.
  - video files emit deterministic keyframe timestamp candidates in the existing `keyframes` field.
  - whisper.cpp transcription runs when `WHISPER_MODEL` or `WHISPER_CPP_MODEL` is configured and the media has an audio stream.
  - ffmpeg converts audio to temporary 16 kHz mono WAV for whisper-cli, then deletes the temp directory.
- The worker remains resilient: missing whisper model/tool does not fail metadata/keyframe enrichment; it simply skips transcript generation.
- Docs now describe `WHISPER_MODEL`, `WHISPER_LANG`, and the current local Mac tool baseline.

## Tests added/updated

- `scripts/providers/local_media_worker_test.py` now asserts:
  - video metadata output includes keyframe candidates.
  - when `WHISPER_MODEL` is configured, the worker calls ffmpeg + whisper-cli and returns `transcript`, `keyframes`, `whisper-cpp` tag, and `local-tools+whisper` model label.

## Validation

Run in this slice:

- `python3 -m py_compile scripts/providers/*.py`
- `python3 scripts/providers/local_media_worker_test.py`
- `python3 scripts/providers/auto_media_worker_test.py`
- `python3 scripts/providers/openai_vision_worker_test.py`
- `cargo fmt --check`
- `cargo test -p pack-cli enrich_pending_runs_provider_command_and_rebuilds_search`
- `scripts/real-test.sh`
- `cargo clippy --all-targets -- -D warnings`
- `cargo test`
- `cargo check -p pack-cli --features real-embed`
- `git diff --check`

## Known gaps

- Keyframes are timestamp candidates only; actual extracted thumbnail assets should be a follow-up once the pack has a durable derivative-asset location.
- Live whisper transcription was not run because no specific ggml whisper model path was selected for the project default.
- API providers for Gemini/OpenAI video/audio are separate future sibling workers, not part of this local-only worker slice.
