#!/usr/bin/env python3
"""Local-only OntoPack media worker for macOS-first setup.

Uses installed local tools when available:
- Ollama vision model for image captions (`OLLAMA_MODEL`, default `gemma4:e4b`)
- Tesseract for OCR
- ffprobe for video/audio metadata and video keyframe candidates
- whisper.cpp (`whisper-cli`) for optional audio/video transcription when `WHISPER_MODEL` is set

It never calls a cloud API.
"""
import json
import mimetypes
import os
import re
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path
from typing import Any, Optional

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


def ffprobe_data(path: str) -> Optional[dict[str, Any]]:
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
        return json.loads(proc.stdout)
    except json.JSONDecodeError:
        return None


def ffprobe_summary(data: dict[str, Any]) -> str:
    return json.dumps(data, ensure_ascii=False)


def media_duration_seconds(data: dict[str, Any]) -> Optional[float]:
    try:
        return float(data.get("format", {}).get("duration"))
    except (TypeError, ValueError):
        return None


def has_stream(data: dict[str, Any], stream_type: str) -> bool:
    return any(stream.get("codec_type") == stream_type for stream in data.get("streams", []) or [])


def timestamp(seconds: float) -> str:
    seconds = max(0, int(seconds))
    hours, remainder = divmod(seconds, 3600)
    minutes, secs = divmod(remainder, 60)
    return f"{hours:02d}:{minutes:02d}:{secs:02d}"


def keyframe_candidates(data: dict[str, Any]) -> list[dict[str, str]]:
    if not has_stream(data, "video"):
        return []
    duration = media_duration_seconds(data)
    if not duration or duration <= 1:
        times = [0]
    elif duration < 6:
        times = [0, max(0, duration / 2)]
    else:
        times = [0, duration / 2, max(0, duration - 1)]

    seen = set()
    frames = []
    for seconds in times:
        label = timestamp(seconds)
        if label in seen:
            continue
        seen.add(label)
        frames.append({"time": label, "text": f"Representative video frame candidate at {label}."})
    return frames


def safe_slug(value: str) -> str:
    slug = re.sub(r"[^A-Za-z0-9._-]+", "-", value).strip(".-")
    return slug or "asset"


def pack_root_for_asset(asset_abs_path: str, asset_path: Optional[str]) -> Optional[Path]:
    abs_path = Path(asset_abs_path).resolve()
    if asset_path:
        parts = Path(asset_path).parts
        if parts and parts[0] == "assets":
            root = abs_path
            for _ in parts:
                root = root.parent
            return root
    for parent in [abs_path.parent, *abs_path.parents]:
        if parent.name == "assets":
            return parent.parent
    return None


def extract_keyframe_assets(
    path: str, payload: dict, candidates: list[dict[str, str]]
) -> list[dict[str, str]]:
    if not candidates or not shutil.which("ffmpeg"):
        return candidates
    if os.environ.get("ONTOPACK_EXTRACT_KEYFRAMES", "1").lower() in {"0", "false", "no", "off"}:
        return candidates

    root = pack_root_for_asset(path, payload.get("asset_path"))
    if not root:
        return candidates
    note_id = safe_slug(str(payload.get("note_id") or Path(path).stem))
    derived_rel_dir = Path("assets") / ".derived" / note_id
    derived_abs_dir = root / derived_rel_dir
    derived_abs_dir.mkdir(parents=True, exist_ok=True)

    enriched = []
    for index, candidate in enumerate(candidates):
        rel_path = derived_rel_dir / f"keyframe-{index:04d}.jpg"
        abs_out = root / rel_path
        proc = run(
            [
                "ffmpeg",
                "-y",
                "-ss",
                candidate["time"],
                "-i",
                path,
                "-frames:v",
                "1",
                "-vf",
                "scale='min(480,iw)':-2",
                "-q:v",
                "3",
                str(abs_out),
            ],
            timeout=int(os.environ.get("FFMPEG_TIMEOUT", "180")),
        )
        if proc.returncode == 0 and abs_out.is_file():
            enriched.append(
                {
                    "time": candidate["time"],
                    "text": f"Representative video frame extracted at {candidate['time']}.",
                    "asset": rel_path.as_posix(),
                }
            )
        else:
            enriched.append(candidate)
    return enriched


def selected_whisper_model() -> Optional[str]:
    return os.environ.get("WHISPER_MODEL") or os.environ.get("WHISPER_CPP_MODEL")


def whisper_transcript(path: str, data: dict[str, Any]) -> Optional[str]:
    model = selected_whisper_model()
    if not model or not has_stream(data, "audio"):
        return None
    if not shutil.which("whisper-cli") or not shutil.which("ffmpeg"):
        return None

    with tempfile.TemporaryDirectory(prefix="ontopack-whisper-") as tmp:
        wav = Path(tmp) / "audio.wav"
        ffmpeg = run(
            [
                "ffmpeg",
                "-y",
                "-i",
                path,
                "-vn",
                "-ac",
                "1",
                "-ar",
                "16000",
                "-f",
                "wav",
                str(wav),
            ],
            timeout=int(os.environ.get("FFMPEG_TIMEOUT", "180")),
        )
        if ffmpeg.returncode != 0:
            return None

        command = [
            "whisper-cli",
            "-m",
            model,
            "-f",
            str(wav),
            "-np",
            "-l",
            os.environ.get("WHISPER_LANG", "auto"),
        ]
        duration_ms = os.environ.get("WHISPER_DURATION_MS")
        if duration_ms:
            command.extend(["-d", duration_ms])
        proc = run(command, timeout=int(os.environ.get("WHISPER_TIMEOUT", "600")))
        if proc.returncode != 0:
            return None
        transcript = clean_ollama_output(proc.stdout)
        return transcript or None


def main() -> None:
    payload = json.load(sys.stdin)
    asset = payload.get("asset_abs_path")
    if not asset:
        fail("local_media_worker requires asset_abs_path")
    mime, _ = mimetypes.guess_type(asset)
    note_type = payload.get("note_type") or "asset"

    caption = None
    ocr = None
    transcript = None
    summary = None
    keyframes = []
    tags = ["local", note_type]

    if mime and mime.startswith("image/"):
        caption = ollama_caption(asset, payload)
        ocr = tesseract_ocr(asset)
        if caption:
            tags.append("ollama-vision")
        if ocr:
            tags.append("ocr")
    elif mime and (mime.startswith("video/") or mime.startswith("audio/")):
        metadata = ffprobe_data(asset)
        if metadata:
            summary = f"Local media metadata from ffprobe: {ffprobe_summary(metadata)}"
            caption = f"Local {note_type} asset inspected with ffprobe."
            tags.append("ffprobe")
            keyframes = extract_keyframe_assets(asset, payload, keyframe_candidates(metadata))
            transcript = whisper_transcript(asset, metadata)
            if transcript:
                tags.append("whisper-cpp")
    else:
        fail(f"local_media_worker does not know how to handle asset mime={mime!r}: {asset}")

    if not any([caption, ocr, transcript, summary, keyframes]):
        fail(
            "No local enrichment tool produced output. On macOS install local dependencies, e.g. "
            f"brew install ollama tesseract ffmpeg, then `ollama pull {DEFAULT_OLLAMA_MODEL}` "
            "or set OLLAMA_MODEL to another installed vision model. For audio/video transcripts, "
            "install whisper.cpp and set WHISPER_MODEL=/path/to/ggml-model.bin."
        )

    patch = {
        "caption": caption,
        "tags": tags,
        "ocr": ocr,
        "transcript": transcript,
        "summary": summary,
        "keyframes": keyframes,
        "provider": "local-media-worker",
        "model": selected_ollama_model()
        if caption and "ollama-vision" in tags
        else "local-tools+whisper"
        if transcript
        else "local-tools",
    }
    print(json.dumps({k: v for k, v in patch.items() if v not in (None, "", [])}, ensure_ascii=False))


if __name__ == "__main__":
    main()
