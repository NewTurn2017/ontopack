#!/usr/bin/env python3
"""OntoPack provider router: API first, then local Mac-ready fallback.

Default priority:
1. API provider when OPENAI_API_KEY is present.
2. Local provider when API credentials are absent.
3. Honest failure with setup instructions when neither path is available.

Environment overrides:
- ONTOPACK_PROVIDER_MODE=auto|api|local
- ONTOPACK_API_WORKER="/path/to/worker [args...]"
- ONTOPACK_LOCAL_WORKER="/path/to/worker [args...]"
"""
import json
import os
import shlex
import shutil
import subprocess
import sys
from pathlib import Path

SCRIPT_DIR = Path(__file__).resolve().parent
DEFAULT_API_WORKER = SCRIPT_DIR / "openai_vision_worker.py"
DEFAULT_LOCAL_WORKER = SCRIPT_DIR / "local_media_worker.py"


def fail(message: str, code: int = 2) -> None:
    print(message, file=sys.stderr)
    raise SystemExit(code)


def command_from_env(name: str, default: Path) -> list[str]:
    raw = os.environ.get(name)
    if raw:
        return shlex.split(raw)
    return [str(default)]


def command_available(command: list[str]) -> bool:
    if not command:
        return False
    executable = command[0]
    if os.path.isabs(executable) or "/" in executable:
        return Path(executable).exists()
    return shutil.which(executable) is not None


def run_worker(command: list[str], payload: dict) -> dict:
    proc = subprocess.run(
        command,
        input=json.dumps(payload, ensure_ascii=False),
        text=True,
        capture_output=True,
        check=False,
    )
    if proc.returncode != 0:
        fail(proc.stderr.strip() or f"provider failed: {' '.join(command)}", proc.returncode)
    try:
        return json.loads(proc.stdout)
    except json.JSONDecodeError as exc:
        fail(f"provider returned invalid JSON: {exc}: {proc.stdout[:500]}", 1)


def main() -> None:
    payload = json.load(sys.stdin)
    mode = os.environ.get("ONTOPACK_PROVIDER_MODE", "auto").strip().lower()
    if mode not in {"auto", "api", "local"}:
        fail("ONTOPACK_PROVIDER_MODE must be one of: auto, api, local")

    api_command = command_from_env("ONTOPACK_API_WORKER", DEFAULT_API_WORKER)
    local_command = command_from_env("ONTOPACK_LOCAL_WORKER", DEFAULT_LOCAL_WORKER)

    api_ready = bool(os.environ.get("OPENAI_API_KEY")) and command_available(api_command)
    local_ready = command_available(local_command)

    if mode == "api":
        if not api_ready:
            fail("API provider requested but OPENAI_API_KEY or API worker is missing; pack was not modified.")
        patch = run_worker(api_command, payload)
    elif mode == "local":
        if not local_ready:
            fail("Local provider requested but local worker is missing; install local tools or set ONTOPACK_LOCAL_WORKER.")
        patch = run_worker(local_command, payload)
    elif api_ready:
        patch = run_worker(api_command, payload)
    elif local_ready:
        patch = run_worker(local_command, payload)
    else:
        fail(
            "No provider worker is available. For API mode set OPENAI_API_KEY. "
            "For local Mac mode install/use scripts/providers/local_media_worker.py dependencies "
            "(Ollama for vision, Tesseract for OCR, FFmpeg/Whisper for video/audio), "
            "or set ONTOPACK_LOCAL_WORKER."
        )

    print(json.dumps(patch, ensure_ascii=False))


if __name__ == "__main__":
    main()
