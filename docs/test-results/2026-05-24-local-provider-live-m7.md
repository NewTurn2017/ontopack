# M7 local media provider live validation

## Claim

The local provider path remains live with installed ffmpeg tooling and can enrich a real generated video asset into searchable media intelligence.

## Scope

- `scripts/media-intelligence-test.sh` generated a synthetic MP4 using ffmpeg.
- `pack add` created a video sidecar.
- `pack enrich-pending --provider-command scripts/providers/auto_media_worker.py` ran in local provider mode.
- The local worker produced metadata and derived keyframe JPEG assets under `assets/.derived/...`.
- CLI search found generated media-intelligence text.
- Viewer APIs exposed keyframe asset URLs and the derived asset route served the JPEG.

## Evidence

- `scripts/media-intelligence-test.sh` — passed.
- Provider doctor before this run reported local optional tools available: ffmpeg, ffprobe, tesseract, ollama, and whisper-cli.

## Notes

Real Whisper transcription was skipped because no `WHISPER_MODEL`/`WHISPER_CPP_MODEL` path was configured. OpenAI API provider live calls were not run in this automatic slice to avoid spending external API quota without an explicit API-call instruction.
