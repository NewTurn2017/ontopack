#!/usr/bin/env python3
"""Diagnose optional OntoPack media provider tools without mutating the pack."""
from __future__ import annotations

import argparse
import json
import os
import shutil
import subprocess
import sys
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]

TOOLS = [
    ("python3", ["python3", "--version"]),
    ("ffmpeg", ["ffmpeg", "-version"]),
    ("ffprobe", ["ffprobe", "-version"]),
    ("tesseract", ["tesseract", "--version"]),
    ("ollama", ["ollama", "--version"]),
    ("whisper-cli", ["whisper-cli", "--help"]),
]

WORKERS = [
    "scripts/providers/auto_media_worker.py",
    "scripts/providers/local_media_worker.py",
    "scripts/providers/fixture_media_worker.py",
    "scripts/providers/openai_vision_worker.py",
]


def first_line(command: list[str]) -> str:
    try:
        output = subprocess.run(
            command,
            check=False,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            text=True,
            timeout=5,
        ).stdout.strip()
    except Exception as exc:  # pragma: no cover - defensive diagnostics
        return f"error: {exc}"
    return output.splitlines()[0] if output else "no version output"


def tool_check(name: str, version_command: list[str]) -> dict[str, Any]:
    path = shutil.which(name)
    return {
        "name": name,
        "ok": path is not None,
        "path": path,
        "detail": first_line(version_command) if path else "not found on PATH",
    }


def worker_check(rel: str) -> dict[str, Any]:
    path = ROOT / rel
    return {
        "name": rel,
        "ok": path.exists() and os.access(path, os.R_OK),
        "path": str(path),
        "detail": "present" if path.exists() else "missing",
    }


def build_report() -> dict[str, Any]:
    checks = [tool_check(name, command) for name, command in TOOLS]
    worker_checks = [worker_check(rel) for rel in WORKERS]
    env = {
        "OPENAI_API_KEY": bool(os.environ.get("OPENAI_API_KEY")),
        "WHISPER_MODEL": os.environ.get("WHISPER_MODEL"),
        "OLLAMA_MODEL": os.environ.get("OLLAMA_MODEL"),
        "ONTOPACK_PROVIDER_MODE": os.environ.get("ONTOPACK_PROVIDER_MODE"),
        "ONTOPACK_LOCAL_WORKER": os.environ.get("ONTOPACK_LOCAL_WORKER"),
        "ONTOPACK_API_WORKER": os.environ.get("ONTOPACK_API_WORKER"),
    }
    local_ready = any(c["name"] == "ffprobe" and c["ok"] for c in checks) or any(
        c["name"] == "tesseract" and c["ok"] for c in checks
    ) or any(c["name"] == "ollama" and c["ok"] for c in checks)
    api_ready = env["OPENAI_API_KEY"]
    fixture_ready = any(c["name"].endswith("fixture_media_worker.py") and c["ok"] for c in worker_checks)
    return {
        "ok": fixture_ready and all(c["ok"] for c in worker_checks),
        "provider_ready": {
            "fixture": fixture_ready,
            "local_optional_tools": local_ready,
            "api_credentials": api_ready,
        },
        "tools": checks,
        "workers": worker_checks,
        "environment": env,
        "notes": [
            "Only fixture provider readiness is required for deterministic tests.",
            "Local/API providers are optional and depend on installed tools or credentials.",
        ],
    }


def main() -> int:
    parser = argparse.ArgumentParser(description="Diagnose optional OntoPack media provider tools")
    parser.add_argument("--json", action="store_true", help="print JSON report")
    parser.add_argument(
        "--require",
        choices=["fixture", "local", "api"],
        action="append",
        default=[],
        help="fail if the selected provider class is not ready",
    )
    args = parser.parse_args()
    report = build_report()
    if args.json:
        print(json.dumps(report, ensure_ascii=False, indent=2))
    else:
        print(f"provider doctor: ok={str(report['ok']).lower()}")
        for group in ("tools", "workers"):
            print(f"{group}:")
            for check in report[group]:
                status = "ok" if check["ok"] else "missing"
                print(f"- {status} {check['name']} {check['path'] or '-'} :: {check['detail']}")
        ready = report["provider_ready"]
        print(
            "provider_ready: "
            f"fixture={ready['fixture']} local_optional_tools={ready['local_optional_tools']} api_credentials={ready['api_credentials']}"
        )
    missing = [name for name in args.require if not report["provider_ready"][{"fixture":"fixture", "local":"local_optional_tools", "api":"api_credentials"}[name]]]
    if missing:
        print(f"provider requirements missing: {', '.join(missing)}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
