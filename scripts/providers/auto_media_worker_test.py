#!/usr/bin/env python3
"""Smoke tests for auto_media_worker.py. Intended to be run directly."""
import json
import os
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
WORKER = ROOT / "scripts" / "providers" / "auto_media_worker.py"


def run_worker(payload, env):
    merged = os.environ.copy()
    merged.update(env)
    return subprocess.run(
        [sys.executable, str(WORKER)],
        input=json.dumps(payload),
        text=True,
        capture_output=True,
        env=merged,
        check=False,
    )


def test_api_is_preferred_when_available():
    with tempfile.TemporaryDirectory() as tmp:
        api = Path(tmp) / "api.py"
        local = Path(tmp) / "local.py"
        api.write_text(
            "import json, sys; json.load(sys.stdin); "
            "json.dump({'caption':'api caption','provider':'api-stub','model':'stub'}, sys.stdout)\n"
        )
        local.write_text(
            "import json, sys; json.load(sys.stdin); "
            "json.dump({'caption':'local caption','provider':'local-stub','model':'stub'}, sys.stdout)\n"
        )
        result = run_worker(
            {"note_id": "x", "note_type": "image", "asset_abs_path": str(Path(tmp) / "x.png")},
            {
                "OPENAI_API_KEY": "test-key",
                "ONTOPACK_API_WORKER": f"{sys.executable} {api}",
                "ONTOPACK_LOCAL_WORKER": f"{sys.executable} {local}",
            },
        )
        assert result.returncode == 0, result.stderr
        patch = json.loads(result.stdout)
        assert patch["caption"] == "api caption"
        assert patch["provider"] == "api-stub"


def test_local_fallback_without_api_key():
    with tempfile.TemporaryDirectory() as tmp:
        api = Path(tmp) / "api.py"
        local = Path(tmp) / "local.py"
        api.write_text("raise SystemExit('api should not run')\n")
        local.write_text(
            "import json, sys; json.load(sys.stdin); "
            "json.dump({'caption':'local caption','provider':'local-stub','model':'stub'}, sys.stdout)\n"
        )
        result = run_worker(
            {"note_id": "x", "note_type": "image", "asset_abs_path": str(Path(tmp) / "x.png")},
            {
                "OPENAI_API_KEY": "",
                "ONTOPACK_API_WORKER": f"{sys.executable} {api}",
                "ONTOPACK_LOCAL_WORKER": f"{sys.executable} {local}",
            },
        )
        assert result.returncode == 0, result.stderr
        patch = json.loads(result.stdout)
        assert patch["caption"] == "local caption"
        assert patch["provider"] == "local-stub"


def test_honest_failure_when_no_provider_available():
    result = run_worker(
        {"note_id": "x", "note_type": "image", "asset_abs_path": "/tmp/x.png"},
        {
            "OPENAI_API_KEY": "",
            "ONTOPACK_API_WORKER": "/missing/api-worker",
            "ONTOPACK_LOCAL_WORKER": "/missing/local-worker",
            "PATH": "/usr/bin:/bin",
        },
    )
    assert result.returncode != 0
    assert "No provider worker is available" in result.stderr


def main():
    tests = [
        test_api_is_preferred_when_available,
        test_local_fallback_without_api_key,
        test_honest_failure_when_no_provider_available,
    ]
    for test in tests:
        test()
        print(f"ok {test.__name__}")


if __name__ == "__main__":
    main()
