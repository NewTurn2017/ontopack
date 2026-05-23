# Mac provider baseline audit — 2026-05-23

Goal: confirm OntoPack's API-first/local-fallback media provider defaults match the current optimized Mac setup while remaining overrideable for Windows/future installs.

## Local runtime evidence

Commands checked on the development Mac:

- `ollama version` -> `ollama version is 0.20.0`
- `ollama list` -> includes `gemma4:e4b` (`9.6 GB`) plus larger Gemma/Qwen coding models.
- `ollama show gemma4:e4b` -> architecture `gemma4`, parameters `8.0B`, context length `131072`, embedding length `2560`, quantization `Q4_K_M`, required runtime `0.20.0`, capabilities `completion`, `vision`, `audio`, `tools`, `thinking`.
- `tesseract --version` -> `tesseract 5.5.2`, NEON available.
- `ffmpeg -version` / `ffprobe -version` -> `8.1`, built with NEON/OpenCL/VideoToolbox via Homebrew.

## External reference check

- Ollama vision docs confirm vision models accept image+text prompts for description/classification/Q&A.
- Ollama Gemma 4 library page lists Gemma 4 as multimodal and includes `gemma4:e4b` with text/image/audio support and on-device/edge positioning.
- OpenAI model docs list GPT-5-class models as current and GPT-5 mini as a cost-efficient model.
- OpenAI GPT-5 mini page lists image input support and Responses API support.

## Decision

- Local default model: `gemma4:e4b`.
- API default model: `gpt-5-mini`.
- Keep all defaults overrideable via `OLLAMA_MODEL` and `OPENAI_MODEL`.
- Keep provider selection API-first when `OPENAI_API_KEY` exists, then local fallback when absent.

## Known gaps

- Live smoke: a generated valid PNG passed through `scripts/providers/local_media_worker.py` with `OLLAMA_MODEL=gemma4:e4b`, returned provider `local-media-worker`, model `gemma4:e4b`, non-empty caption, and no visible thinking/ANSI control output after `--hidethinking`/`--think=false`/cleanup.
- This audit validates installed versions and provider defaults. It does not benchmark production-scale caption latency/quality.
- Windows setup remains provider-contract compatible but was not exercised on Windows hardware.
