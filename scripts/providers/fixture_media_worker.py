#!/usr/bin/env python3
"""Deterministic OntoPack media enrichment provider for tests and demos.

Protocol:
- stdin: JSON payload from `pack enrich-pending`
- stdout: EnrichmentPatch JSON
"""
import json
import sys

payload = json.load(sys.stdin)
note_id = payload.get("note_id", "unknown")
note_type = payload.get("note_type", "asset")
asset_path = payload.get("asset_path") or "no-asset"

json.dump(
    {
        "caption": f"fixture-provider caption for {note_id} ({note_type}) at {asset_path}",
        "tags": ["fixture-provider", "auto-enriched", note_type],
        "summary": "Deterministic fixture provider output for OntoPack worker-loop validation.",
        "provider": "fixture-media-worker",
        "model": "deterministic-fixture",
    },
    sys.stdout,
    ensure_ascii=False,
)
sys.stdout.write("\n")
