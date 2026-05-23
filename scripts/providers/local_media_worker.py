#!/usr/bin/env python3
"""Local-only OntoPack media worker for macOS-first setup.

Uses installed local tools when available:
- Ollama vision model for image captions (`OLLAMA_MODEL`, default `gemma4:e4b`)
- Tesseract for OCR
- ffprobe for video/audio metadata

It never calls a cloud API.
"""
import json
import mimetypes
import os
import re
import shutil
import subprocess
import sys
from pathlib import Path
from typing import Optional

DEFAULT_OLLAMA_MODEL = "gemma4:e4b"
ANSI_ESCAPE_RE = re.compile(r"\x1b\[[0-?]*[ -/]*[@-~]")


def fail(message: str, code: int = 2) -> None:
    print(message, file=sys.stderr)
    raise SystemExit(code)


def run(command: list[str], stdin: Optional[str] = None, timeout: int = 120) -> subprocess.CompletedProcess:
    return subprocess.run(command, input=stdin, text=True, capture_output=True, timeout=timeout, check=False)


def selected_ollama_model() -> str:
    return os.environ.get("OLLAMA_MODEL") or DEFAULT_OLLAMA_MODEL


def clean_ollama_output(text: str) -> str:
    return ANSI_ESCAPE_RE.sub("", text).strip()


def ollama_caption(path: str, payload: dict) -> Optional[str]:
    if not shutil.which("ollama"):
        return None
    model = selected_ollama_model()
    prompt = (
        "Describe this image for a private local knowledge ontology pack. "
        "Be concise and include visible text, objects, UI, diagrams, and why it may be useful for search."
    )
    proc = run(
        ["ollama", "run", "--nowordwrap", "--hidethinking", "--think=false", model, path, prompt],
        timeout=int(os.environ.get("OLLAMA_TIMEOUT", "180")),
    )
    if proc.returncode != 0:
        fail(f"ollama vision failed: {proc.stderr.strip() or proc.stdout.strip()}", proc.returncode)
    return clean_ollama_output(proc.stdout)


def tesseract_ocr(path: str) -> Optional[str]:
    if not shutil.which("tesseract"):
        return None
    langs = os.environ.get("TESSERACT_LANG", "eng+kor")
    proc = run(["tesseract", path, "stdout", "-l", langs], timeout=120)
    if proc.returncode != 0:
        return None
    text = proc.stdout.strip()
    return text or None


def ffprobe_summary(path: str) -> Optional[str]:
    if not shutil.which("ffprobe"):
        return None
    proc = run(
        [
            "ffprobe",
            "-v",
            "error",
            "-show_entries",
            "format=duration,format_name:stream=codec_type,codec_name,width,height",
            "-of",
            "json",
            path,
        ],
        timeout=60,
    )
    if proc.returncode != 0:
        return None
    try:
        data = json.loads(proc.stdout)
    except json.JSONDecodeError:
        return None
    return json.dumps(data, ensure_ascii=False)


def main() -> None:
    payload = json.load(sys.stdin)
    asset = payload.get("asset_abs_path")
    if not asset:
        fail("local_media_worker requires asset_abs_path")
    mime, _ = mimetypes.guess_type(asset)
    note_type = payload.get("note_type") or "asset"

    caption = None
    ocr = None
    summary = None
    tags = ["local", note_type]

    if mime and mime.startswith("image/"):
        caption = ollama_caption(asset, payload)
        ocr = tesseract_ocr(asset)
        if caption:
            tags.append("ollama-vision")
        if ocr:
            tags.append("ocr")
    elif mime and (mime.startswith("video/") or mime.startswith("audio/")):
        metadata = ffprobe_summary(asset)
        if metadata:
            summary = f"Local media metadata from ffprobe: {metadata}"
            caption = f"Local {note_type} asset inspected with ffprobe."
            tags.append("ffprobe")
    else:
        fail(f"local_media_worker does not know how to handle asset mime={mime!r}: {asset}")

    if not any([caption, ocr, summary]):
        fail(
            "No local enrichment tool produced output. On macOS install local dependencies, e.g. "
            f"brew install ollama tesseract ffmpeg, then `ollama pull {DEFAULT_OLLAMA_MODEL}` "
            "or set OLLAMA_MODEL to another installed vision model."
        )

    patch = {
        "caption": caption,
        "tags": tags,
        "ocr": ocr,
        "summary": summary,
        "provider": "local-media-worker",
        "model": selected_ollama_model() if caption and "ollama-vision" in tags else "local-tools",
    }
    print(json.dumps({k: v for k, v in patch.items() if v not in (None, "")}, ensure_ascii=False))


if __name__ == "__main__":
    main()
