#!/usr/bin/env python3
"""OpenAI vision enrichment provider for `pack enrich-pending`.

Requires:
  OPENAI_API_KEY=...
Optional:
  OPENAI_MODEL=gpt-4.1-mini   # override with any vision-capable Responses model

stdin:  OntoPack provider payload JSON
stdout: EnrichmentPatch JSON
"""
import base64
import json
import mimetypes
import os
import re
import sys
import urllib.error
import urllib.request

API_URL = "https://api.openai.com/v1/responses"


def fail(message: str, code: int = 2) -> None:
    print(message, file=sys.stderr)
    raise SystemExit(code)


def data_url(path: str) -> str:
    mime, _ = mimetypes.guess_type(path)
    if not mime or not mime.startswith("image/"):
        fail(
            f"openai_vision_worker supports image assets only; got {path!r} with mime {mime!r}. "
            "Use an OCR/STT/video-specific worker for this asset."
        )
    with open(path, "rb") as f:
        encoded = base64.b64encode(f.read()).decode("ascii")
    return f"data:{mime};base64,{encoded}"


def extract_output_text(response: dict) -> str:
    if isinstance(response.get("output_text"), str):
        return response["output_text"]
    chunks = []
    for item in response.get("output", []) or []:
        for content in item.get("content", []) or []:
            if content.get("type") == "output_text" and isinstance(content.get("text"), str):
                chunks.append(content["text"])
    return "\n".join(chunks)


def parse_patch(text: str) -> dict:
    raw = text.strip()
    fence = re.search(r"```(?:json)?\s*(.*?)```", raw, re.DOTALL)
    if fence:
        raw = fence.group(1).strip()
    start = raw.find("{")
    end = raw.rfind("}")
    if start >= 0 and end >= start:
        raw = raw[start : end + 1]
    patch = json.loads(raw)
    allowed = {
        "caption",
        "tags",
        "ocr",
        "transcript",
        "summary",
        "keyframes",
        "provider",
        "model",
        "generated_at",
    }
    return {k: v for k, v in patch.items() if k in allowed and v not in (None, "")}


def main() -> None:
    api_key = os.environ.get("OPENAI_API_KEY")
    if not api_key:
        fail("OPENAI_API_KEY is required for openai_vision_worker.py; pack was not modified.")
    model = os.environ.get("OPENAI_MODEL", "gpt-4.1-mini")
    payload = json.load(sys.stdin)
    asset_abs_path = payload.get("asset_abs_path")
    if not asset_abs_path:
        fail("provider payload is missing asset_abs_path")

    prompt = {
        "note_id": payload.get("note_id"),
        "title": payload.get("title"),
        "note_type": payload.get("note_type"),
        "existing_sidecar_body": payload.get("body"),
        "task": (
            "Describe the image for a local personal knowledge ontology pack. "
            "Return ONLY compact JSON with fields: caption (string), tags (array of short strings), "
            "summary (string), ocr (string if visible text exists), provider, model. "
            "Do not include Markdown fences or prose outside JSON."
        ),
    }
    request_body = {
        "model": model,
        "input": [
            {
                "role": "user",
                "content": [
                    {"type": "input_text", "text": json.dumps(prompt, ensure_ascii=False)},
                    {"type": "input_image", "image_url": data_url(asset_abs_path), "detail": "auto"},
                ],
            }
        ],
    }
    req = urllib.request.Request(
        API_URL,
        data=json.dumps(request_body).encode("utf-8"),
        headers={
            "Authorization": f"Bearer {api_key}",
            "Content-Type": "application/json",
        },
        method="POST",
    )
    try:
        with urllib.request.urlopen(req, timeout=120) as resp:
            response = json.loads(resp.read().decode("utf-8"))
    except urllib.error.HTTPError as e:
        body = e.read().decode("utf-8", errors="replace")
        fail(f"OpenAI Responses API request failed: HTTP {e.code}: {body}", 1)

    text = extract_output_text(response)
    if not text:
        fail(f"OpenAI response did not include output_text: {json.dumps(response)[:1000]}", 1)
    patch = parse_patch(text)
    patch.setdefault("provider", "openai")
    patch.setdefault("model", model)
    print(json.dumps(patch, ensure_ascii=False))


if __name__ == "__main__":
    main()
