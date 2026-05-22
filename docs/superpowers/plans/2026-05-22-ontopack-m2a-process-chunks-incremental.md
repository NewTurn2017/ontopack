# ontopack M2A — Process + Chunks + Incremental Index Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:test-driven-development` plus `superpowers:subagent-driven-development` (recommended) or `superpowers:executing-plans` to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking. **Iron law:** no production code without first watching the matching test fail.

**Goal:** M1의 안정적인 pack-core/CLI 위에 `_inbox` 처리, deterministic chunks, 그리고 변경 감지 기반 증분 인덱싱의 토대를 추가한다.

**Architecture:** `pack-core` keeps all behavior. CLI only parses args and prints `pack-core` results. Chunks are deterministic derived rows from plaintext notes. Incremental build uses note `mtime` + content hash to skip unchanged notes while full rebuild remains the safe fallback.

**Tech Stack:** Rust, rusqlite bundled SQLite/FTS5, serde/serde_yaml/serde_json, clap v4, walkdir, tempfile/assert_cmd/predicates. No ML, sqlite-vec, MCP, server, or viewer in M2A.

---

## TDD operating contract

Every task follows this sequence:

1. Write one failing test for one behavior.
2. Run the narrow command and confirm it fails for the expected reason.
3. Implement the smallest production code that passes.
4. Run the narrow command and confirm green.
5. Run the relevant broader test set.
6. Commit.

Do **not** batch multiple behaviors into one RED test. If a test passes before implementation, rewrite it.

---

## File Structure

```
ontopack/
├─ crates/
│  ├─ pack-core/
│  │  └─ src/
│  │     ├─ lib.rs                 # add chunk/process exports
│  │     ├─ chunk.rs               # deterministic chunking + Chunk model
│  │     ├─ process.rs             # _inbox processing result types/helpers
│  │     ├─ note.rs                # content_hash helper or note hash inputs
│  │     ├─ pack.rs                # Pack::process_inbox, incremental build wrapper
│  │     ├─ index.rs               # chunks table, note hash column, incremental update
│  │     └─ search.rs              # still keyword search only in M2A
│  └─ pack-cli/
│     ├─ src/main.rs               # add process, build --incremental
│     └─ tests/cli.rs              # process/incremental CLI tests
├─ README.md                       # document M2A commands
└─ docs/superpowers/plans/...
```

Responsibility boundaries:
- `chunk.rs` is pure and deterministic: no I/O, no SQLite.
- `process.rs` defines inbox result structs and type inference helpers; filesystem orchestration is exposed through `Pack`.
- `index.rs` owns SQLite schema, full rebuild, and incremental mutation.
- `pack.rs` owns pack-level flows: scan notes, add file, process inbox, build full/incremental.
- `pack-cli/main.rs` remains a thin adapter.

---

## Task 1: Chunk model + deterministic body chunking

**Files:**
- Create: `crates/pack-core/src/chunk.rs`
- Modify: `crates/pack-core/src/lib.rs`

### RED

- [ ] **Step 1: Write failing tests in `chunk.rs`**

Create `crates/pack-core/src/chunk.rs` with tests first:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_body_becomes_one_chunk() {
        let chunks = chunk_text("note-a", "짧은 본문", 20, 5);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].id, "note-a#0000");
        assert_eq!(chunks[0].note_id, "note-a");
        assert_eq!(chunks[0].ord, 0);
        assert_eq!(chunks[0].text, "짧은 본문");
    }

    #[test]
    fn long_body_chunks_with_overlap_without_breaking_utf8() {
        let chunks = chunk_text("n", "가나다라마바사아자차카타파하", 6, 2);
        assert_eq!(chunks.iter().map(|c| c.text.as_str()).collect::<Vec<_>>(), vec![
            "가나다라마바",
            "마바사아자차",
            "자차카타파하",
        ]);
        assert_eq!(chunks[1].id, "n#0001");
        assert_eq!(chunks[2].ord, 2);
    }

    #[test]
    fn blank_body_produces_no_chunks() {
        assert!(chunk_text("n", "  \n\t", 10, 2).is_empty());
    }
}
```

- [ ] **Step 2: Verify RED**

Run:

```bash
cargo test -p pack-core chunk::tests
```

Expected: compile failure because `chunk_text` and `Chunk` are undefined.

### GREEN

- [ ] **Step 3: Implement minimal chunking**

Add above the tests:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chunk {
    pub id: String,
    pub note_id: String,
    pub ord: i64,
    pub text: String,
}

pub fn chunk_text(note_id: &str, body: &str, chunk_chars: usize, overlap_chars: usize) -> Vec<Chunk> {
    let trimmed = body.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }
    let chars: Vec<char> = trimmed.chars().collect();
    let chunk_chars = chunk_chars.max(1);
    let overlap_chars = overlap_chars.min(chunk_chars.saturating_sub(1));
    let step = chunk_chars - overlap_chars;

    let mut chunks = Vec::new();
    let mut start = 0usize;
    let mut ord = 0i64;
    while start < chars.len() {
        let end = (start + chunk_chars).min(chars.len());
        let text: String = chars[start..end].iter().collect();
        chunks.push(Chunk {
            id: format!("{note_id}#{ord:04}"),
            note_id: note_id.to_string(),
            ord,
            text,
        });
        if end == chars.len() {
            break;
        }
        start += step;
        ord += 1;
    }
    chunks
}
```

Modify `crates/pack-core/src/lib.rs`:

```rust
pub mod chunk;
pub mod config;
pub mod index;
pub mod note;
pub mod pack;
pub mod process;
pub mod search;
```

For this task, create an empty `crates/pack-core/src/process.rs` if needed so `lib.rs` compiles later only when Task 5 starts. If you do not add `process` yet, add only `pub mod chunk;` in Task 1 and defer `process` to Task 5.

- [ ] **Step 4: Verify GREEN**

Run:

```bash
cargo test -p pack-core chunk::tests
cargo test -p pack-core
```

Expected: chunk tests pass and existing pack-core tests remain green.

- [ ] **Step 5: Commit**

```bash
git add crates/pack-core/src/chunk.rs crates/pack-core/src/lib.rs
git commit -m "feat(core): add deterministic note chunking"
```

---

## Task 2: SQLite schema stores chunks during full rebuild

**Files:**
- Modify: `crates/pack-core/src/index.rs`
- Modify: `crates/pack-core/src/chunk.rs` if needed

### RED

- [ ] **Step 1: Add failing schema/rebuild tests**

In `crates/pack-core/src/index.rs` tests, add:

```rust
#[test]
fn creates_chunks_table_in_memory() {
    let idx = Index::open_in_memory().unwrap();
    let count: i64 = idx
        .conn
        .query_row(
            "SELECT count(*) FROM sqlite_master WHERE name = 'chunks'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(count, 1);
}

#[test]
fn rebuild_inserts_chunks_for_notes() {
    let mut idx = Index::open_in_memory().unwrap();
    let n = parse_str("a", "0123456789abcdef").unwrap();
    idx.rebuild(&[n]).unwrap();

    let chunks: i64 = idx
        .conn
        .query_row("SELECT count(*) FROM chunks WHERE note_id = 'a'", [], |r| r.get(0))
        .unwrap();
    assert!(chunks >= 1);

    let first: String = idx
        .conn
        .query_row("SELECT id FROM chunks WHERE note_id = 'a' ORDER BY ord LIMIT 1", [], |r| r.get(0))
        .unwrap();
    assert_eq!(first, "a#0000");
}
```

- [ ] **Step 2: Verify RED**

Run:

```bash
cargo test -p pack-core index::tests::creates_chunks_table_in_memory
```

Expected: failure because `chunks` table does not exist.

### GREEN

- [ ] **Step 3: Add `chunks` table and populate it**

In `SCHEMA`, add:

```sql
CREATE TABLE IF NOT EXISTS chunks (
  id      TEXT PRIMARY KEY,
  note_id TEXT NOT NULL,
  ord     INTEGER NOT NULL,
  text    TEXT NOT NULL,
  FOREIGN KEY(note_id) REFERENCES notes(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_chunks_note_ord ON chunks(note_id, ord);
```

In `Index::rebuild`, delete chunks first or after edges:

```rust
tx.execute_batch("DELETE FROM chunks; DELETE FROM notes; DELETE FROM notes_fts; DELETE FROM edges;")?;
```

Prepare insert:

```rust
let mut ins_chunk = tx.prepare(
    "INSERT INTO chunks (id, note_id, ord, text) VALUES (?1, ?2, ?3, ?4)",
)?;
```

Inside the note loop after `notes_fts` insert:

```rust
for chunk in crate::chunk::chunk_text(&n.id, &n.body, 900, 120) {
    ins_chunk.execute(rusqlite::params![chunk.id, chunk.note_id, chunk.ord, chunk.text])?;
}
```

- [ ] **Step 4: Verify GREEN**

Run:

```bash
cargo test -p pack-core index::tests::creates_chunks_table_in_memory
cargo test -p pack-core index::tests::rebuild_inserts_chunks_for_notes
cargo test -p pack-core index::tests
```

Expected: all index tests pass.

- [ ] **Step 5: Commit**

```bash
git add crates/pack-core/src/index.rs
git commit -m "feat(core): store deterministic chunks in the index"
```

---

## Task 3: Content hash + incremental build report types

**Files:**
- Modify: `crates/pack-core/src/note.rs`
- Modify: `crates/pack-core/src/index.rs`

### RED

- [ ] **Step 1: Add failing note hash tests**

In `note.rs` tests, add:

```rust
#[test]
fn content_hash_changes_when_body_or_metadata_changes() {
    let a = parse_str("x", "---\ntitle: A\n---\n본문").unwrap();
    let b = parse_str("x", "---\ntitle: A\n---\n본문 changed").unwrap();
    let c = parse_str("x", "---\ntitle: B\n---\n본문").unwrap();
    assert_ne!(a.content_hash(), b.content_hash());
    assert_ne!(a.content_hash(), c.content_hash());
    assert_eq!(a.content_hash(), a.content_hash());
}
```

- [ ] **Step 2: Verify RED**

Run:

```bash
cargo test -p pack-core note::tests::content_hash_changes_when_body_or_metadata_changes
```

Expected: compile failure because `content_hash` is undefined.

### GREEN

- [ ] **Step 3: Implement deterministic hash**

In `Note` impl in `note.rs`, add:

```rust
impl Note {
    pub fn content_hash(&self) -> String {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.id.hash(&mut hasher);
        self.note_type.hash(&mut hasher);
        self.title.hash(&mut hasher);
        self.tags.hash(&mut hasher);
        self.created.hash(&mut hasher);
        self.asset.hash(&mut hasher);
        self.related.hash(&mut hasher);
        self.body.hash(&mut hasher);
        format!("{:016x}", hasher.finish())
    }
}
```

- [ ] **Step 4: Add index report structs without behavior change**

In `index.rs`, add:

```rust
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct BuildReport {
    pub indexed: usize,
    pub skipped: usize,
    pub removed: usize,
}
```

No incremental method yet.

- [ ] **Step 5: Verify GREEN**

Run:

```bash
cargo test -p pack-core note::tests::content_hash_changes_when_body_or_metadata_changes
cargo test -p pack-core
```

Expected: green.

- [ ] **Step 6: Commit**

```bash
git add crates/pack-core/src/note.rs crates/pack-core/src/index.rs
git commit -m "feat(core): add note content hash and build report type"
```

---

## Task 4: Incremental index skips unchanged notes and removes deleted notes

**Files:**
- Modify: `crates/pack-core/src/index.rs`
- Modify: `crates/pack-core/src/pack.rs`

### RED

- [ ] **Step 1: Add failing incremental tests**

In `index.rs` tests, add:

```rust
#[test]
fn incremental_rebuild_skips_unchanged_notes() {
    let mut idx = Index::open_in_memory().unwrap();
    let n = parse_str("a", "본문").unwrap();
    let first = idx.rebuild_incremental(std::slice::from_ref(&n)).unwrap();
    assert_eq!(first.indexed, 1);
    assert_eq!(first.skipped, 0);

    let second = idx.rebuild_incremental(std::slice::from_ref(&n)).unwrap();
    assert_eq!(second.indexed, 0);
    assert_eq!(second.skipped, 1);
}

#[test]
fn incremental_rebuild_removes_missing_notes() {
    let mut idx = Index::open_in_memory().unwrap();
    let a = parse_str("a", "A").unwrap();
    let b = parse_str("b", "B").unwrap();
    idx.rebuild_incremental(&[a]).unwrap();
    let report = idx.rebuild_incremental(&[b]).unwrap();
    assert_eq!(report.removed, 1);

    let a_count: i64 = idx
        .conn
        .query_row("SELECT count(*) FROM notes WHERE id = 'a'", [], |r| r.get(0))
        .unwrap();
    assert_eq!(a_count, 0);
}
```

- [ ] **Step 2: Verify RED**

Run:

```bash
cargo test -p pack-core index::tests::incremental_rebuild_skips_unchanged_notes
```

Expected: compile failure because `rebuild_incremental` is undefined.

### GREEN

- [ ] **Step 3: Extend schema with `hash`**

Change `notes` schema:

```sql
hash    TEXT NOT NULL
```

Update full rebuild insert statement and params:

```rust
"INSERT INTO notes (id, path, type, title, tags, created, asset, body, mtime, hash)
 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)"
```

Add `n.content_hash()` param.

- [ ] **Step 4: Implement incremental rebuild**

Add helper methods inside `impl Index`:

```rust
pub fn rebuild_incremental(&mut self, notes: &[Note]) -> Result<BuildReport> {
    reject_duplicate_note_ids(notes)?;
    let tx = self.conn.transaction()?;
    let mut report = BuildReport::default();

    let incoming: std::collections::HashSet<&str> = notes.iter().map(|n| n.id.as_str()).collect();
    let existing_ids = collect_existing_note_ids(&tx)?;
    for id in existing_ids {
        if !incoming.contains(id.as_str()) {
            delete_note_rows(&tx, &id)?;
            report.removed += 1;
        }
    }

    for note in notes {
        let hash = note.content_hash();
        let existing: Option<(i64, String)> = tx
            .query_row("SELECT mtime, hash FROM notes WHERE id = ?1", [note.id.as_str()], |r| {
                Ok((r.get(0)?, r.get(1)?))
            })
            .optional()?;
        if existing.as_ref().is_some_and(|(mtime, old_hash)| *mtime == note.mtime && old_hash == &hash) {
            report.skipped += 1;
            continue;
        }
        delete_note_rows(&tx, &note.id)?;
        insert_note_rows(&tx, note, &hash)?;
        report.indexed += 1;
    }

    tx.commit()?;
    Ok(report)
}
```

Required imports:

```rust
use rusqlite::{Connection, OptionalExtension, Transaction};
```

Refactor full rebuild to reuse `insert_note_rows(&Transaction, &Note, &str)` if practical. Keep tests green.

- [ ] **Step 5: Verify GREEN**

Run:

```bash
cargo test -p pack-core index::tests::incremental_rebuild_skips_unchanged_notes
cargo test -p pack-core index::tests::incremental_rebuild_removes_missing_notes
cargo test -p pack-core index::tests
```

Expected: green.

- [ ] **Step 6: Add pack-level wrapper test**

In `pack.rs` tests, add:

```rust
#[test]
fn build_index_incremental_reports_skips() {
    let dir = tempdir().unwrap();
    let root = dir.path().join("p");
    Pack::init(&root, "p").unwrap();
    std::fs::write(root.join("notes/a.md"), "본문").unwrap();
    let pack = Pack::open(&root).unwrap();

    let first = pack.build_index_incremental().unwrap();
    assert_eq!(first.indexed, 1);
    let second = pack.build_index_incremental().unwrap();
    assert_eq!(second.skipped, 1);
}
```

Implement in `Pack`:

```rust
pub fn build_index_incremental(&self) -> Result<crate::index::BuildReport> {
    let notes = self.scan_notes()?;
    let mut idx = Index::open(&self.index_path())?;
    idx.rebuild_incremental(&notes)
}
```

- [ ] **Step 7: Verify pack GREEN**

Run:

```bash
cargo test -p pack-core pack::tests::build_index_incremental_reports_skips
cargo test -p pack-core
```

Expected: green.

- [ ] **Step 8: Commit**

```bash
git add crates/pack-core/src/index.rs crates/pack-core/src/pack.rs
git commit -m "feat(core): add incremental index rebuild"
```

---

## Task 5: `_inbox` process module and pack-level process flow

**Files:**
- Create: `crates/pack-core/src/process.rs`
- Modify: `crates/pack-core/src/lib.rs`
- Modify: `crates/pack-core/src/pack.rs`

### RED

- [ ] **Step 1: Add process tests first**

Create `crates/pack-core/src/process.rs`:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InferredType {
    Note,
    Image,
    Video,
    Asset,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn infers_type_from_extension() {
        assert_eq!(infer_type("memo.md"), InferredType::Note);
        assert_eq!(infer_type("pic.png"), InferredType::Image);
        assert_eq!(infer_type("clip.mp4"), InferredType::Video);
        assert_eq!(infer_type("data.bin"), InferredType::Asset);
    }
}
```

- [ ] **Step 2: Verify RED**

Run:

```bash
cargo test -p pack-core process::tests::infers_type_from_extension
```

Expected: compile failure because `infer_type` is undefined.

### GREEN

- [ ] **Step 3: Implement type inference and process result structs**

```rust
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InferredType {
    Note,
    Image,
    Video,
    Asset,
}

impl InferredType {
    pub fn as_note_type(&self) -> &'static str {
        match self {
            InferredType::Note => "note",
            InferredType::Image => "image",
            InferredType::Video => "video",
            InferredType::Asset => "asset",
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ProcessReport {
    pub processed: usize,
    pub created: Vec<PathBuf>,
}

pub fn infer_type(file_name: &str) -> InferredType {
    let ext = std::path::Path::new(file_name)
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    match ext.as_str() {
        "md" | "markdown" | "txt" => InferredType::Note,
        "png" | "jpg" | "jpeg" | "gif" | "webp" => InferredType::Image,
        "mp4" | "mov" | "mkv" | "webm" => InferredType::Video,
        _ => InferredType::Asset,
    }
}
```

Modify `lib.rs`:

```rust
pub mod process;
```

- [ ] **Step 4: Verify GREEN**

Run:

```bash
cargo test -p pack-core process::tests
cargo test -p pack-core
```

- [ ] **Step 5: Add failing pack process test**

In `pack.rs` tests, add:

```rust
#[test]
fn process_inbox_imports_files_and_clears_inbox() {
    let dir = tempdir().unwrap();
    let root = dir.path().join("p");
    Pack::init(&root, "p").unwrap();
    std::fs::write(root.join("_inbox/memo.md"), "메모").unwrap();
    std::fs::write(root.join("_inbox/pic.png"), [0x89, 0x50, 0x4e, 0x47]).unwrap();

    let pack = Pack::open(&root).unwrap();
    let report = pack.process_inbox().unwrap();

    assert_eq!(report.processed, 2);
    assert!(root.join("notes/memo.md").exists());
    assert!(root.join("assets/pic.png").exists());
    assert!(root.join("notes/pic.md").exists());
    assert!(!root.join("_inbox/memo.md").exists());
    assert!(!root.join("_inbox/pic.png").exists());
}
```

- [ ] **Step 6: Verify RED**

Run:

```bash
cargo test -p pack-core pack::tests::process_inbox_imports_files_and_clears_inbox
```

Expected: compile failure because `process_inbox` is undefined.

- [ ] **Step 7: Implement `Pack::process_inbox`**

In `pack.rs` imports:

```rust
use crate::process::{infer_type, ProcessReport};
```

In `impl Pack`:

```rust
pub fn process_inbox(&self) -> Result<ProcessReport> {
    let inbox = self.root.join("_inbox");
    let mut report = ProcessReport::default();
    for entry in WalkDir::new(&inbox).min_depth(1).max_depth(1) {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let file_name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
        let inferred = infer_type(file_name);
        let outcome = self.add_file(path, inferred.as_note_type())?;
        match outcome {
            AddOutcome::Note { path } => report.created.push(path),
            AddOutcome::AssetWithSidecar { note_path, .. } => report.created.push(note_path),
        }
        std::fs::remove_file(path)?;
        report.processed += 1;
    }
    Ok(report)
}
```

- [ ] **Step 8: Verify GREEN**

Run:

```bash
cargo test -p pack-core process::tests
cargo test -p pack-core pack::tests::process_inbox_imports_files_and_clears_inbox
cargo test -p pack-core
```

Expected: green.

- [ ] **Step 9: Commit**

```bash
git add crates/pack-core/src/process.rs crates/pack-core/src/lib.rs crates/pack-core/src/pack.rs
git commit -m "feat(core): process inbox files into pack notes and assets"
```

---

## Task 6: CLI `process` and `build --incremental`

**Files:**
- Modify: `crates/pack-cli/src/main.rs`
- Modify: `crates/pack-cli/tests/cli.rs`

### RED

- [ ] **Step 1: Add failing CLI tests**

In `crates/pack-cli/tests/cli.rs`, add:

```rust
#[test]
fn process_imports_inbox_files() {
    let dir = tempdir().unwrap();
    let root = dir.path().join("p");
    Command::cargo_bin("pack").unwrap()
        .args(["init", root.to_str().unwrap()]).assert().success();
    std::fs::write(root.join("_inbox/memo.md"), "메모").unwrap();

    Command::cargo_bin("pack").unwrap()
        .current_dir(&root)
        .args(["process"])
        .assert()
        .success()
        .stdout(predicate::str::contains("처리 완료"));

    assert!(root.join("notes/memo.md").exists());
    assert!(!root.join("_inbox/memo.md").exists());
}

#[test]
fn build_incremental_reports_skips_on_second_run() {
    let dir = tempdir().unwrap();
    let root = dir.path().join("p");
    Command::cargo_bin("pack").unwrap()
        .args(["init", root.to_str().unwrap()]).assert().success();
    std::fs::write(root.join("notes/a.md"), "본문").unwrap();

    Command::cargo_bin("pack").unwrap()
        .current_dir(&root).args(["build", "--incremental"])
        .assert().success()
        .stdout(predicate::str::contains("indexed=1"));

    Command::cargo_bin("pack").unwrap()
        .current_dir(&root).args(["build", "--incremental"])
        .assert().success()
        .stdout(predicate::str::contains("skipped=1"));
}
```

- [ ] **Step 2: Verify RED**

Run:

```bash
cargo test -p pack-cli process_imports_inbox_files
```

Expected: failure because CLI lacks `process`.

### GREEN

- [ ] **Step 3: Add CLI commands**

Modify `Commands`:

```rust
/// _inbox 파일을 notes/assets로 정리한다
Process,

/// 인덱스를 (재)빌드한다
Build {
    /// 변경된 노트만 갱신한다
    #[arg(long)]
    incremental: bool,
},
```

Update match:

```rust
Commands::Process => {
    let root = find_pack_root(&std::env::current_dir()?)?;
    let pack = Pack::open(&root)?;
    let report = pack.process_inbox()?;
    println!("인박스 처리 완료: {}개", report.processed);
}
Commands::Build { incremental } => {
    let root = find_pack_root(&std::env::current_dir()?)?;
    let pack = Pack::open(&root)?;
    if incremental {
        let report = pack.build_index_incremental()?;
        println!(
            "증분 인덱스 빌드 완료: indexed={} skipped={} removed={}",
            report.indexed, report.skipped, report.removed
        );
    } else {
        let count = pack.build_index()?;
        println!("인덱스 빌드 완료: 노트 {count}개");
    }
}
```

Update existing tests that call `args(["build"])` only if enum syntax requires no changes to CLI invocation. It should remain `pack build`.

- [ ] **Step 4: Verify GREEN**

Run:

```bash
cargo test -p pack-cli process_imports_inbox_files
cargo test -p pack-cli build_incremental_reports_skips_on_second_run
cargo test -p pack-cli
```

Expected: green.

- [ ] **Step 5: Commit**

```bash
git add crates/pack-cli/src/main.rs crates/pack-cli/tests/cli.rs
git commit -m "feat(cli): add process and incremental build flags"
```

---

## Task 7: End-to-end process/build/search smoke and README update

**Files:**
- Modify: `README.md`
- Optionally modify tests if smoke exposes a missing behavior

### RED

- [ ] **Step 1: Add failing end-to-end CLI test**

In `crates/pack-cli/tests/cli.rs`, add:

```rust
#[test]
fn end_to_end_process_build_and_search() {
    let dir = tempdir().unwrap();
    let root = dir.path().join("p");
    Command::cargo_bin("pack").unwrap()
        .args(["init", root.to_str().unwrap()]).assert().success();
    std::fs::write(
        root.join("_inbox/hook.md"),
        "---\ntype: prompt\ntitle: 썸네일 훅\n---\n클릭을 부르는 훅 카피.",
    ).unwrap();

    Command::cargo_bin("pack").unwrap()
        .current_dir(&root).args(["process"])
        .assert().success();
    Command::cargo_bin("pack").unwrap()
        .current_dir(&root).args(["build", "--incremental"])
        .assert().success();
    Command::cargo_bin("pack").unwrap()
        .current_dir(&root).args(["search", "훅"])
        .assert().success()
        .stdout(predicate::str::contains("썸네일 훅"));
}
```

- [ ] **Step 2: Verify RED if any part missing**

Run:

```bash
cargo test -p pack-cli end_to_end_process_build_and_search
```

Expected: fail until Tasks 5-6 are implemented. If it already passes after Tasks 5-6, keep it as regression coverage and proceed.

### GREEN

- [ ] **Step 3: Update README**

Update M1/M2A section:

```markdown
## M2A
- `pack process` — `_inbox/` 파일을 `notes/` 또는 `assets/` + 사이드카로 정리
- `pack build --incremental` — 변경된 노트만 파생 인덱스 갱신
- 인덱스는 `notes`, `notes_fts`, `edges`, `chunks`를 재생성/갱신
```

- [ ] **Step 4: Manual smoke**

Run:

```bash
SMOKE_DIR=$(mktemp -d)
./target/release/pack init "$SMOKE_DIR/p"
printf -- "---\ntype: prompt\ntitle: 썸네일 훅\n---\n클릭을 부르는 훅 카피.\n" > "$SMOKE_DIR/p/_inbox/hook.md"
cd "$SMOKE_DIR/p"
/Users/genie/dev/ontopack/target/release/pack process
/Users/genie/dev/ontopack/target/release/pack build --incremental
/Users/genie/dev/ontopack/target/release/pack search 훅
```

Expected output includes:

```text
인박스 처리 완료: 1개
증분 인덱스 빌드 완료: indexed=1 skipped=0 removed=0
[prompt] 썸네일 훅  (hook)
```

- [ ] **Step 5: Commit**

```bash
git add README.md crates/pack-cli/tests/cli.rs
git commit -m "docs: document M2A process and incremental build workflow"
```

---

## Task 8: Final quality gate

**Files:**
- No new production files unless review finds required fixes.

- [ ] **Step 1: Run full verification**

```bash
cargo fmt --check
cargo test
cargo clippy --all-targets -- -D warnings
cargo build --release
git diff --check
```

Expected: all pass.

- [ ] **Step 2: Run changed-file ai-slop-cleaner pass**

Scope:
- `crates/pack-core/src/chunk.rs`
- `crates/pack-core/src/process.rs`
- `crates/pack-core/src/index.rs`
- `crates/pack-core/src/pack.rs`
- `crates/pack-core/src/note.rs`
- `crates/pack-core/src/lib.rs`
- `crates/pack-cli/src/main.rs`
- `crates/pack-cli/tests/cli.rs`
- `README.md`

Required checks:
- No CLI-owned domain logic.
- No silent source-file overwrite.
- No swallowed WalkDir/process errors.
- No partial index mutation outside transaction.
- No broad speculative abstractions for embeddings/MCP.

- [ ] **Step 3: Run final code review**

Expected final gate:
- code-review recommendation: `APPROVE`
- architect status: `CLEAR`

- [ ] **Step 4: Final commit if cleanup changed code**

Use Lore protocol.

---

## Self-review checklist

Spec coverage:
- `_inbox`/`process` minimal workflow: Tasks 5-7.
- `chunks` table and deterministic chunk IDs: Tasks 1-2.
- changed-note detection with `mtime` + content hash: Tasks 3-4.
- full rebuild safe fallback remains: Task 4 preserves `Pack::build_index` and CLI `pack build`.
- no embeddings/sqlite-vec/MCP/viewer yet: explicitly out of scope.

Placeholder scan:
- No `TBD`, `TODO`, or “add appropriate tests” placeholders allowed in this plan.

Type consistency:
- `Chunk`, `BuildReport`, `ProcessReport`, `InferredType`, `Pack::process_inbox`, and `Pack::build_index_incremental` are named consistently across tasks.

---

## Execution handoff

Plan complete. Recommended execution:

```text
$ultragoal M2A: Execute docs/superpowers/plans/2026-05-22-ontopack-m2a-process-chunks-incremental.md with strict TDD. For every behavior, write the failing test first, run it to verify RED, implement the minimum, verify GREEN, then commit. Do not add embeddings, sqlite-vec, MCP, server, or viewer.
```

For speed, use `superpowers:subagent-driven-development` per task with review after each task. For lower coordination overhead, execute inline with `superpowers:executing-plans`.
