# OntoPack provider workers

OntoPack keeps storage deterministic and lets external workers do model-specific media enrichment. A provider worker is any executable that follows this contract:

- stdin: JSON payload from `pack enrich-pending`
- stdout: `EnrichmentPatch` JSON with any of `caption`, `tags`, `ocr`, `transcript`, `summary`, `keyframes`, `provider`, `model`, `generated_at`
- stderr + non-zero exit: honest failure; OntoPack stops and leaves the current sidecar unchanged

Run one pending item through the recommended router:

```bash
pack enrich-pending --provider-command scripts/providers/auto_media_worker.py --limit 1
```

`auto_media_worker.py` prioritizes API mode when credentials exist, then falls back to local tools. Override behavior with `ONTOPACK_PROVIDER_MODE=api|local|auto`.

## macOS latest local setup

For the current Mac-first path, install local media tools with Homebrew:

```bash
brew install ollama tesseract ffmpeg whisper-cpp
ollama pull gemma4:e4b
```

Recommended local-only run:

```bash
ONTOPACK_PROVIDER_MODE=local \
  OLLAMA_MODEL=gemma4:e4b \
  TESSERACT_LANG=eng+kor \
  WHISPER_MODEL=/path/to/ggml-model.bin \
  WHISPER_LANG=auto \
  pack enrich-pending --provider-command /path/to/ontopack/scripts/providers/auto_media_worker.py --limit 1
```

Behavior by media type:

- image: Ollama vision caption when `ollama` is installed; Tesseract OCR when `tesseract` is installed.
- video/audio: ffprobe metadata through `ffmpeg`; video keyframes are extracted into `assets/.derived/<note-id>/keyframe-####.jpg` when ffmpeg can decode the file, and their paths are emitted in `keyframes[].asset`; optional whisper.cpp transcription runs when `WHISPER_MODEL` points at an installed ggml model.

Verified Mac baseline on the current development machine:

- Ollama `0.20.0` with `gemma4:e4b` installed. `ollama show gemma4:e4b` reports a 128K context window plus vision/audio/tools/thinking capabilities, so it is the default local caption model for the Mac path. Override with `OLLAMA_MODEL` when a stronger local model is installed.
- Tesseract `5.5.2` with NEON support is sufficient for local OCR.
- FFmpeg/ffprobe `8.1` with NEON/OpenCL/VideoToolbox is sufficient for local metadata extraction and audio preparation.
- whisper.cpp `1.8.4` is installed; transcript generation is enabled only when `WHISPER_MODEL` is set so normal installs do not fail on missing STT models.

Reference docs checked for the default choices:

- Ollama Vision docs: <https://docs.ollama.com/capabilities/vision>
- Ollama Gemma 4 library page: <https://ollama.com/library/gemma4>
- OpenAI model list: <https://platform.openai.com/docs/models>

## API priority option

When `OPENAI_API_KEY` is present and mode is `auto`, the router uses the API worker before local tools:

```bash
export OPENAI_API_KEY=...
export OPENAI_MODEL=gpt-5.4-mini
pack enrich-pending --provider-command /path/to/ontopack/scripts/providers/auto_media_worker.py --limit 1
```

`gpt-5.4-mini` is the user-selected default API model for the image-caption/OCR patch task. If the deployed OpenAI account/model list changes, set `OPENAI_MODEL` to any available vision-capable Responses model without changing OntoPack storage.

Gemini 3.5 Flash can be added as a parallel API worker later using the same stdin/stdout provider contract; do not overload the OpenAI worker with non-OpenAI model names.

Force local even when an API key exists:

```bash
ONTOPACK_PROVIDER_MODE=local pack enrich-pending --provider-command /path/to/ontopack/scripts/providers/auto_media_worker.py --limit 1
```

## Windows future path

The provider contract is OS-neutral: an executable reads JSON from stdin and writes `EnrichmentPatch` JSON to stdout. For Windows, keep the same `pack enrich-pending --provider-command ...` command shape and install equivalents with winget/Chocolatey or native installers:

- Ollama for Windows for local vision models.
- Tesseract OCR Windows builds or package-manager install.
- FFmpeg Windows build in PATH for video/audio metadata/extraction.
- whisper.cpp or another local STT executable in PATH for transcript workers; expose its model path through `WHISPER_MODEL`.

No OntoPack storage format should change for Windows; only provider executable discovery/setup should vary.

## Bundled providers

### `scripts/providers/auto_media_worker.py`

Recommended entrypoint. Routes to API first when `OPENAI_API_KEY` is set, otherwise local. Override worker commands with:

- `ONTOPACK_API_WORKER`
- `ONTOPACK_LOCAL_WORKER`

### `scripts/providers/local_media_worker.py`

Local-only worker for macOS-first setup. Uses Ollama/Tesseract/ffprobe/ffmpeg/whisper.cpp when available and never calls a cloud API. Images can get captions/OCR; videos/audio get metadata, derived keyframe JPEGs under `assets/.derived/`, and optional transcript text when `WHISPER_MODEL` is configured.

### `scripts/providers/fixture_media_worker.py`

Deterministic offline provider for tests, demos, and contract debugging. It does not inspect pixels; it proves the JSON worker loop and search indexing path.

```bash
pack enrich-pending --provider-command /path/to/ontopack/scripts/providers/fixture_media_worker.py --limit 10
```

### `scripts/providers/openai_vision_worker.py`

Image caption/OCR/summary provider using the OpenAI Responses API with base64 image input. It requires a vision-capable model and an API key.

```bash
export OPENAI_API_KEY=...
export OPENAI_MODEL=gpt-5.4-mini   # optional override
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
  "transcript": "[00:00:00] spoken words if any",
  "summary": "Short search-oriented summary.",
  "keyframes": [
    { "time": "00:00:01", "text": "slide title", "asset": "assets/.derived/demo/keyframe-0000.jpg" }
  ],
  "provider": "my-worker",
  "model": "my-model"
}
```
