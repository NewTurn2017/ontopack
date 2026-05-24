# OntoPack · Local-first knowledge packs

```text
┌──────────────────────────────────────────────────────────────────────────────┐
│ ONTOPACK                                                                     │
│                                                                              │
│   Plain files are the source of truth. SQLite is the fast derived index.      │
│   Capture notes, media, context and citations in one portable local pack.     │
│                                                                              │
│   notes/*.md  +  assets/*  ──▶  pack build/search/serve/mcp/bundle            │
│                                                                              │
│   local-first · MCP-ready · citation-ready · Windows-portable smoke path      │
└──────────────────────────────────────────────────────────────────────────────┘
```

**OntoPack** is a Rust CLI and local viewer for building portable, multimodal knowledge packs.
**온토팩**은 노트·이미지·영상·오디오·에이전트 컨텍스트를 로컬 폴더 하나로 관리하는 Rust 기반 지식 팩 엔진입니다.

- **Source of truth:** Markdown files and assets stay readable on disk.
- **Fast derived index:** SQLite + FTS5 powers keyword search; optional `real-embed` adds BGE-M3 vector/hybrid search.
- **Agent-ready:** MCP server returns citation-ready context blocks instead of hallucinated answers.
- **Portable:** `pack export`, `pack bundle`, and `pack import` move packs across machines.
- **No framework viewer:** `pack open` serves a localhost wiki/gallery/search UI from the binary.

> Current status: pre-release OSS project. Source builds and smoke tests are available; prebuilt binary releases are not published yet.

---

## Quick links

| Topic | Link |
| --- | --- |
| Official-style tutorial page | [`docs/index.html`](docs/index.html) · GitHub Pages after publish: `https://newturn2017.github.io/ontopack/` |
| 5-minute MVP runbook | [`docs/mvp.md`](docs/mvp.md) |
| MCP setup | [`docs/mcp.md`](docs/mcp.md) |
| Local viewer/API | [`docs/viewer.md`](docs/viewer.md) |
| Media providers | [`docs/providers.md`](docs/providers.md) |
| Real test notes | [`docs/real-test.md`](docs/real-test.md) |

---

## Install from GitHub

### macOS / Linux

```bash
# 1) Clone
git clone https://github.com/NewTurn2017/ontopack.git
cd ontopack

# 2) Build the CLI
cargo build --release -p pack-cli

# 3) Run directly
./target/release/pack --help

# 4) Optional: put release binaries on PATH for the tutorial commands below
export PATH="$PWD/target/release:$PATH"

# 5) Optional: install to ~/.local/bin with shell completion
scripts/install.sh --prefix "$HOME/.local" --completion-shell zsh
```

Build every workspace binary, including the MCP server:

```bash
cargo build --release
./target/release/pack-mcp --help
```

Enable semantic vector/hybrid search with the optional real embedding feature:

```bash
cargo build --release -p pack-cli --features real-embed
./target/release/pack embed
./target/release/pack search "ontology" --mode hybrid
```

The first `real-embed` run may download/cache the BGE-M3 model through FastEmbed/Hugging Face.

### Windows / Parallels PowerShell

```powershell
# Install prerequisites first: Git + Rust toolchain from rustup.
# Then clone and build.
git clone https://github.com/NewTurn2017/ontopack.git
cd ontopack
cargo build --release -p pack-cli

# Run the CLI
.\target\release\pack.exe --help

# Validate the Windows portability path
powershell -ExecutionPolicy Bypass -File .\scripts\windows-smoke.ps1 -PackBin .\target\release\pack.exe
```

The Windows smoke covers `init`, `build --no-embed`, `search`, `doctor`, `export`, `bundle`, and `import`.
Live Windows validation is intentionally separated so it can be run on your Parallels machine.

---

## Visible ontology pack smoke · public web metadata

To generate a visually inspectable test pack with 100 prompt records, 100 image records, and 100 video records from public web APIs without downloading original media assets:

```bash
cargo build -p pack-cli
python3 scripts/visible-ontology-pack.py \
  --limit-each 100 \
  --no-download-assets \
  --output /tmp/ontopack-visible-live \
  --pack-bin "$PWD/target/debug/pack" \
  --build
cd /tmp/ontopack-visible-live
/Users/genie/dev/ontopack/target/debug/pack open --no-browser --print-url
```

The runner writes Markdown notes plus `.pack/provenance/*.jsonl`. Search, gallery, and graph APIs use `remote_url`, `thumbnail_url`, `media_kind`, and `mime` frontmatter so the viewer can show remote previews while keeping original downloads opt-in for future ingest commands.

For an offline deterministic smoke with the same 100/100/100 shape:

```bash
python3 scripts/visible-ontology-pack.py \
  --fixture --limit-each 100 --no-download-assets \
  --output /tmp/ontopack-visible-fixture \
  --pack-bin "$PWD/target/debug/pack" \
  --build
```

See [`docs/test-results/2026-05-25-visible-ontology-pack-g001.md`](docs/test-results/2026-05-25-visible-ontology-pack-g001.md) for the verified live run evidence.

---

## 5-minute tutorial · 한국어

### 1. 새 팩 만들기

```bash
pack init ~/ontopack-demo
cd ~/ontopack-demo
```

### 2. `_inbox`에 노트 넣기

```bash
cat > _inbox/hook.md <<'NOTE'
---
type: prompt
title: 썸네일 훅
tags: [youtube, hook]
created: 2026-05-24
---
클릭을 부르는 훅 카피와 강의 오프닝 구조.
NOTE
```

### 3. 처리하고 검색하기

```bash
pack process
pack build --incremental
pack search "훅" --mode keyword
```

성공하면 `[keyword]` source card와 `note_id / chunk_id`가 함께 출력됩니다.

### 4. 로컬 뷰어 열기

```bash
pack open
```

자동화/원격 환경에서는 브라우저를 열지 않고 URL만 출력할 수 있습니다.

```bash
pack open --port 0 --no-browser --print-url
```

### 5. 다른 머신으로 옮기기

```bash
pack bundle ../ontopack-demo-bundle --archive ../ontopack-demo.tar.gz
```

복원할 머신에서:

```bash
pack init ~/restored-pack
cd ~/restored-pack
pack import /path/to/ontopack-demo.tar.gz
pack build --no-embed
pack search "훅"
```

---

## 5-minute tutorial · English

### 1. Create a pack

```bash
pack init ~/ontopack-demo
cd ~/ontopack-demo
```

### 2. Drop a note into `_inbox`

```bash
cat > _inbox/source-card.md <<'NOTE'
---
type: note
title: Source Cards
tags: [research, citation]
created: 2026-05-24
---
A source card keeps useful context, search keywords, and citation-ready snippets.
NOTE
```

### 3. Process, index, and search

```bash
pack process
pack build --incremental
pack search "citation" --mode keyword
```

A successful result prints a citation-ready card with the note id and chunk id.

### 4. Open the local viewer

```bash
pack open
```

Or print a URL without launching a browser:

```bash
pack open --port 0 --no-browser --print-url
```

### 5. Bundle and restore elsewhere

```bash
pack bundle ../ontopack-demo-bundle --archive ../ontopack-demo.tar.gz
```

On another machine:

```bash
pack init ~/restored-pack
cd ~/restored-pack
pack import /path/to/ontopack-demo.tar.gz
pack build --no-embed
pack search "citation"
```

---

## Command map

| Command | Purpose |
| --- | --- |
| `pack init [path]` | Create a pack skeleton. |
| `pack add <file> [--type T]` | Add Markdown as notes, other files as assets with sidecars. |
| `pack process` | Move `_inbox/` files into managed notes/assets. |
| `pack build [--incremental] [--no-embed]` | Build or refresh the derived SQLite/FTS/chunk index. |
| `pack search <query> [--mode keyword\|vector\|hybrid]` | Search and print citation-ready source cards. |
| `pack embed [--skip-build]` | Build vector chunk index with the optional real embedding feature. |
| `pack serve` / `pack open` | Run the localhost JSON API and static viewer. |
| `pack export` / `pack import` / `pack bundle` | Move context and assets across tools or machines. |
| `pack watch [--once]` | Poll `_inbox`, process, and incrementally index. |
| `pack doctor [--json]` | Diagnose install and pack health. |
| `pack duplicates` / `orphans` / `gaps` | Maintain knowledge quality. |
| `pack topics` / `recommend` | Build tag topic maps and relation suggestions. |
| `pack enrich-pending` | Run external media provider workers safely. |
| `pack completions <bash\|zsh\|fish>` | Print shell completion scripts. |

MCP binary:

```bash
pack-mcp --pack-root /path/to/pack
```

MCP tools include `search`, `ask`, `related`, `add`, `timeline`, `media/list_pending`, `media/read_note`, `media/write_enrichment`, and `index/rebuild`.

---

## Pack layout

```text
my-pack/
├── _inbox/             # drop zone for unprocessed files
├── notes/              # Markdown notes and media sidecars: human-readable truth
├── assets/             # original assets plus derived media
└── .pack/              # derived DB/index/runtime metadata; can be rebuilt
```

OntoPack treats `notes/` and `assets/` as durable content. `.pack/` is a derived working area.

---

## Use cases

- **Lecture and research packs:** collect references, slides, screenshots, and source-card snippets.
- **Agent memory packs:** expose local context to Claude/Codex through MCP without uploading a database.
- **Media knowledge bases:** enrich images/video/audio with captions, OCR, transcripts, tags, and summaries.
- **Portable demos:** bundle a pack into `.tar.gz`, move it to another OS, rebuild the index, and search.
- **Knowledge maintenance:** find duplicates, orphans, broken wiki links, topic clusters, and relation candidates.

---

## Verification

Useful local gates:

```bash
cargo test
cargo clippy --all-targets -- -D warnings
scripts/mvp-smoke.sh
scripts/real-test.sh
scripts/perf-smoke.sh
```

Optional heavier proof paths:

```bash
RUN_REAL_EMBED=1 scripts/real-test.sh
NOTE_COUNT=10000 MEDIA_COUNT=800 scripts/perf-benchmark.sh
```

Current development-machine evidence includes successful CLI/MCP/viewer smoke, real embedding path, media intelligence checks, and 10k-note synthetic performance runs. See `docs/test-results/` for generated reports when present.

---

## Project status and license

- Package version: `0.1.0` workspace crates.
- Distribution: source build from GitHub; binary release packaging is not published yet.
- License: not specified in this repository yet. Choose and add a `LICENSE` file before inviting third-party reuse beyond source download/testing.
