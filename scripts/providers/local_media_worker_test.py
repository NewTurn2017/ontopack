#!/usr/bin/env python3
"""Smoke tests for local_media_worker.py. Intended to be run directly."""
import json
import os
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
WORKER = ROOT / "scripts" / "providers" / "local_media_worker.py"


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


def write_executable(path: Path, content: str) -> None:
    path.write_text(content)
    path.chmod(0o755)


def test_image_default_uses_current_mac_ollama_model():
    with tempfile.TemporaryDirectory() as tmp:
        bin_dir = Path(tmp) / "bin"
        bin_dir.mkdir()
        calls = Path(tmp) / "ollama-calls.json"
        write_executable(
            bin_dir / "ollama",
            f"""#!/usr/bin/env python3
import json, sys
from pathlib import Path
Path({str(calls)!r}).write_text(json.dumps(sys.argv[1:]))
print("local caption")
""",
        )
        write_executable(
            bin_dir / "tesseract",
            """#!/usr/bin/env python3
print("ocr text")
""",
        )
        result = run_worker(
            {"note_id": "x", "note_type": "image", "asset_abs_path": str(Path(tmp) / "x.png")},
            {
                "PATH": f"{bin_dir}{os.pathsep}/usr/bin:/bin",
                "OLLAMA_MODEL": "",
            },
        )
        assert result.returncode == 0, result.stderr
        patch = json.loads(result.stdout)
        assert patch["caption"] == "local caption"
        assert patch["ocr"] == "ocr text"
        assert patch["model"] == "gemma4:e4b"
        args = json.loads(calls.read_text())
        assert args[:5] == ["run", "--nowordwrap", "--hidethinking", "--think=false", "gemma4:e4b"]


def test_image_env_override_reports_selected_model():
    with tempfile.TemporaryDirectory() as tmp:
        bin_dir = Path(tmp) / "bin"
        bin_dir.mkdir()
        write_executable(
            bin_dir / "ollama",
            """#!/usr/bin/env python3
print("custom caption")
""",
        )
        result = run_worker(
            {"note_id": "x", "note_type": "image", "asset_abs_path": str(Path(tmp) / "x.png")},
            {
                "PATH": f"{bin_dir}{os.pathsep}/usr/bin:/bin",
                "OLLAMA_MODEL": "custom-vision:latest",
            },
        )
        assert result.returncode == 0, result.stderr
        patch = json.loads(result.stdout)
        assert patch["caption"] == "custom caption"
        assert patch["model"] == "custom-vision:latest"


def test_video_metadata_uses_local_tools_model_label():
    with tempfile.TemporaryDirectory() as tmp:
        bin_dir = Path(tmp) / "bin"
        bin_dir.mkdir()
        write_executable(
            bin_dir / "ffprobe",
            """#!/usr/bin/env python3
print('{"format":{"duration":"1.0","format_name":"mov"},"streams":[{"codec_type":"video","codec_name":"h264","width":1920,"height":1080}]}')
""",
        )
        result = run_worker(
            {"note_id": "x", "note_type": "video", "asset_abs_path": str(Path(tmp) / "x.mp4")},
            {"PATH": f"{bin_dir}{os.pathsep}/usr/bin:/bin"},
        )
        assert result.returncode == 0, result.stderr
        patch = json.loads(result.stdout)
        assert patch["provider"] == "local-media-worker"
        assert patch["model"] == "local-tools"
        assert "ffprobe" in patch["tags"]


def main():
    tests = [
        test_image_default_uses_current_mac_ollama_model,
        test_image_env_override_reports_selected_model,
        test_video_metadata_uses_local_tools_model_label,
    ]
    for test in tests:
        test()
        print(f"ok {test.__name__}")


if __name__ == "__main__":
    main()
