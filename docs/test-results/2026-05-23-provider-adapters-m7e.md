# M7E provider adapter templates validation

Date: 2026-05-23

## Goal

Make the provider-worker loop usable without ad-hoc scripts by shipping documented adapter templates: one deterministic offline worker for tests/demos, and one OpenAI vision worker skeleton for real image caption/OCR/summary enrichment.

## Acceptance criteria

- A bundled fixture provider can be used directly with `pack enrich-pending`.
- The fixture provider writes searchable enrichment text through the existing worker loop.
- `scripts/real-test.sh` uses the bundled fixture provider instead of generating a throwaway provider.
- An OpenAI vision provider script exists and fails honestly when `OPENAI_API_KEY` is missing.
- Provider contract and runbook are documented.

## TDD evidence

- RED: `cargo test -p pack-cli bundled_fixture_provider_enriches_media` failed because `scripts/providers/fixture_media_worker.py` did not exist.
- GREEN: after adding the bundled provider script, the same test passed.

## Validation log

All checks below were run locally on 2026-05-23 after the M7E implementation.

- `python3 scripts/providers/fixture_media_worker.py` with sample JSON — returned valid EnrichmentPatch JSON.
- `env -u OPENAI_API_KEY python3 scripts/providers/openai_vision_worker.py` with sample JSON — failed honestly with setup guidance and no pack mutation path.
- `cargo fmt --check` — passed.
- `cargo test -p pack-cli bundled_fixture_provider_enriches_media` — passed.
- `scripts/real-test.sh` — passed with realistic pack + bundled provider worker + MCP media enrichment + viewer APIs + filter stress + open URL.

## Known gap

The OpenAI adapter is wired as a ready-to-run provider script, but no live OpenAI API call was executed in this validation because credentials were not used in this local test slice.
