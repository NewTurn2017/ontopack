# ontopack M2B Embeddings + sqlite-vec Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:test-driven-development` plus `superpowers:executing-plans` or native inline TDD to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking. **Iron law:** no production code without first watching the matching test fail.

**Goal:** Add a deterministic embedding pipeline over M2A chunks, store vectors in SQLite through sqlite-vec, and expose a controlled vector-search API without requiring model downloads in tests.

**Architecture:** `pack-core` owns embedding abstractions, vector storage, and vector retrieval. Tests use a fake embedder so no BGE-M3 download is required. CLI remains a thin adapter and gets a `build --no-embed` escape hatch while semantic search is kept in core for M2C integration.

**Tech Stack:** Rust, rusqlite bundled SQLite/FTS5, sqlite-vec C extension, deterministic fake embedders for tests, clap v4, assert_cmd/tempfile.

---

## File Structure

- Modify `Cargo.toml`: add `sqlite-vec` workspace dependency.
- Modify `crates/pack-core/Cargo.toml`: depend on `sqlite-vec`.
- Modify `crates/pack-core/src/lib.rs`: export `embed`.
- Create `crates/pack-core/src/embed.rs`: `Embedder` trait, vector serialization helpers, test fake embedder.
- Modify `crates/pack-core/src/config.rs`: add `embed_enabled`, `embed_dim`, `chunk_size`, `chunk_overlap` defaults.
- Modify `crates/pack-core/src/index.rs`: register sqlite-vec, create vector table per dimension, store chunk embeddings, query nearest chunks.
- Modify `crates/pack-core/src/pack.rs`: add pack-level embedding build/search wrappers.
- Modify `crates/pack-cli/src/main.rs`: add `build --no-embed` flag while preserving current build behavior.
- Modify `crates/pack-cli/tests/cli.rs`: assert `--no-embed` remains fast/offline.
- Modify `README.md`: document M2B prototype boundaries and commands.

---

## Task 1: Config + CLI no-embed contract

**Files:**
- Modify: `crates/pack-core/src/config.rs`
- Modify: `crates/pack-cli/src/main.rs`
- Modify: `crates/pack-cli/tests/cli.rs`

- [ ] **Step 1: Write failing config and CLI tests**

Add config assertions for `embed_enabled == true`, `embed_dim == 1024`, `chunk_size == 900`, `chunk_overlap == 120`. Add a CLI test that `pack build --no-embed` succeeds and prints the existing index-build success message.

- [ ] **Step 2: Verify RED**

Run:

```bash
cargo test -p pack-core config::tests::parses_config_with_defaults
cargo test -p pack-cli build_no_embed_keeps_keyword_only_build_offline
```

Expected: compile/argument failure because new config fields and `--no-embed` do not exist.

- [ ] **Step 3: Implement minimal config and CLI flag**

Add defaulted config fields and pass `--no-embed` through the CLI without changing the current keyword-only build implementation yet.

- [ ] **Step 4: Verify GREEN and commit**

```bash
cargo test -p pack-core config::tests
cargo test -p pack-cli build_no_embed_keeps_keyword_only_build_offline
git add crates/pack-core/src/config.rs crates/pack-cli/src/main.rs crates/pack-cli/tests/cli.rs
git commit
```

---

## Task 2: Embedder trait + vector bytes helpers

**Files:**
- Create: `crates/pack-core/src/embed.rs`
- Modify: `crates/pack-core/src/lib.rs`

- [ ] **Step 1: Write failing embed tests**

Add tests for `f32s_to_vec_blob`, `vec_blob_to_f32s`, and a deterministic `FakeEmbedder` returning configured query/passage vectors.

- [ ] **Step 2: Verify RED**

Run:

```bash
cargo test -p pack-core embed::tests
```

Expected: compile failure because `embed` module and functions do not exist.

- [ ] **Step 3: Implement minimal embed module**

Implement `Embedder`, `EmbeddingInput`, `EmbeddingVector`, byte conversion helpers, and test-only `FakeEmbedder`.

- [ ] **Step 4: Verify GREEN and commit**

```bash
cargo test -p pack-core embed::tests
cargo test -p pack-core
git add crates/pack-core/src/embed.rs crates/pack-core/src/lib.rs
git commit
```

---

## Task 3: sqlite-vec storage + nearest chunk search

**Files:**
- Modify: `Cargo.toml`
- Modify: `crates/pack-core/Cargo.toml`
- Modify: `crates/pack-core/src/index.rs`

- [ ] **Step 1: Write failing index test**

Add a test that rebuilds notes, indexes chunk embeddings with a fake embedder, then searches with a query vector that retrieves a chunk whose Korean text does not share exact query keywords.

- [ ] **Step 2: Verify RED**

Run:

```bash
cargo test -p pack-core index::tests::vector_search_finds_semantic_chunk_without_keyword_overlap
```

Expected: compile failure because vector embedding methods do not exist.

- [ ] **Step 3: Implement sqlite-vec registration, vector table creation, embedding inserts, and KNN query**

Register `sqlite3_vec_init`, create `vec_chunks` with `embedding float[dim] distance_metric=cosine`, insert embeddings keyed by chunk rowid, and return `ChunkHit` rows joined back to `chunks` and `notes`.

- [ ] **Step 4: Verify GREEN and commit**

```bash
cargo test -p pack-core index::tests::vector_search_finds_semantic_chunk_without_keyword_overlap
cargo test -p pack-core index::tests
git add Cargo.toml Cargo.lock crates/pack-core/Cargo.toml crates/pack-core/src/index.rs
git commit
```

---

## Task 4: Pack-level embedding build/search wrappers + docs

**Files:**
- Modify: `crates/pack-core/src/pack.rs`
- Modify: `README.md`

- [ ] **Step 1: Write failing pack-level test**

Add a test that builds the keyword index, indexes embeddings with a fake embedder through `Pack`, and retrieves a vector hit through `Pack`.

- [ ] **Step 2: Verify RED**

Run:

```bash
cargo test -p pack-core pack::tests::pack_builds_and_searches_chunk_embeddings_with_fake_embedder
```

Expected: compile failure because pack wrappers do not exist.

- [ ] **Step 3: Implement minimal wrappers and docs**

Add `Pack::build_chunk_embeddings_with` and `Pack::search_vector_chunks_with`. Document that M2B has sqlite-vec storage and fake/test embedders; real BGE-M3 provider remains behind the trait for the next slice.

- [ ] **Step 4: Verify GREEN and commit**

```bash
cargo test -p pack-core pack::tests::pack_builds_and_searches_chunk_embeddings_with_fake_embedder
cargo test
git add crates/pack-core/src/pack.rs README.md
git commit
```

---

## Final Verification

```bash
cargo fmt --check
cargo test
cargo clippy --all-targets -- -D warnings
cargo build --release
git diff --check
git status --short
```

Stop when all commands pass, M2B commits are on `main`, and vector retrieval is proven by fake-embedder tests without downloading a model.
