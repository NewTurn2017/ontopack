# ontopack M2B-real FastEmbed Provider Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:test-driven-development` plus `superpowers:executing-plans` or inline RED/GREEN. Steps use checkbox (`- [ ]`) syntax for tracking. **Iron law:** no production code without first watching the matching test fail.

**Goal:** Add an optional real BGE-M3 embedding provider using `fastembed` while keeping default builds/tests offline and model-download-free.

**Architecture:** `pack-core::embed` keeps the existing `Embedder` trait and adds a `FastEmbedder` only behind a `real-embed` Cargo feature. `pack-cli` exposes `pack embed` as the user-facing command; without the feature it returns an actionable error, with the feature it builds the chunk index and writes sqlite-vec embeddings through existing pack APIs. Tests avoid model downloads by checking feature gates and model mapping; optional manual smoke can instantiate BGE-M3 only when explicitly requested.

**Tech Stack:** Rust, optional `fastembed` 5.13.x, existing `sqlite-vec` storage, clap v4, assert_cmd/tempfile. Official API references: docs.rs fastembed `TextEmbedding::try_new`, `EmbeddingModel::BGEM3`, and `TextEmbedding::embed`.

---

## File Structure

- Modify `Cargo.toml`: add optional workspace `fastembed` dependency.
- Modify `crates/pack-core/Cargo.toml`: add `real-embed` feature and optional dependency.
- Modify `crates/pack-cli/Cargo.toml`: add feature pass-through to `pack-core/real-embed`.
- Modify `crates/pack-core/src/embed.rs`: add feature-gated `FastEmbedder`, BGE-M3 model mapping, passage/query prefixes.
- Modify `crates/pack-cli/src/main.rs`: add `pack embed` command with feature-gated implementation and non-feature actionable error.
- Modify `crates/pack-cli/tests/cli.rs`: add default-build test for actionable `pack embed` failure.
- Modify `README.md`: document `--features real-embed` build and `pack embed`.

---

## Task 1: Default CLI guard for real embedding

**Files:**
- Modify: `crates/pack-cli/src/main.rs`
- Modify: `crates/pack-cli/tests/cli.rs`

- [ ] **Step 1: Write failing CLI test**

Add a test `embed_requires_real_embed_feature_by_default` that initializes a pack and runs `pack embed`, expecting failure text containing `real-embed` and `cargo build --release --features real-embed`.

- [ ] **Step 2: Verify RED**

Run:

```bash
cargo test -p pack-cli embed_requires_real_embed_feature_by_default
```

Expected: fail because `embed` command does not exist.

- [ ] **Step 3: Implement minimal command guard**

Add `Commands::Embed` and make the default build return the actionable error.

- [ ] **Step 4: Verify GREEN and commit**

```bash
cargo test -p pack-cli embed_requires_real_embed_feature_by_default
cargo test -p pack-cli
git add crates/pack-cli/src/main.rs crates/pack-cli/tests/cli.rs
git commit
```

---

## Task 2: Feature-gated FastEmbedder core provider

**Files:**
- Modify: `Cargo.toml`
- Modify: `crates/pack-core/Cargo.toml`
- Modify: `crates/pack-core/src/embed.rs`

- [ ] **Step 1: Write failing feature-gated test**

Add a `#[cfg(feature = "real-embed")]` test that asserts `fastembed_model_from_name("bge-m3") == EmbeddingModel::BGEM3` and `fastembed_model_from_name("BAAI/bge-m3") == EmbeddingModel::BGEM3`. This must not instantiate/download a model.

- [ ] **Step 2: Verify RED**

Run:

```bash
cargo test -p pack-core --features real-embed fastembed_model_mapping_accepts_bge_m3_aliases
```

Expected: fail because the feature/dependency/function do not exist.

- [ ] **Step 3: Implement feature, dependency, and provider**

Add optional `fastembed`, implement `FastEmbedder`, map BGE-M3 aliases, and prefix passage/query strings before calling `TextEmbedding::embed`.

- [ ] **Step 4: Verify GREEN and commit**

```bash
cargo test -p pack-core --features real-embed fastembed_model_mapping_accepts_bge_m3_aliases
cargo check -p pack-core --features real-embed
git add Cargo.toml Cargo.lock crates/pack-core/Cargo.toml crates/pack-core/src/embed.rs
git commit
```

---

## Task 3: Feature-enabled CLI wiring and docs

**Files:**
- Modify: `crates/pack-cli/Cargo.toml`
- Modify: `crates/pack-cli/src/main.rs`
- Modify: `README.md`

- [ ] **Step 1: Write failing compile check target**

Run:

```bash
cargo check -p pack-cli --features real-embed
```

Expected: fail because `pack-cli` has no `real-embed` feature and the command does not call `FastEmbedder` yet.

- [ ] **Step 2: Implement feature pass-through and enabled command path**

Add `pack-cli` feature pass-through and make `pack embed` build the index unless `--skip-build` is provided, instantiate `FastEmbedder` from `pack.toml`, then call `Pack::build_chunk_embeddings_with`.

- [ ] **Step 3: Verify GREEN and commit**

```bash
cargo check -p pack-cli --features real-embed
cargo test -p pack-cli embed_requires_real_embed_feature_by_default
cargo test
git add crates/pack-cli/Cargo.toml crates/pack-cli/src/main.rs README.md
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

Stop when default verification is clean, feature-enabled code compiles, and README clearly states that first real embedding may download BGE-M3 via fastembed/Hugging Face.
