# M3 media intelligence e2e validation — 2026-05-23

Goal: lock the real local media intelligence path with a replayable script, not only unit tests.

## Added script

`scripts/media-intelligence-test.sh` performs a full local-only vertical slice:

1. Builds the debug `pack` binary.
2. Creates a temporary pack.
3. Generates a real 3-second mp4 with ffmpeg test video + sine audio.
4. Adds the mp4 as a video sidecar.
5. Runs `pack enrich-pending` through `scripts/providers/auto_media_worker.py` forced to local mode.
6. Verifies the sidecar has a `## Keyframes` block.
7. Verifies `assets/.derived/source-media-intel/keyframe-0000.jpg` exists.
8. Verifies CLI keyword search finds generated keyframe text.
9. Verifies `/api/notes/source-media-intel` exposes `keyframes[].asset_url`.
10. Verifies `/api/gallery` includes keyframes for the video card.
11. Optionally verifies real whisper transcription when `RUN_REAL_WHISPER=1` and `WHISPER_MODEL` or `WHISPER_CPP_MODEL` is set.
12. In the real Whisper path, generates a second mp4 using a real speech sample, verifies the `## Transcript` block contains `country`, and verifies keyword search can retrieve the transcript text.

## Validation

- `WHISPER_MODEL="$HOME/.cache/ontopack/whisper/ggml-tiny.en.bin" RUN_REAL_WHISPER=1 scripts/media-intelligence-test.sh`
- `scripts/media-intelligence-test.sh`

## Known gaps

- Default path still skips Whisper model runtime so normal CI/developer smoke does not download or require a ggml model.
- Windows Whisper model setup is not validated yet.
- The keyframe asset route is smoke-checked for non-empty output; binary response bytes are not decoded by the shell test.
