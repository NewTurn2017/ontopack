# ontopack Post-M1 MVP Roadmap

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:writing-plans` to expand each milestone into a task-by-task implementation plan before coding. Use `superpowers:subagent-driven-development` or `$ultragoal` to execute those plans. This document is the roadmap, not the per-task implementation plan.

**Goal:** M1의 키워드 검색 CLI를, 실제로 매일 쓰는 로컬 지식팩 MVP로 끌어올린다.

**Current Baseline:** `ae896cf` 기준 M1 완료. `pack-core` owns pack parsing/add/build/search; `pack-cli` is a thin adapter. SQLite+FTS5 keyword search works and is tested.

**MVP Definition:**
- **Agent-first MVP (v0.1):** 자료를 팩에 넣고 `pack build` 후 Claude Code/Codex에서 MCP로 `search/ask/related/add/timeline`을 호출해 출처 있는 답을 받는다.
- **Human-facing MVP (v0.2):** `pack open`으로 로컬 위키 뷰어를 열고 ask/검색/갤러리/그래프를 브라우저에서 쓴다.

---

## 0. Guiding Principles

1. **Plain files are truth.** `.pack/index.db` is always discardable and rebuildable.
2. **Core owns behavior.** CLI, MCP, and HTTP/viewer are adapters over `pack-core`.
3. **Agent-first before UI.** The fastest useful MVP is MCP search/ask; viewer follows after the core retrieval contract is stable.
4. **Hybrid retrieval before fancy UI.** BGE-M3 + sqlite-vec + BM25 + RRF is the next quality jump.
5. **Small measurable slices.** Each milestone must end with a runnable command and regression tests.

---

## 1. Recommended Sequence

| Milestone | Outcome | Why now | MVP status |
|---|---|---|---|
| M2A — Process + chunks + incremental index foundation | `_inbox` workflow, `chunks` table, changed-note detection | Prepares stable indexing units before embeddings | Required for v0.1 |
| M2B — Embeddings + sqlite-vec | Local BGE-M3 dense embeddings in SQLite | Enables semantic retrieval | Required for v0.1 |
| M2C — Hybrid search + RRF | `pack search` returns fused BM25+vector ranked hits with snippets | Core retrieval quality | Required for v0.1 |
| M3 — MCP server | Claude/Codex can call `search/ask/related/add/timeline` | Agent-first daily use | Agent-first MVP v0.1 |
| M4 — serve/open + viewer | Browser wiki UI with ask, gallery, graph, filters | Human-facing polish/wow | Human-facing MVP v0.2 |
| M5 — Multimodal intake | CLIP image search, video transcription, richer media cards | Broader media knowledge pack | Post-MVP |
| M6 — Knowledge intelligence | duplicates, clusters, orphan notes, topic map | Maintenance/wow analysis | Post-MVP |
| M7 — Distribution | watcher, installer, optional Tauri shell, perf scaling | Productization | Post-MVP |

---

## 2. M2A — Process + chunks + incremental index foundation

### Goal
Make the index model ready for embeddings and citations without adding ML yet.

### Scope
- Add `chunks` table to `.pack/index.db`.
- Split note body into deterministic text chunks.
- Store chunk ordinals and source note IDs.
- Add `pack process` minimal inbox flow.
- Add changed-note detection using `mtime` plus content hash.

### Files likely touched
- `crates/pack-core/src/index.rs`
- `crates/pack-core/src/pack.rs`
- `crates/pack-core/src/note.rs`
- Create `crates/pack-core/src/chunk.rs`
- Create `crates/pack-core/src/process.rs`
- `crates/pack-cli/src/main.rs`
- `crates/pack-cli/tests/cli.rs`
- `README.md`

### Acceptance criteria
- `pack process` moves/imports files from `_inbox` into `notes/` or `assets/` using existing non-overwrite behavior.
- `pack build` creates `notes`, `notes_fts`, `edges`, and `chunks`.
- Rebuilding the same pack is idempotent.
- A long note produces stable chunk IDs across rebuilds when content is unchanged.
- A changed note updates only its derived rows when incremental mode is enabled; full rebuild remains available as the safe fallback.

### Verification
```bash
cargo test -p pack-core chunk
cargo test -p pack-core process
cargo test -p pack-cli process
cargo test
cargo clippy --all-targets -- -D warnings
```

### Stop condition
`pack process && pack build && pack search` works on a temp pack with notes plus asset sidecars, and chunks are queryable from tests.

---

## 3. M2B — Embeddings + sqlite-vec

### Goal
Add local semantic retrieval while preserving offline-first behavior.

### Recommended dependency path
- `fastembed` or a Rust-compatible local embedding crate for BGE-M3.
- `sqlite-vec` for vector storage/search.
- Gate model download/cache behind explicit first-run behavior and clear error messages.

### Scope
- Add embedding config fields to `PackConfig`:
  - `embed_model = "bge-m3"`
  - `embed_enabled = true`
  - `chunk_size`, `chunk_overlap`
- Add `Embedder` trait in `pack-core` so tests can use deterministic fake embeddings.
- Add `vec_chunks` or `chunk_embeddings` table backed by sqlite-vec.
- Add `pack build --no-embed` for fast/index-only rebuilds.
- Add `pack embed` only if separating build/index from embedding proves cleaner.

### Files likely touched
- `crates/pack-core/Cargo.toml`
- `crates/pack-core/src/config.rs`
- `crates/pack-core/src/index.rs`
- Create `crates/pack-core/src/embed.rs`
- `crates/pack-core/src/search.rs`
- `crates/pack-cli/src/main.rs`
- Tests under `pack-core` and `pack-cli`

### Acceptance criteria
- Unit tests can build embeddings with a fake embedder without downloading a model.
- Integration smoke can run real BGE-M3 embedding when model cache is available.
- `pack build --no-embed` remains fully offline and fast.
- Missing model/cache errors are actionable, not panics.
- Existing BM25 search still works if embeddings are disabled.

### Verification
```bash
cargo test -p pack-core embed
cargo test -p pack-core index
cargo test -p pack-core search
cargo test --features real-embed   # only if a feature gate is introduced
cargo clippy --all-targets -- -D warnings
```

### Stop condition
A query that does not share exact Korean keywords with a note can still retrieve the semantically related chunk through vector search in a controlled fixture.

---

## 4. M2C — Hybrid search + RRF

### Goal
Make search quality good enough for `ask` and viewer citations.

### Scope
- Define `ChunkHit` and `SearchHit` with:
  - `note_id`, `chunk_id`, `title`, `note_type`, `snippet`, `score`, `rank_source`, `path`
- Implement BM25 chunk/note retrieval.
- Implement vector retrieval.
- Implement RRF fusion.
- Update `pack search` output to show concise snippets and source IDs.

### Files likely touched
- `crates/pack-core/src/search.rs`
- `crates/pack-core/src/index.rs`
- `crates/pack-core/src/pack.rs`
- `crates/pack-cli/src/main.rs`
- `crates/pack-cli/tests/cli.rs`

### Acceptance criteria
- RRF is deterministic for equal fixture data.
- BM25-only mode, vector-only mode, and hybrid mode are all testable.
- CLI exposes `--mode keyword|vector|hybrid` or equivalent.
- Search result carries enough source metadata for MCP/viewer citation.
- p95 search target for small local fixture stays comfortably below 50ms excluding embedding.

### Verification
```bash
cargo test -p pack-core search
cargo test -p pack-cli search
cargo test
```

### Stop condition
`pack search "질문" --mode hybrid` returns fused ranked source cards with snippets from fixture data.

---

## 5. M3 — MCP server (Agent-first MVP v0.1)

### Goal
Claude Code and Codex can use the pack as a tool, without depending on the CLI output format.

### Scope
- Create `crates/pack-mcp` binary.
- MCP stdio tools:
  - `search(query, type?, k?)`
  - `ask(question, k?)`
  - `related(note_id, depth?)`
  - `add(content|path, type?, tags?)`
  - `timeline(from?, to?, type?)`
- Keep LLM answer generation outside deterministic core at first if needed: MCP returns context blocks/citations; the calling agent can synthesize.
- Add generated or documented tool schemas.
- Add local install/config docs for Claude Code and Codex.

### Files likely touched
- `Cargo.toml`
- Create `crates/pack-mcp/Cargo.toml`
- Create `crates/pack-mcp/src/main.rs`
- `crates/pack-core/src/pack.rs`
- `crates/pack-core/src/search.rs`
- `README.md`
- Create `docs/mcp.md`

### Acceptance criteria
- `pack-mcp` starts over stdio and exposes all MVP tools.
- Tool calls return JSON with stable schemas.
- `search` and `related` are deterministic and do not require network.
- `ask` returns either citation-ready context or a citation-bearing answer, depending on chosen MCP contract.
- Claude Code/Codex config examples are copy-pasteable.

### Verification
```bash
cargo test -p pack-mcp
cargo test
cargo build --release
# stdio smoke with a small JSON-RPC/MCP fixture script
```

### Stop condition
From an agent runtime, a user can ask “이 팩에서 썸네일 훅 관련 자료 찾아줘” and get source IDs/citations back through MCP.

---

## 6. M4 — serve/open + viewer (Human-facing MVP v0.2)

### Goal
Open a fast local wiki UI over the same core search/index APIs.

### Scope
- Add `pack serve` with local HTTP JSON API.
- Add `pack open` to start server and open browser.
- Static viewer SPA:
  - ask/search bar
  - source cards with snippets
  - type/tag/date filters
  - note detail page
  - related panel
  - lightweight graph view
- Prefer no heavy framework until interaction complexity justifies it.

### Files likely touched
- Create `crates/pack-server` or add server module if simpler.
- Create `viewer/` static files.
- `crates/pack-cli/src/main.rs`
- `crates/pack-core/src/search.rs`
- `README.md`
- Create `docs/viewer.md`

### Acceptance criteria
- `pack serve --port 0` binds a local port and serves `/api/search`, `/api/notes/:id`, `/api/related/:id`.
- `pack open` opens the viewer for the current pack.
- Viewer works with JavaScript disabled fallback only if cheap; otherwise no requirement.
- Browser smoke verifies search and note-detail navigation.
- Graph view is bounded by filters to avoid hairball.

### Verification
```bash
cargo test -p pack-server
cargo test -p pack-cli serve
# browser smoke via Playwright or lightweight HTTP assertions
```

### Stop condition
A non-agent user can browse/search/ask a local pack from a browser and click citations into notes.

---

## 7. Post-MVP Backlog

### M5 — Multimodal intake
- CLIP image embeddings.
- Video transcript ingestion through existing Scribe/Whisper workflow.
- Asset thumbnails and media metadata.
- Timeline-aware video citations.

### M6 — Knowledge intelligence
- Duplicate note detection.
- Orphan/gap detection.
- Topic clusters and topic map.
- “관련 노트 자동 추천” as a proactive review report.
- Optional GraphRAG context expansion.

### M7 — Productization
- Watch folder / background daemon.
- Installer and shell completion.
- Optional Tauri shell.
- Pack export/import validation.
- Performance profile for 10k+ notes.
- Evaluate LanceDB only if SQLite+sqlite-vec is measured insufficient.

---

## 8. Execution recommendation

### Immediate next plan to write
Write a task-by-task implementation plan for **M2A only** first:

```bash
# suggested file
docs/superpowers/plans/2026-05-22-ontopack-m2a-process-chunks-incremental.md
```

Why M2A first:
- It reduces risk before model/dependency work.
- It creates the citation unit (`chunks`) that M2B/M2C/M3/M4 all need.
- It keeps tests deterministic without model download complexity.

### Execution lane
Use `$ultragoal` for each milestone. Use Team only when the milestone has separable lanes:
- M2A: solo or subagent-driven is enough.
- M2B: dependency-expert + executor + verifier is useful.
- M3: executor + verifier + writer can run in parallel.
- M4: designer + executor + verifier can run in parallel.

---

## 9. Decision points before M2B

These should be answered by measurement/prototype, not preference:

1. **Embedding crate choice:** Which Rust path gives stable BGE-M3 local inference with acceptable install friction?
2. **sqlite-vec integration shape:** direct table schema vs extension helper wrapper.
3. **Chunk defaults:** initial target around 700-1,000 Korean chars with 100-200 overlap, then tune by retrieval evidence.
4. **Ask contract:** MCP returns context-only vs MCP returns full answer. Recommendation: context-first for v0.1, answer synthesis by caller; add direct answer later.
5. **Viewer graph library:** choose only after API shape is stable. Avoid heavy graph dependency in M3.

---

## 10. Success metrics

- Search p95 < 50ms for thousands of chunks, excluding first query embedding.
- Cold CLI command startup < 1s for non-embedding commands.
- `pack build --no-embed` remains fast and deterministic.
- Rebuild is safe: failed derived-index work never corrupts source files.
- MCP search/ask results always include source note/chunk IDs.
- A fresh pack can be initialized, populated, built, searched, and queried by an agent in under 5 minutes.
