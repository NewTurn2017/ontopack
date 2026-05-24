# OntoPack tutorial / 온토팩 튜토리얼

This tutorial is the text companion to `docs/index.html`. It is intentionally command-first so it can be copied into terminals or automation scripts.

이 문서는 `docs/index.html`의 텍스트 버전입니다. 새 머신에서 그대로 복사해 실행할 수 있도록 명령 중심으로 구성했습니다.

## Prerequisites / 준비물

- Git
- Rust/Cargo toolchain
- A terminal or PowerShell

No model download is required for the basic keyword-search path.
기본 keyword 검색 경로에는 모델 다운로드가 필요 없습니다.

## Install / 설치

### macOS / Linux

```bash
git clone https://github.com/NewTurn2017/ontopack.git
cd ontopack
cargo build --release -p pack-cli
./target/release/pack --help
```

Optional install:

```bash
scripts/install.sh --prefix "$HOME/.local" --completion-shell zsh
```

### Windows / Parallels PowerShell

```powershell
git clone https://github.com/NewTurn2017/ontopack.git
cd ontopack
cargo build --release -p pack-cli
.\target\release\pack.exe --help
powershell -ExecutionPolicy Bypass -File .\scripts\windows-smoke.ps1 -PackBin .\target\release\pack.exe
```

## Korean quickstart / 한국어 퀵스타트

```bash
pack init ~/ontopack-demo
cd ~/ontopack-demo
cat > _inbox/hook.md <<'NOTE'
---
type: prompt
title: 썸네일 훅
tags: [youtube, hook]
created: 2026-05-24
---
클릭을 부르는 훅 카피와 강의 오프닝 구조.
NOTE
pack process
pack build --incremental
pack search "훅" --mode keyword
pack open --port 0 --no-browser --print-url
```

## English quickstart

```bash
pack init ~/ontopack-demo
cd ~/ontopack-demo
cat > _inbox/source-card.md <<'NOTE'
---
type: note
title: Source Cards
tags: [research, citation]
created: 2026-05-24
---
A source card keeps useful context, search keywords, and citation-ready snippets.
NOTE
pack process
pack build --incremental
pack search "citation" --mode keyword
pack open --port 0 --no-browser --print-url
```

## Portable bundle / 포터블 번들

```bash
pack bundle ../ontopack-demo-bundle --archive ../ontopack-demo.tar.gz
```

Restore elsewhere:

```bash
pack init ~/restored-pack
cd ~/restored-pack
pack import /path/to/ontopack-demo.tar.gz
pack build --no-embed
pack search "citation"
```

## MCP

```bash
cargo build --release -p pack-mcp
pack-mcp --pack-root /path/to/pack
```

MCP clients can use `search`, `ask`, `related`, `add`, `timeline`, and media tools. `ask` returns context blocks; the AI client is responsible for the final natural-language answer.

## Semantic search / 의미 검색

```bash
cargo build --release -p pack-cli --features real-embed
pack embed
pack search "ontology relation" --mode hybrid
```

The first semantic run may download/cache the BGE-M3 embedding model.
