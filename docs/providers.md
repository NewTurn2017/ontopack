# OntoPack provider workers

OntoPack keeps storage deterministic and lets external workers do model-specific media enrichment. A provider worker is any executable that follows this contract:

- stdin: JSON payload from `pack enrich-pending`
- stdout: `EnrichmentPatch` JSON with any of `caption`, `tags`, `ocr`, `transcript`, `summary`, `keyframes`, `provider`, `model`, `generated_at`
- stderr + non-zero exit: honest failure; OntoPack stops and leaves the current sidecar unchanged

Run one pending item through a provider:

```bash
pack enrich-pending --provider-command scripts/providers/fixture_media_worker.py --limit 1
```

## Bundled providers

### `scripts/providers/fixture_media_worker.py`

Deterministic offline provider for tests, demos, and contract debugging. It does not inspect pixels; it proves the JSON worker loop and search indexing path.

```bash
pack enrich-pending --provider-command /path/to/ontopack/scripts/providers/fixture_media_worker.py --limit 10
```

### `scripts/providers/openai_vision_worker.py`

Image caption/OCR/summary provider using the OpenAI Responses API with base64 image input. It requires a vision-capable model and an API key.

```bash
export OPENAI_API_KEY=...
export OPENAI_MODEL=gpt-4.1-mini   # optional override
pack enrich-pending --provider-command /path/to/ontopack/scripts/providers/openai_vision_worker.py --limit 1
```

If `OPENAI_API_KEY` is missing or the asset is not an image, the worker exits non-zero with setup guidance and OntoPack does not write enrichment for that item.

## Provider input shape

Typical fields passed to the worker:

```json
{
  "note_id": "board",
  "title": "board",
  "note_type": "image",
  "tags": [],
  "created": null,
  "related": [],
  "note_path": "/pack/notes/board.md",
  "asset_path": "assets/board.png",
  "asset_abs_path": "/pack/assets/board.png",
  "body": "캡션을 적어주세요(검색 대상).\n",
  "raw": "---\ntype: image\n...",
  "content_hash": "..."
}
```

## Provider output shape

```json
{
  "caption": "A whiteboard showing an ontology graph.",
  "tags": ["whiteboard", "ontology", "graph"],
  "ocr": "visible text if any",
  "summary": "Short search-oriented summary.",
  "provider": "my-worker",
  "model": "my-model"
}
```
