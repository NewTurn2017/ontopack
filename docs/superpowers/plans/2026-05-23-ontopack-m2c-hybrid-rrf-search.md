# ontopack M2C Hybrid Search + RRF Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:test-driven-development` plus inline RED/GREEN execution. Steps use checkbox (`- [ ]`) syntax for tracking. **Iron law:** no production code without first watching the matching test fail.

**Goal:** Upgrade `pack search` and `pack-core` search APIs from note-only BM25 to citation-ready chunk result cards with keyword, vector, and hybrid/RRF modes.

**Architecture:** `search.rs` owns public search result contracts and deterministic RRF fusion. `index.rs` owns SQLite retrieval for keyword chunks and vector chunks. `pack.rs` exposes adapter-safe wrappers; CLI remains thin and defaults to keyword mode. Vector/hybrid CLI modes require `real-embed`, while core tests exercise vector/hybrid using `FakeEmbedder` so default validation stays offline.

**Tech Stack:** Rust, rusqlite/FTS5, sqlite-vec, existing `Embedder` trait, clap `ValueEnum`, assert_cmd/tempfile.

---

## File Structure

- Modify `crates/pack-core/src/search.rs`: add `RankSource`, `SearchHit`, `SearchMode`, `rrf_fuse`, tests.
- Modify `crates/pack-core/src/index.rs`: add keyword chunk retrieval and vector hit path field.
- Modify `crates/pack-core/src/pack.rs`: add keyword/vector/hybrid search wrappers.
- Modify `crates/pack-cli/src/main.rs`: add `--mode keyword|vector|hybrid`, keyword output cards, feature-gated vector/hybrid path.
- Modify `crates/pack-cli/tests/cli.rs`: add keyword card test and non-feature vector/hybrid guard test.
- Modify `README.md`: document M2C modes and feature boundary.

---

## Task 1: Search contracts + deterministic RRF

**Files:**
- Modify: `crates/pack-core/src/search.rs`

- [ ] **Step 1: Write failing tests**

Add tests that create keyword/vector `SearchHit` fixtures and assert `rrf_fuse` returns a shared chunk first with `RankSource::Hybrid`, then deterministic tie order.

- [ ] **Step 2: Verify RED**

```bash
cargo test -p pack-core search::tests::rrf_fusion_promotes_hits_seen_by_both_rankers
```

Expected: compile failure for missing `SearchHit`, `RankSource`, and `rrf_fuse`.

- [ ] **Step 3: Implement minimal contracts and RRF**

Define result structs/enums and implement deterministic RRF over chunk identity.

- [ ] **Step 4: Verify GREEN and commit**

```bash
cargo test -p pack-core search::tests
cargo test -p pack-core
git add crates/pack-core/src/search.rs
git commit
```

---

## Task 2: SQLite keyword chunk cards

**Files:**
- Modify: `crates/pack-core/src/index.rs`
- Modify: `crates/pack-core/src/search.rs` if needed

- [ ] **Step 1: Write failing test**

Add an index test that rebuilds a note and asserts `search_keyword_chunks("훅", 10)` returns note id, chunk id, title, note type, path, snippet text, and `RankSource::Keyword`.

- [ ] **Step 2: Verify RED**

```bash
cargo test -p pack-core index::tests::keyword_chunk_search_returns_citation_ready_cards
```

Expected: missing method failure.

- [ ] **Step 3: Implement keyword chunk retrieval**

Use `notes_fts` for ranking, join first matching note chunk and notes metadata, return `SearchHit` cards.

- [ ] **Step 4: Verify GREEN and commit**

```bash
cargo test -p pack-core index::tests::keyword_chunk_search_returns_citation_ready_cards
cargo test -p pack-core index::tests
git add crates/pack-core/src/index.rs crates/pack-core/src/search.rs
git commit
```

---

## Task 3: Core vector/hybrid wrappers

**Files:**
- Modify: `crates/pack-core/src/index.rs`
- Modify: `crates/pack-core/src/pack.rs`

- [ ] **Step 1: Write failing pack test**

Add a `FakeEmbedder` test that builds embeddings and asserts `Pack::search_hybrid_with("강의 준비", 5, &embedder)` returns a semantic note first with a snippet and `RankSource::Hybrid` or `Vector`.

- [ ] **Step 2: Verify RED**

```bash
cargo test -p pack-core pack::tests::pack_hybrid_search_returns_fused_chunk_cards_with_fake_embedder
```

Expected: missing wrapper/method failure.

- [ ] **Step 3: Implement wrappers**

Convert vector hits to `SearchHit`, add `search_keyword_chunks`, `search_vector_chunks_as_hits`, and `search_hybrid_with` using `rrf_fuse`.

- [ ] **Step 4: Verify GREEN and commit**

```bash
cargo test -p pack-core pack::tests::pack_hybrid_search_returns_fused_chunk_cards_with_fake_embedder
cargo test -p pack-core search index pack
git add crates/pack-core/src/index.rs crates/pack-core/src/pack.rs
git commit
```

---

## Task 4: CLI search mode + docs

**Files:**
- Modify: `crates/pack-cli/src/main.rs`
- Modify: `crates/pack-cli/tests/cli.rs`
- Modify: `README.md`

- [ ] **Step 1: Write failing CLI tests**

Add a keyword mode test asserting `pack search 훅 --mode keyword` prints `[keyword]`, note/chunk ids, and snippet. Add a default-build guard test that `--mode hybrid` fails with `real-embed` guidance.

- [ ] **Step 2: Verify RED**

```bash
cargo test -p pack-cli search_keyword_mode_prints_source_cards
cargo test -p pack-cli search_hybrid_requires_real_embed_feature_by_default
```

Expected: `--mode` not accepted / output not card-shaped.

- [ ] **Step 3: Implement CLI mode parsing and output**

Default mode is keyword. Feature-gate vector/hybrid mode behind `real-embed` and use `FastEmbedder` when enabled.

- [ ] **Step 4: Verify GREEN and commit**

```bash
cargo test -p pack-cli search_keyword_mode_prints_source_cards
cargo test -p pack-cli search_hybrid_requires_real_embed_feature_by_default
cargo test
git add crates/pack-cli/src/main.rs crates/pack-cli/tests/cli.rs README.md
git commit
```

---

## Final Verification

```bash
cargo fmt --check
cargo test
cargo clippy --all-targets -- -D warnings
cargo check -p pack-core --features real-embed
cargo check -p pack-cli --features real-embed
cargo build --release
git diff --check
git status --short
```

Stop when default keyword CLI smoke returns source cards and core hybrid search is proven with fake embeddings.
