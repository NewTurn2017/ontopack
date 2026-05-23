# OntoPack system deep dive and next development plan

Last refreshed: 2026-05-23

## 0. One-line product definition

OntoPack is a **local-first multimodal knowledge vault**. Plain files remain the source of truth, SQLite/FTS/vector tables are derived indexes, and the CLI/MCP/viewer expose citation-ready source cards so a human or agent can search, inspect, and synthesize from local material without turning the core into a hallucinating answer bot.

## 1. What the system is for

### Primary job

Turn a folder of lecture notes, prompts, transcripts, images, videos, research snippets, and project documents into a searchable local pack.

The pack should support three modes of use:

1. **Human viewer** — `pack open` gives a mechanical vault UI for search, media browsing, timeline, graph, note detail, and Ask context blocks.
2. **Agent connector** — `pack-mcp` lets Codex/Claude call search/context tools plus media enrichment tools against local material.
3. **Terminal workflow** — `pack init/add/process/build/search/embed/serve/open` stays scriptable and testable.

### Non-goal

The deterministic core does **not** generate final LLM answers. `/api/ask` and MCP `ask` return context blocks. A separate LLM should synthesize the final answer with citations.

## 2. How it works today

```text
raw files / _inbox
   │
   ▼
pack process / pack add
pack enrich-pending --provider-command ./worker
   │
   ├─ text/markdown/txt ───────────────► notes/*.md
   │
   └─ image/video/binary asset ────────► assets/*
                                      └► notes/<asset-sidecar>.md
                                             frontmatter: type, title, asset, tags
                                             body: caption/search text
   │
   ▼
pack build / pack build --incremental
   │
   ▼
.pack/index.db
   ├─ notes      metadata + body + asset path + hash
   ├─ notes_fts  SQLite FTS5 title/body/tags
   ├─ chunks     chunked note bodies
   ├─ edges      related/wiki links
   └─ vec_chunks optional sqlite-vec embeddings after pack embed
   │
   ├─ CLI: pack search
   ├─ MCP: pack-mcp tools
   └─ HTTP: pack serve / pack open
        ├─ embedded viewer: /, /app.js, /style.css
        └─ JSON API: /api/search, /api/ask, /api/gallery, ...
```

### Source-of-truth model

- `notes/*.md`: canonical text, frontmatter, tags, relations, captions.
- `assets/*`: original binary media.
- `.pack/index.db`: rebuildable derived index. It should be safe to delete and rebuild.
- `pack.toml`: pack configuration, chunk sizes, embedding defaults.

### M7 storage/enrichment status model

OntoPack now exposes the storage core before UI polish:

- `pack status` scans source-of-truth notes/assets and refreshes `.pack/objects.jsonl`, a rebuildable derived object ledger.
- `pack list --pending-enrichment` shows asset sidecars that still need AI caption/OCR/transcript/tag work.
- `pack enrich-note <note_id>` writes generated caption/tags/transcript into a managed Markdown block between `<!-- ontopack:enrichment:start -->` and `<!-- ontopack:enrichment:end -->`. Human-authored sidecar text outside that block is preserved.
- After `pack build --incremental --no-embed` or MCP `index/rebuild`, generated enrichment text is searchable through CLI, HTTP, and MCP search surfaces.
- `pack enrich-pending --provider-command scripts/providers/auto_media_worker.py` drains pending media sidecars through a JSON-in/JSON-out provider command, writes managed enrichment, refreshes `.pack/objects.jsonl`, and rebuilds search by default. The router prioritizes API providers when credentials exist and falls back to macOS local Ollama/Tesseract/FFmpeg workers when they do not. Bundled worker examples live in `scripts/providers/` and are documented in `docs/providers.md`.
- MCP now exposes the same worker contract: `media/list_pending`, `media/read_note`, `media/write_enrichment`, and `index/rebuild`. This is the intended path for Claude/Codex or another AI provider to caption images, OCR screenshots, transcribe videos/audio, and write results back without owning storage internals.

This keeps original assets and Markdown notes as the durable source of truth while letting Claude/Codex/MCP or future providers act as enrichment workers.

### Current media model

Images/videos already enter the pack as assets plus sidecar notes. Example:

```yaml
---
type: image
title: Board photo caption
asset: assets/evidence.png
tags: [gallery, ontology]
created: 2026-05-21
---
Whiteboard caption searchable by keyword.
```

The viewer now serves local `assets/` files through safe `/assets/<path>` URLs and can render image/video sidecars in gallery cards and selected-note previews. It still does not generate thumbnails, transcode videos, or extract video timelines.

## 3. Detailed execution guide

### 3.1 Build the binaries

From the repository root:

```bash
cargo build --release
export PATH="$PWD/target/release:$PATH"
```

For local development, debug binaries are enough:

```bash
cargo build
export PATH="$PWD/target/debug:$PATH"
```

Optional real embedding build:

```bash
cargo build --release --features real-embed
```

Use the real embedding build only when you want BGE-M3/FastEmbed vector or hybrid search. The first run can download model/runtime files.

### 3.2 Create a demo pack

```bash
pack init ~/ontopack-demo
cd ~/ontopack-demo
```

Add a note:

```bash
cat > _inbox/lecture-outline.md <<'NOTE'
---
type: lecture
title: 로컬 온톨로지 강의 설계
tags: [ontology, lecture]
created: 2026-05-22
related: [board-image]
---
로컬 자료를 노트, 이미지, 영상, 캡션, 메모로 묶어 검색 가능한 지식팩으로 만든다.
NOTE
```

Add an image or video file:

```bash
cp /path/to/board.png _inbox/board.png
cp /path/to/demo.mp4 _inbox/demo.mp4
```

Process inbox:

```bash
pack process
```

Expected result:

- Markdown/text files move to `notes/`.
- Binary media files copy to `assets/`.
- Each binary media file gets a sidecar note under `notes/` with `asset: assets/<file>`.

Edit the generated sidecar note to add useful caption/search text:

```bash
$EDITOR notes/board.md
$EDITOR notes/demo.md
```

### 3.3 Build the searchable index

Offline keyword/chunk index:

```bash
pack build --no-embed
```

Incremental rebuild after edits:

```bash
pack build --incremental --no-embed
```

Optional semantic index:

```bash
pack embed --skip-build
```

If you changed note text before embedding, run:

```bash
pack build --no-embed
pack embed --skip-build
```

### 3.4 Search from CLI

```bash
pack search "온톨로지" --mode keyword -k 10
```

Optional real-embed modes, only after building with `--features real-embed` and running `pack embed`:

```bash
pack search "강의 자료 연결" --mode vector -k 10
pack search "강의 자료 연결" --mode hybrid -k 10
```

### 3.5 Run the viewer

Open browser and keep serving:

```bash
pack open
```

Serve on a fixed port:

```bash
pack serve --port 8787
# open http://127.0.0.1:8787
```

Print a random local URL without opening a browser:

```bash
pack open --port 0 --no-browser --print-url
```

One-shot API smoke:

```bash
pack serve --port 0 --once --request $'GET /api/search?q=%EC%98%A8%ED%86%A8%EB%A1%9C%EC%A7%80 HTTP/1.1\r\nHost: localhost\r\n\r\n'
```

### 3.6 Use MCP with an agent

```bash
pack-mcp --pack-root ~/ontopack-demo
```

Example Codex/Claude MCP config:

```toml
[mcp_servers.ontopack]
command = "/absolute/path/to/ontopack/target/release/pack-mcp"
args = ["--pack-root", "/Users/me/ontopack-demo"]
```

Core MCP tools:

- `search`: source-card search
- `ask`: context blocks, not final answers
- `related`: relation traversal
- `add`: add content/file
- `timeline`: created-date browsing
- `media/list_pending`: media sidecars needing AI enrichment
- `media/read_note`: sidecar body/raw Markdown plus local asset path for an AI worker
- `media/write_enrichment`: safe managed-block caption/OCR/transcript/summary writes
- `index/rebuild`: rebuild derived SQLite index after enrichment writes

### 3.7 Validate the system

Fast MVP smoke:

```bash
scripts/mvp-smoke.sh
```

Realistic pack test:

```bash
scripts/real-test.sh
```

Optional real embedding test:

```bash
RUN_REAL_EMBED=1 scripts/real-test.sh
```

Full development gate:

```bash
cargo fmt --check
cargo test -q
cargo clippy --all-targets -- -D warnings
scripts/real-test.sh
```

## 4. Current bottlenecks and why it is not fast enough yet

### 4.1 HTTP APIs re-scan the filesystem too often

Status: first-pass fix implemented.

The viewer APIs now prefer `.pack/index.db` rows when the index exists and fall back to source markdown scanning only for unbuilt packs. This removes repeated markdown parsing from note detail, related, timeline, graph, facets, and gallery in the normal `pack build` → `pack open` path.

Benchmark status: `scripts/perf-benchmark.sh` measures a persistent local server against a synthetic pack. On 2026-05-23 with 1,200 notes / 120 media notes, M5F showed that keyword search was already fast while APIs materializing all indexed notes were slower (`dashboard_all` p50 37.179ms, gallery/timeline/graph/note/related around 9ms p50).

M5G implemented endpoint-specific SQLite reads for note detail, related, timeline, graph, facets, and gallery. On the same fixture, `dashboard_all` improved to 5.589ms p50, gallery to 1.408ms, timeline to 0.628ms, graph to 1.794ms, note detail to 0.430ms, and related to 0.393ms.

Remaining optimization: run a larger 10k-note benchmark and specialize dashboard/facets payloads if very large tag sets become the next startup bottleneck.

### 4.2 Viewer startup fans out multiple requests

Status: first-pass fix implemented.

The browser now uses `/api/dashboard` for initial facets/gallery/timeline/graph data and for filter-driven panel refreshes. Search remains a separate request only when a query exists. This reduces panel startup fan-out while preserving the existing embedded viewer.

M5D adds stale request cancellation and lightweight timing metrics. Remaining optimization: consider endpoint-specific SQL if dashboard payloads grow too large.

### 4.3 Media visibility

Status: first-pass fix implemented.

Gallery, note detail, and search cards now expose `asset_url`, `media_kind`, and `mime`; the HTTP server safely serves `/assets/...` from the local pack assets directory. Remaining refinement: cache headers and richer file-specific previews.

### 4.4 Search result snippet is too coarse

Status: first-pass fix implemented.

Keyword search still ranks matching notes via FTS, but source cards now choose a chunk containing the query term when possible instead of always showing `ord = 0`. This makes long-note search cards more citation-useful while keeping the fast note-level FTS ranker.

### 4.5 Real vector/hybrid search is CLI-first

Status: first-pass capability gate implemented.

Core and CLI support vector/hybrid with `real-embed`, but server APIs are currently keyword-only. The viewer now calls `/api/capabilities`, enables only `keyword`, and shows vector/hybrid as locked until a future real-embed server capability exists.

## 5. Next development plan: media-visible and much faster viewer

### M5A — Serve local assets safely

Status: implemented as the first pass.

Goal: images/videos become visible in the UI.

Backend tasks:

- Implemented `GET /assets/<path>` route.
- Resolves only inside `pack.root/assets`; rejects traversal like `../`.
- Returns correct content types:
  - images: `image/png`, `image/jpeg`, `image/webp`, `image/gif`, `image/svg+xml`
  - videos: `video/mp4`, `video/webm`, `video/quicktime`
  - fallback: `application/octet-stream`
- Add cache headers for immutable local assets where possible.
- Add tests for image serving, content type, 404, and traversal rejection.

API tasks:

- Extended gallery/note detail/search-card responses with:
  - `asset_url`
  - `media_kind`: `image | video | audio | file | unknown`
  - `mime`

Viewer tasks:

- Gallery cards render:
  - `<img loading="lazy" decoding="async">` for images
  - `<video controls preload="metadata">` for videos
- Selected note panel shows a large media preview when `asset_url` exists.
- Search result cards with assets show a compact thumbnail/icon.

Acceptance:

- `scripts/real-test.sh` includes an actual image/video asset route check.
- Browser screenshot shows at least one visible image or video card.
- Console remains 0 errors/warnings.

### M5B — Move viewer APIs onto SQLite-backed reads

Status: first pass implemented.

Goal: large packs stop reparsing all markdown on every viewer request.

Core/index tasks:

- Implemented `Index::all_notes()` and `Pack::indexed_notes_or_scan()` as the first SQLite-backed read path.
- Server APIs now use indexed note rows when `.pack/index.db` exists:
  - note detail
  - related
  - timeline
  - graph
  - facets
  - gallery
- Existing filesystem parsing remains as a fallback for brand-new packs that have not run `pack build` yet.
- Remaining refinement: add narrower endpoint-specific SQL methods (`note_detail`, `facets`, `gallery`, `timeline`, `graph`, `related`) after dashboard batching proves the final data shape.

Server tasks:

- Changed `pack-server::api` to call the indexed read path.
- Kept fallback scanning when `.pack/index.db` is missing to preserve first-run usability; a future stricter mode can show an explicit `pack build --no-embed` hint.

Acceptance:

- Existing API tests still pass.
- Regression tests prove note detail and gallery still work from the index after source note files are removed.
- Large synthetic pack benchmark is still pending; use M5C dashboard batching as the next measurable speed slice.

### M5C — Add dashboard aggregate endpoint

Status: first pass implemented.

Goal: viewer startup becomes one or two requests, not many redundant scans.

Backend tasks:

- Implemented `GET /api/dashboard?type=&from=&to=&gallery_k=&timeline_k=&graph_limit=` returning:
  - facets
  - gallery preview
  - timeline preview
  - graph summary
- Counts by media kind are still pending.

Viewer tasks:

- Replaced `loadFacets().then(refreshPanels)` fan-out with one dashboard request on startup.
- On filter changes, dashboard updates once plus search runs only if a query exists.

Acceptance:

- Initial dashboard panel data loads with one API request after static assets.
- Filter changes use one dashboard request for panels; stale request cancellation is covered by M5D.

### M5D — Faster, better search interaction

Status: first pass implemented.

Goal: perceived search speed improves even before semantic search is wired.

Implemented viewer tasks:

- Added 180ms typed-search debounce.
- Added `AbortController` cancellation for stale search/dashboard/ask requests.
- Kept loading state local to result/panel regions instead of blocking the full page.

Implemented index/search tasks:

- Source cards now prefer the first chunk containing the query term instead of always `ord = 0`.
- `/api/search`, `/api/ask`, and `/api/dashboard` include `elapsed_ms` timing fields.

Remaining refinement:

- Lazy-render very large result lists if the default cap grows beyond the current viewer limits.
- Expand `source` beyond `sqlite_fts` when server-side vector/hybrid mode exists.

Acceptance:

- Search responses include useful snippets for long notes.
- Browser cancels stale local requests during fast typing/filter changes.
- Tests cover filter-before-limit, snippet selection, timing fields, and real-test API timing checks.

### M5E — Honest vector/hybrid viewer mode

Status: first pass implemented.

Goal: semantic search appears in UI only when available.

Implemented backend tasks:

- Added `/api/capabilities` with `keyword` available and `vector`/`hybrid` locked.
- `/api/search?mode=vector|hybrid` now returns a 400 JSON error instead of silently pretending semantic search works.
- Search responses include `mode: keyword` and `source: sqlite_fts`.

Implemented viewer tasks:

- Added a search mode selector populated from capabilities.
- Disabled unavailable semantic modes with a visible locked status.

Remaining backend tasks:

- Add real `mode=vector|hybrid` execution behind a future `real-embed` server build.
- Load/embedder once per server process, not per request.
- Enable vector/hybrid only after embeddings exist and capability checks pass.

Acceptance:

- Default no-download viewer remains fast, offline, and honest about keyword-only search.
- Current vector/hybrid UI controls are visible but disabled with an explanation.
- Future real-embed viewer can search semantically after explicit build/embed setup.

## 6. Recommended immediate implementation order

1. **Asset route + media previews** — directly addresses “이미지/비디오도 보이도록”.
2. **Indexed gallery/timeline/facets/note APIs** — biggest backend speed win.
3. **Dashboard aggregate endpoint** — biggest viewer startup/perceived speed win.
4. **Search snippet improvement + timing metrics** — makes search feel smarter and measurable.
5. **Synthetic performance benchmark** — implemented; confirmed endpoint-specific SQL was the next speed slice.
6. **Endpoint-specific SQLite reads** — implemented; main non-search viewer APIs now avoid full note materialization.
7. **Vector/hybrid server mode** — only after fast keyword/media path is stable.

## 7. Test strategy for the next phase

Use TDD for each behavior:

- HTTP asset route tests before implementation.
- API response shape tests for `asset_url`, `media_kind`, `mime`.
- Browser QA fixture with one image and one video sidecar.
- Large synthetic pack benchmark fixture for performance regression:

```bash
NOTE_COUNT=1200 MEDIA_COUNT=120 REPEATS=5 WARMUP=1 KEEP_PERF_PACK=1 scripts/perf-benchmark.sh
```

- Existing gates remain mandatory:

```bash
cargo fmt --check
cargo test -q
cargo clippy --all-targets -- -D warnings
scripts/real-test.sh
```

For browser evidence:

```bash
KEEP_REAL_TEST_PACK=1 scripts/real-test.sh
cd <printed pack path>
/path/to/ontopack/target/debug/pack open --port 0 --no-browser --print-url
```

Then run Playwright/Chrome QA against the printed URL and save screenshot under `output/playwright/` plus a written report under `docs/test-results/`.

## 8. Product/design implications

The UI should now evolve from “dashboard with cards” to “archive operations console”:

- Media bay: visual thumbnail/video previews.
- Vault query console: fastest path to source cards.
- Selected record: large preview + metadata + body.
- Context terminal: citation-ready blocks for external LLM.
- System overview: indexed counts, media counts, index freshness, capabilities.
- Graph/timeline: compact navigational aids, not decorative filler.

This keeps the mechanical vault aesthetic while making every panel explain what the system does: **local ingestion, indexing, retrieval, media inspection, relation navigation, and grounded context export**.
