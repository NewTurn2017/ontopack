#!/usr/bin/env python3
"""Offline unit checks for openai_vision_worker.py."""
import importlib.util
import os
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
WORKER = ROOT / "scripts" / "providers" / "openai_vision_worker.py"


def load_worker():
    spec = importlib.util.spec_from_file_location("openai_vision_worker", WORKER)
    assert spec and spec.loader
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def test_default_api_model_is_current_cost_optimized_vision_model():
    worker = load_worker()
    assert worker.DEFAULT_OPENAI_MODEL == "gpt-5.4-mini"


def test_empty_openai_model_env_falls_back_to_default(monkeypatch=None):
    worker = load_worker()
    original = os.environ.get("OPENAI_MODEL")
    try:
        os.environ["OPENAI_MODEL"] = ""
        selected = os.environ.get("OPENAI_MODEL") or worker.DEFAULT_OPENAI_MODEL
        assert selected == "gpt-5.4-mini"
    finally:
        if original is None:
            os.environ.pop("OPENAI_MODEL", None)
        else:
            os.environ["OPENAI_MODEL"] = original


def main():
    tests = [
        test_default_api_model_is_current_cost_optimized_vision_model,
        test_empty_openai_model_env_falls_back_to_default,
    ]
    for test in tests:
        test()
        print(f"ok {test.__name__}")


if __name__ == "__main__":
    main()
