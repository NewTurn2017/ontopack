# M7F provider priority router validation

Date: 2026-05-23

## Goal

Configure OntoPack enrichment providers for the intended distribution model: macOS local-first setup now, Windows-compatible provider contract later, and API providers taking priority when credentials are explicitly available.

## Acceptance criteria

- `auto_media_worker.py` is the recommended provider entrypoint.
- In `auto` mode, an API worker is selected first when `OPENAI_API_KEY` exists.
- Without API credentials, the router falls back to a local worker.
- If neither API nor local worker is available, the router fails honestly and leaves pack mutation to the caller.
- macOS latest setup is documented with Ollama/Tesseract/FFmpeg.
- Windows remains a provider-command setup variation, not a storage-format change.
- `scripts/real-test.sh` exercises the router path deterministically.

## Validation log

All checks below were run locally on 2026-05-23 after the M7F implementation.

- `python3 -m py_compile scripts/providers/*.py` — passed.
- `python3 scripts/providers/auto_media_worker_test.py` — passed:
  - API worker preferred when `OPENAI_API_KEY` exists.
  - local worker used when API key is absent.
  - honest failure when no provider is available.
- `cargo fmt --check` — passed.
- `cargo test -p pack-cli bundled_fixture_provider_enriches_media` — passed.
- `scripts/real-test.sh` — passed with the provider router using the bundled fixture as deterministic local worker.

## Known gap

Live local Ollama/Tesseract/FFmpeg execution and Windows installer verification were not run in this slice. The router and docs are ready for those environment-specific checks.
