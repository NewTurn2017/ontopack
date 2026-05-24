# M7 provider doctor validation

## Claim

OntoPack now has a provider toolchain diagnostic that separates deterministic fixture readiness from optional local/API media provider readiness.

## Scope

- `scripts/provider-doctor.py` reports PATH availability and version details for optional media tools: python3, ffmpeg, ffprobe, tesseract, ollama, and whisper-cli.
- It verifies bundled provider worker files are present.
- It reports relevant environment variables without exposing secret values (`OPENAI_API_KEY` is boolean only).
- `--require fixture|local|api` can fail fast for the provider class a workflow expects.
- `scripts/real-test.sh` requires fixture readiness in both text and JSON modes.

## Evidence

- `scripts/provider-doctor.py --require fixture` — covered by `scripts/real-test.sh`.
- `scripts/provider-doctor.py --json --require fixture` — covered by `scripts/real-test.sh`.
- `scripts/real-test.sh` — passed.
- `cargo test` — passed.
- `cargo clippy --all-targets -- -D warnings` — passed.
- `cargo check -p pack-cli --features real-embed` — passed.
- `git diff --check` — passed.

## Notes

Local/API provider readiness remains environment-dependent. This diagnostic avoids pretending optional tools are installed while still giving users and installers a precise checklist.
