# OntoPack M7 — Universal Knowledge Ingest + Enrichment MCP Plan

Date: 2026-05-23
Status: ready for implementation
Primary goal: make OntoPack a reliable local-first ontology pack that can save, recall, enrich, and export any user knowledge object, independent of the UI.

## 1. Product target

OntoPack should become a **local-first universal knowledge pack**:

```text
any file / text / URL / media / agent note
  -> durable asset or note in the pack
  -> structured sidecar metadata
  -> optional AI enrichment through Claude/Codex/MCP or provider adapters
  -> deterministic index rebuild
  -> searchable/retrievable through CLI, MCP, HTTP, and future UI
```

The viewer is secondary. The core promise is:

> “언제든 저장하고, 어디서든 불러오고, 어떤 형식에도 적용 가능한 내 지식 온톨로지팩.”

## 2. Current code facts

- `Pack::init` creates `notes/`, `assets/`, `_inbox/`, and `.pack/` as the local source-of-truth structure (`crates/pack-core/src/pack.rs:85-96`).
- `Pack::add_file` already preserves non-text files in `assets/` and writes a Markdown sidecar note with `type`, `title`, `asset`, and empty `tags` (`crates/pack-core/src/pack.rs:205-239`).
- `Pack::process_inbox` only processes top-level `_inbox` files and delegates to `add_file`, then removes the inbox file (`crates/pack-core/src/pack.rs:269-289`).
- `infer_type` recognizes text, images, videos, and generic assets by extension (`crates/pack-core/src/process.rs:28-39`).
- `Note` only stores normalized core fields: `id`, `path`, `type`, `title`, `tags`, `created`, `asset`, `related`, `body`, `mtime` (`crates/pack-core/src/note.rs:7-19`). Extra media/enrichment fields are not first-class yet.
- CLI commands cover `init/add/process/build/embed/search/serve/open`, but no enrichment or manifest/status command exists (`crates/pack-cli/src/main.rs:17-84`).
- MCP tools currently cover `search`, `ask`, `related`, `add`, and `timeline`; no media/enrichment write tools exist yet (`crates/pack-mcp/src/server.rs:63-130`).

## 3. Principles

1. **Source of truth remains plain files**: original assets stay under `assets/`, human/agent-readable sidecars stay under `notes/`.
2. **AI is a worker, not the core**: Claude/Codex/providers enrich through MCP/CLI contracts; `pack-core` remains deterministic.
3. **Everything is resumable**: interrupted enrichment must leave a readable pending/error state and never corrupt original assets.
4. **Every derived claim is attributable**: generated captions, tags, transcripts, OCR, and keyframes record provider/model/time/source.
5. **Retrieval before UI**: CLI/MCP/API must prove save/load/search/export before visual polish.

## 4. Decision drivers

1. **Durable local ownership**: no hidden cloud-only state.
2. **Agent interoperability**: Claude, Codex, and future agents must operate through stable MCP tools.
3. **Format coverage**: text, image, video, audio, PDFs, URLs, and arbitrary files must fit one model.

## 5. Architecture decision

### Chosen approach: deterministic pack core + MCP enrichment tools

OntoPack core owns:

- file copy/import
- sidecar note generation
- pack manifest/status
- index build/search
- safe read/write operations

Claude/Codex or provider adapters own:

- image captioning
- OCR
- video/audio transcription
- keyframe description
- semantic tags
- high-level summaries

MCP becomes the agent-safe bridge:

```text
media/list_pending
media/read_asset
media/read_sidecar
media/write_enrichment
media/mark_status
index/rebuild
export/context_bundle
```

### Alternatives rejected

- **Put all AI directly into `pack-core`**: rejected because model/provider churn would make the deterministic core unstable.
- **UI-first enrichment**: rejected because the product promise is storage/retrieval anywhere, not a dashboard demo.
- **Database-only knowledge store**: rejected because users need plain-file portability and recoverability.

## 6. M7 implementation slices

### M7A — Pack manifest + media/enrichment status foundation

Goal: make every imported object introspectable and resumable.

Implementation:

- Add a durable `.pack/manifest.jsonl` or `.pack/objects.jsonl` derived ledger.
- Add core structs for `PackObject`, `ObjectStatus`, and `EnrichmentStatus`.
- Track at minimum:
  - note id
  - note path
  - asset path if any
  - detected kind: note/image/video/audio/pdf/asset
  - content hash
  - indexed status
  - enrichment status: `none|pending|done|error`
  - updated timestamp
- Add CLI:
  - `pack status`
  - `pack list --pending-enrichment`
- Tests:
  - importing image/video creates status rows
  - modifying sidecar changes hash/status deterministically
  - manifest can be rebuilt from notes/assets if missing

Acceptance criteria:

- A user can run `pack status` and see exactly what is stored, what is indexed, and what is still not AI-enriched.
- No AI dependency is introduced in this slice.

### M7B — Safe sidecar update API

Goal: let agents enrich sidecars without overwriting user-authored content.

Implementation:

- Add `Pack::update_enrichment(note_id, patch)` or equivalent.
- Write enrichment into clearly bounded sections:

```markdown
## AI Caption
...

## AI Tags
...

## Transcript
...

## Keyframes
...

## Enrichment Metadata
provider: codex
model: ...
generated_at: ...
```

- Preserve existing human body text.
- Use atomic temp-file write + rename.
- Add conflict behavior:
  - default append/update managed sections only
  - never delete unknown human sections
  - error if note id missing

Acceptance criteria:

- Agent can add caption/tags/transcript to a sidecar.
- Human-written content outside managed sections survives repeated updates.
- Re-running update is idempotent for the same patch.

### M7C — MCP enrichment surface

Goal: make Claude/Codex the enrichment worker through MCP.

Add MCP tools:

```text
media/list_pending
media/read_asset
media/read_note
media/write_enrichment
media/mark_enrichment_error
index/rebuild
```

Tool semantics:

- `media/list_pending`: returns asset notes needing caption/OCR/transcript.
- `media/read_asset`: returns path, mime, size, and optionally local asset URL/path for the agent runtime.
- `media/read_note`: returns current sidecar content.
- `media/write_enrichment`: writes structured caption/tags/transcript/keyframes.
- `index/rebuild`: rebuilds after enrichment.

Acceptance criteria:

- A Claude/Codex MCP client can list pending images/videos, read their sidecars, write enrichment, rebuild, and search the new text.
- Existing tools remain backward compatible.

### M7D — Local utility extractors

Goal: handle non-LLM deterministic media prep locally.

Implementation:

- Detect local tools:
  - `ffmpeg`
  - `ffprobe`
  - optional `whisper.cpp`/local STT later
- Add CLI dry-run:
  - `pack media inspect <note_id>`
  - `pack media extract-keyframes <note_id>`
  - `pack media extract-audio <note_id>`
- Store derived artifacts under `.pack/derived/<note_id>/` or `assets/.derived/` with clear derived status.

Acceptance criteria:

- Real video duration and stream info are visible.
- Keyframe/audio extraction is testable without any cloud AI.

### M7E — Provider-backed enrichment command

Goal: optional one-command enrichment when provider credentials/tools exist.

Implementation:

```bash
pack enrich --provider mcp-agent
pack enrich --provider openai
pack enrich --provider local
pack enrich --dry-run
```

Provider interface:

```text
input: asset path + current sidecar + detected media metadata
output: caption, tags, transcript, keyframes, summary, confidence, provenance
```

The first real provider can be an MCP-agent workflow rather than embedded API calls.

Acceptance criteria:

- With a provider available, one image becomes searchable by generated caption/tags.
- With provider unavailable, command fails honestly with setup instructions and leaves pack unchanged.

### M7F — Export/context portability

Goal: make stored knowledge usable anywhere.

Implementation:

- Add exports:
  - `pack export --format markdown-bundle`
  - `pack export --format jsonl`
  - `pack export --format mcp-context`
- Ensure output includes citations to note ids and asset paths.

Acceptance criteria:

- Same pack content can be passed to Codex/Claude, another app, or a lecture/demo bundle without the viewer.

## 7. First vertical to implement next

Start with **M7A + M7B**, not AI provider work.

Why:

- AI enrichment needs a safe place to write results first.
- MCP tools need stable status and update primitives.
- This proves the storage/retrieval core before adding model/provider complexity.

Concrete first PR/commit target:

```text
M7A/B: Add pack object status and safe enrichment sidecar updates
```

Files likely touched:

- `crates/pack-core/src/pack.rs`
- `crates/pack-core/src/note.rs`
- new `crates/pack-core/src/enrichment.rs` or `object.rs`
- `crates/pack-cli/src/main.rs`
- `crates/pack-cli/tests/cli.rs`
- `docs/system-deep-dive.md`
- `docs/mcp.md` after M7C
- `scripts/real-test.sh`

## 8. Verification plan

### Unit tests

- parse sidecar with managed enrichment sections
- update enrichment without deleting human sections
- idempotent repeat update
- object status detects asset/note kind correctly

### CLI tests

- `pack status` after `pack add image.png`
- `pack list --pending-enrichment`
- enrichment write command if exposed in CLI

### MCP tests, M7C

- `tools/list` includes media tools
- `media/list_pending` returns imported image/video
- `media/write_enrichment` makes generated caption searchable after rebuild

### E2E real test

- create real image/video fixture
- add/process/build
- write enrichment
- rebuild
- search by generated caption/tag/transcript
- verify note detail returns enriched text

## 9. Risks and mitigations

- Risk: sidecar updates overwrite user notes.
  - Mitigation: managed section markers and atomic writes only.
- Risk: provider-specific fields pollute core schema.
  - Mitigation: generic provenance metadata in body/frontmatter; provider adapters stay outside core.
- Risk: manifest becomes another source of truth.
  - Mitigation: manifest is rebuildable from notes/assets; source-of-truth remains files.
- Risk: binary assets leak through MCP unsafely.
  - Mitigation: explicit local path return, size/mime checks, no remote upload by core.

## 10. Stop condition for M7 MVP

M7 MVP is complete when this works end-to-end:

```bash
pack init demo
cp image.png demo/_inbox/
cd demo
pack process
pack status
# Claude/Codex via MCP writes caption/tags OR local test writes enrichment patch
pack build --incremental --no-embed
pack search "generated visual concept" --mode keyword
```

Expected result:

- original image remains in `assets/`
- sidecar contains AI/human-readable enrichment
- search finds the asset by generated caption/tag text
- MCP can retrieve the enriched source card
