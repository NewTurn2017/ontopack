# ontopack M1 — 코어 + CLI (기초) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 평문 마크다운 팩을 파싱해 SQLite+FTS5 인덱스로 만들고, 키워드(BM25) 검색이 CLI에서 동작하는 최소 기초를 만든다. ML 의존성 없음.

**Architecture:** Rust 워크스페이스. `pack-core`(라이브러리)가 팩 파싱·인덱싱·검색 로직 전부를 갖고, `pack-cli`(바이너리)는 그 위의 얇은 어댑터. 데이터(팩)는 평문 파일이 진실, `.pack/index.db`는 재생성 가능한 파생 캐시.

**Tech Stack:** Rust, rusqlite(bundled, FTS5 포함), serde/serde_yaml/serde_json, clap v4, walkdir, regex, anyhow. 테스트: 내장 `#[test]` + tempfile + assert_cmd + predicates.

> **Phase 1 분해**: M1(이 계획) = 코어+CLI 기초. M2 = 임베딩+sqlite-vec+하이브리드/RRF. M3 = MCP. M4 = 뷰어. M1은 의도적으로 키워드 검색만, 위험을 먼저 제거한다. `_inbox`/`process`·증분 인덱싱·임베딩은 M2 이후.

> **설계 문서**: `docs/superpowers/specs/2026-05-22-ontopack-design.md`

---

## File Structure

```
ontopack/
├─ Cargo.toml                      # 워크스페이스 정의
├─ crates/
│  ├─ pack-core/
│  │  ├─ Cargo.toml
│  │  └─ src/
│  │     ├─ lib.rs                 # 공개 re-export
│  │     ├─ note.rs                # Note 모델, frontmatter/위키링크 파싱
│  │     ├─ config.rs              # pack.toml 로드 (PackConfig)
│  │     ├─ pack.rs                # Pack: init/open/find_root/scan_notes
│  │     ├─ index.rs               # SQLite 스키마, rebuild, FTS5
│  │     └─ search.rs              # NoteHit, keyword 검색
│  └─ pack-cli/
│     ├─ Cargo.toml
│     └─ src/main.rs               # clap: init/add/build/search
```

각 파일 책임:
- `note.rs` — 한 파일을 `Note`로 파싱(frontmatter + 본문 + 위키링크). 순수 함수, I/O 최소.
- `config.rs` — `pack.toml` 읽기(타입/관계 어휘, 임베딩 설정 자리만).
- `pack.rs` — 팩 루트 탐색·생성, `notes/` 스캔.
- `index.rs` — `.pack/index.db` 스키마와 전체 재빌드(notes/notes_fts/edges).
- `search.rs` — FTS5 BM25 쿼리 → 순위.
- `pack-cli/main.rs` — 명령 → `pack-core` 호출.

---

## Task 1: 워크스페이스 + 두 크레이트 골격

**Files:**
- Create: `Cargo.toml` (워크스페이스)
- Create: `crates/pack-core/Cargo.toml`
- Create: `crates/pack-core/src/lib.rs`
- Create: `crates/pack-cli/Cargo.toml`
- Create: `crates/pack-cli/src/main.rs`

- [ ] **Step 1: 워크스페이스 Cargo.toml 작성**

`Cargo.toml`:
```toml
[workspace]
resolver = "2"
members = ["crates/pack-core", "crates/pack-cli"]

[workspace.dependencies]
anyhow = "1"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
serde_yaml = "0.9"   # 안정. 원하면 유지보수 포크 serde_yml로 교체 가능(API 호환)
rusqlite = { version = "0.32", features = ["bundled"] }  # bundled = FTS5 포함, 시스템 SQLite 불필요
walkdir = "2"
regex = "1"
clap = { version = "4", features = ["derive"] }
toml = "0.8"
tempfile = "3"
assert_cmd = "2"
predicates = "3"
```

- [ ] **Step 2: pack-core/Cargo.toml 작성**

`crates/pack-core/Cargo.toml`:
```toml
[package]
name = "pack-core"
version = "0.1.0"
edition = "2021"

[dependencies]
anyhow = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
serde_yaml = { workspace = true }
rusqlite = { workspace = true }
walkdir = { workspace = true }
regex = { workspace = true }
toml = { workspace = true }

[dev-dependencies]
tempfile = { workspace = true }
```

- [ ] **Step 3: pack-core/src/lib.rs 골격 작성**

`crates/pack-core/src/lib.rs`:
```rust
pub mod config;
pub mod index;
pub mod note;
pub mod pack;
pub mod search;

pub use note::Note;
pub use pack::Pack;
pub use search::NoteHit;
```

(이 시점엔 하위 모듈이 없으니 빈 모듈 파일을 만든다.)

`crates/pack-core/src/note.rs`, `config.rs`, `pack.rs`, `index.rs`, `search.rs` — 각각 빈 파일로 생성(다음 태스크에서 채움).

- [ ] **Step 4: pack-cli/Cargo.toml 작성**

`crates/pack-cli/Cargo.toml`:
```toml
[package]
name = "pack-cli"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "pack"
path = "src/main.rs"

[dependencies]
pack-core = { path = "../pack-core" }
anyhow = { workspace = true }
clap = { workspace = true }

[dev-dependencies]
assert_cmd = { workspace = true }
predicates = { workspace = true }
tempfile = { workspace = true }
```

- [ ] **Step 5: pack-cli/src/main.rs 최소 골격**

`crates/pack-cli/src/main.rs`:
```rust
fn main() -> anyhow::Result<()> {
    println!("pack");
    Ok(())
}
```

- [ ] **Step 6: 빌드 확인**

Run: `cargo build`
Expected: 성공(경고는 무시). 두 크레이트 컴파일됨.

- [ ] **Step 7: 커밋**

```bash
git add Cargo.toml crates/
git commit -m "chore: ontopack 워크스페이스 + pack-core/pack-cli 골격"
```

---

## Task 2: Note frontmatter 파싱

**Files:**
- Modify: `crates/pack-core/src/note.rs`

- [ ] **Step 1: 실패하는 테스트 작성**

`crates/pack-core/src/note.rs` 하단에:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_frontmatter_and_body() {
        let raw = "---\ntype: prompt\ntitle: 썸네일 훅\ntags: [thumbnail, hook]\nrelated:\n  - \"[[project_오로라]]\"\n---\n본문 텍스트.\n";
        let note = parse_str("prompt_x", raw).unwrap();
        assert_eq!(note.id, "prompt_x");
        assert_eq!(note.note_type, "prompt");
        assert_eq!(note.title, "썸네일 훅");
        assert_eq!(note.tags, vec!["thumbnail", "hook"]);
        assert_eq!(note.related, vec!["project_오로라"]);
        assert_eq!(note.body.trim(), "본문 텍스트.");
    }

    #[test]
    fn defaults_when_no_frontmatter() {
        let note = parse_str("plain", "그냥 본문만.").unwrap();
        assert_eq!(note.note_type, "note");
        assert_eq!(note.title, "plain");
        assert!(note.tags.is_empty());
        assert_eq!(note.body.trim(), "그냥 본문만.");
    }
}
```

- [ ] **Step 2: 테스트 실패 확인**

Run: `cargo test -p pack-core note::tests`
Expected: 컴파일 실패(`parse_str`, `Note` 미정의).

- [ ] **Step 3: 최소 구현 작성**

`crates/pack-core/src/note.rs` 상단에:
```rust
use anyhow::Result;
use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct Note {
    pub id: String,
    pub path: PathBuf,
    pub note_type: String,
    pub title: String,
    pub tags: Vec<String>,
    pub created: Option<String>,
    pub asset: Option<String>,
    pub related: Vec<String>,
    pub body: String,
    pub mtime: i64,
}

#[derive(Debug, Default, Deserialize)]
struct FrontMatter {
    #[serde(rename = "type")]
    note_type: Option<String>,
    title: Option<String>,
    tags: Option<Vec<String>>,
    created: Option<String>,
    asset: Option<String>,
    related: Option<Vec<String>>,
}

/// `---\n...\n---\n` frontmatter를 본문과 분리한다. frontmatter가 없으면 (None, 전체).
fn split_frontmatter(raw: &str) -> (Option<&str>, &str) {
    let trimmed = raw.strip_prefix('\u{feff}').unwrap_or(raw); // BOM 제거
    if let Some(rest) = trimmed.strip_prefix("---\n") {
        if let Some(end) = rest.find("\n---\n") {
            let fm = &rest[..end];
            let body = &rest[end + 5..];
            return (Some(fm), body);
        }
        if let Some(end) = rest.find("\n---") {
            // 파일이 `---`로 끝나는 경우
            let fm = &rest[..end];
            let body = rest.get(end + 4..).unwrap_or("");
            return (Some(fm), body);
        }
    }
    (None, trimmed)
}

/// 위키링크 문자열 `"[[name|alias]]"` → 정규화된 id `name`.
fn normalize_link(s: &str) -> String {
    let s = s.trim();
    let s = s.strip_prefix("[[").unwrap_or(s);
    let s = s.strip_suffix("]]").unwrap_or(s);
    s.split('|').next().unwrap_or(s).trim().to_string()
}

/// id와 원문으로 Note 생성. mtime은 0(파일에서 읽을 땐 parse_file이 채움).
pub fn parse_str(id: &str, raw: &str) -> Result<Note> {
    let (fm_raw, body) = split_frontmatter(raw);
    let fm: FrontMatter = match fm_raw {
        Some(f) => serde_yaml::from_str(f)?,
        None => FrontMatter::default(),
    };
    let related = fm
        .related
        .unwrap_or_default()
        .iter()
        .map(|s| normalize_link(s))
        .collect();
    Ok(Note {
        id: id.to_string(),
        path: PathBuf::new(),
        note_type: fm.note_type.unwrap_or_else(|| "note".to_string()),
        title: fm.title.unwrap_or_else(|| id.to_string()),
        tags: fm.tags.unwrap_or_default(),
        created: fm.created,
        asset: fm.asset,
        related,
        body: body.to_string(),
        mtime: 0,
    })
}
```

- [ ] **Step 4: 테스트 통과 확인**

Run: `cargo test -p pack-core note::tests`
Expected: 2개 PASS.

- [ ] **Step 5: 커밋**

```bash
git add crates/pack-core/src/note.rs
git commit -m "feat(core): Note frontmatter/본문 파싱(parse_str)"
```

---

## Task 3: 본문 위키링크 추출 + 파일 파싱

**Files:**
- Modify: `crates/pack-core/src/note.rs`

- [ ] **Step 1: 실패하는 테스트 추가**

`note.rs`의 `mod tests`에 추가:
```rust
    #[test]
    fn extracts_body_wikilinks() {
        let links = extract_wikilinks("앞 [[a]] 중간 [[b|별칭]] 끝 [[c]]");
        assert_eq!(links, vec!["a", "b", "c"]);
    }

    #[test]
    fn related_merges_frontmatter_and_body_dedup() {
        let raw = "---\nrelated:\n  - \"[[a]]\"\n---\n본문 [[a]] 그리고 [[d]]";
        let note = parse_str("x", raw).unwrap();
        assert_eq!(note.related, vec!["a", "d"]);
    }
```

- [ ] **Step 2: 테스트 실패 확인**

Run: `cargo test -p pack-core note::tests`
Expected: `extract_wikilinks` 미정의로 실패.

- [ ] **Step 3: 구현 추가**

`note.rs`에 `regex` 사용 추가. 상단 import에 `use regex::Regex;` 추가하고:
```rust
/// 본문에서 `[[...]]` 위키링크 id 목록을 등장 순서대로(중복 제거) 추출.
pub fn extract_wikilinks(text: &str) -> Vec<String> {
    let re = Regex::new(r"\[\[([^\]]+)\]\]").unwrap();
    let mut out = Vec::new();
    for cap in re.captures_iter(text) {
        let id = normalize_link(&cap[1]);
        if !id.is_empty() && !out.contains(&id) {
            out.push(id);
        }
    }
    out
}
```

`parse_str`에서 related 계산을 frontmatter + 본문 병합으로 교체:
```rust
    let mut related: Vec<String> = fm
        .related
        .unwrap_or_default()
        .iter()
        .map(|s| normalize_link(s))
        .collect();
    for l in extract_wikilinks(body) {
        if !related.contains(&l) {
            related.push(l);
        }
    }
```

`parse_file` 추가(파일에서 id=파일stem, mtime 채움):
```rust
use std::time::UNIX_EPOCH;

/// 파일을 읽어 Note로. id = 파일명(확장자 제외), mtime = 수정시각(초).
pub fn parse_file(path: &Path) -> Result<Note> {
    let raw = std::fs::read_to_string(path)?;
    let id = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("untitled")
        .to_string();
    let mut note = parse_str(&id, &raw)?;
    note.path = path.to_path_buf();
    let meta = std::fs::metadata(path)?;
    note.mtime = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    Ok(note)
}
```

- [ ] **Step 4: 테스트 통과 확인**

Run: `cargo test -p pack-core note`
Expected: 모든 note 테스트 PASS.

- [ ] **Step 5: 커밋**

```bash
git add crates/pack-core/src/note.rs
git commit -m "feat(core): 위키링크 추출 + parse_file(파일→Note)"
```

---

## Task 4: PackConfig (pack.toml)

**Files:**
- Modify: `crates/pack-core/src/config.rs`

- [ ] **Step 1: 실패하는 테스트 작성**

`crates/pack-core/src/config.rs`:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_config_with_defaults() {
        let cfg: PackConfig = toml::from_str("name = \"내 팩\"\n").unwrap();
        assert_eq!(cfg.name, "내 팩");
        assert!(cfg.types.is_empty());
    }

    #[test]
    fn parses_types_and_relations() {
        let cfg: PackConfig =
            toml::from_str("name = \"p\"\ntypes = [\"prompt\", \"image\"]\nrelations = [\"related\"]\n")
                .unwrap();
        assert_eq!(cfg.types, vec!["prompt", "image"]);
        assert_eq!(cfg.relations, vec!["related"]);
    }
}
```

- [ ] **Step 2: 테스트 실패 확인**

Run: `cargo test -p pack-core config::tests`
Expected: `PackConfig` 미정의로 실패.

- [ ] **Step 3: 구현 작성**

`crates/pack-core/src/config.rs` 상단:
```rust
use anyhow::Result;
use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Clone, Deserialize)]
pub struct PackConfig {
    pub name: String,
    #[serde(default)]
    pub types: Vec<String>,
    #[serde(default)]
    pub relations: Vec<String>,
    /// M2에서 사용. 지금은 자리만.
    #[serde(default = "default_embed_model")]
    pub embed_model: String,
}

fn default_embed_model() -> String {
    "bge-m3".to_string()
}

impl PackConfig {
    /// 팩 루트의 pack.toml을 읽는다.
    pub fn load(root: &Path) -> Result<PackConfig> {
        let raw = std::fs::read_to_string(root.join("pack.toml"))?;
        Ok(toml::from_str(&raw)?)
    }

    /// init 시 기본 설정의 직렬화 문자열.
    pub fn default_toml(name: &str) -> String {
        format!(
            "name = \"{name}\"\ntypes = [\"prompt\", \"image\", \"video\", \"project\"]\nrelations = [\"related\"]\nembed_model = \"bge-m3\"\n"
        )
    }
}
```

- [ ] **Step 4: 테스트 통과 확인**

Run: `cargo test -p pack-core config::tests`
Expected: 2개 PASS.

- [ ] **Step 5: 커밋**

```bash
git add crates/pack-core/src/config.rs
git commit -m "feat(core): PackConfig(pack.toml) 로드/기본값"
```

---

## Task 5: Pack — init / find_root / open / scan_notes

**Files:**
- Modify: `crates/pack-core/src/pack.rs`

- [ ] **Step 1: 실패하는 테스트 작성**

`crates/pack-core/src/pack.rs`:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn init_creates_skeleton() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("my-pack");
        Pack::init(&root, "my-pack").unwrap();
        assert!(root.join("pack.toml").exists());
        assert!(root.join("notes").is_dir());
        assert!(root.join("assets").is_dir());
        assert!(root.join("_inbox").is_dir());
    }

    #[test]
    fn find_root_walks_up() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("p");
        Pack::init(&root, "p").unwrap();
        let nested = root.join("notes");
        let found = find_pack_root(&nested).unwrap();
        assert_eq!(found, root);
    }

    #[test]
    fn scan_notes_reads_markdown() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("p");
        Pack::init(&root, "p").unwrap();
        std::fs::write(
            root.join("notes/a.md"),
            "---\ntype: prompt\ntitle: A\n---\n본문 a",
        )
        .unwrap();
        std::fs::write(root.join("notes/b.md"), "본문 b").unwrap();
        let pack = Pack::open(&root).unwrap();
        let mut notes = pack.scan_notes().unwrap();
        notes.sort_by(|x, y| x.id.cmp(&y.id));
        assert_eq!(notes.len(), 2);
        assert_eq!(notes[0].id, "a");
        assert_eq!(notes[0].note_type, "prompt");
        assert_eq!(notes[1].id, "b");
    }
}
```

- [ ] **Step 2: 테스트 실패 확인**

Run: `cargo test -p pack-core pack::tests`
Expected: `Pack`/`find_pack_root` 미정의로 실패.

- [ ] **Step 3: 구현 작성**

`crates/pack-core/src/pack.rs` 상단:
```rust
use crate::config::PackConfig;
use crate::note::{self, Note};
use anyhow::{anyhow, Result};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub struct Pack {
    pub root: PathBuf,
    pub config: PackConfig,
}

impl Pack {
    /// 새 팩 골격 생성: pack.toml + notes/ assets/ _inbox/ .pack/
    pub fn init(root: &Path, name: &str) -> Result<()> {
        std::fs::create_dir_all(root.join("notes"))?;
        std::fs::create_dir_all(root.join("assets"))?;
        std::fs::create_dir_all(root.join("_inbox"))?;
        std::fs::create_dir_all(root.join(".pack"))?;
        let toml_path = root.join("pack.toml");
        if !toml_path.exists() {
            std::fs::write(&toml_path, PackConfig::default_toml(name))?;
        }
        Ok(())
    }

    /// pack.toml이 있는 디렉터리를 팩 루트로 연다.
    pub fn open(root: &Path) -> Result<Pack> {
        let config = PackConfig::load(root)?;
        Ok(Pack {
            root: root.to_path_buf(),
            config,
        })
    }

    /// notes/ 아래 모든 .md를 Note로 읽는다.
    pub fn scan_notes(&self) -> Result<Vec<Note>> {
        let notes_dir = self.root.join("notes");
        let mut out = Vec::new();
        for entry in WalkDir::new(&notes_dir).into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("md") {
                out.push(note::parse_file(path)?);
            }
        }
        Ok(out)
    }

    /// 인덱스 DB 경로 (.pack/index.db)
    pub fn index_path(&self) -> PathBuf {
        self.root.join(".pack").join("index.db")
    }
}

/// start에서 위로 올라가며 pack.toml이 있는 디렉터리를 찾는다.
pub fn find_pack_root(start: &Path) -> Result<PathBuf> {
    let mut cur = if start.is_file() {
        start.parent().unwrap_or(start)
    } else {
        start
    };
    loop {
        if cur.join("pack.toml").exists() {
            return Ok(cur.to_path_buf());
        }
        match cur.parent() {
            Some(p) => cur = p,
            None => return Err(anyhow!("pack.toml을 찾지 못함(여기는 팩 안이 아닙니다)")),
        }
    }
}
```

- [ ] **Step 4: 테스트 통과 확인**

Run: `cargo test -p pack-core pack::tests`
Expected: 3개 PASS.

- [ ] **Step 5: 커밋**

```bash
git add crates/pack-core/src/pack.rs
git commit -m "feat(core): Pack init/open/scan_notes + find_pack_root"
```

---

## Task 6: Index — 스키마 생성

**Files:**
- Modify: `crates/pack-core/src/index.rs`

- [ ] **Step 1: 실패하는 테스트 작성**

`crates/pack-core/src/index.rs`:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_schema_in_memory() {
        let idx = Index::open_in_memory().unwrap();
        // 세 테이블이 존재해야 한다
        let count: i64 = idx
            .conn
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE name IN ('notes','notes_fts','edges')",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 3);
    }
}
```

- [ ] **Step 2: 테스트 실패 확인**

Run: `cargo test -p pack-core index::tests`
Expected: `Index` 미정의로 실패.

- [ ] **Step 3: 구현 작성**

`crates/pack-core/src/index.rs` 상단:
```rust
use anyhow::Result;
use rusqlite::Connection;
use std::path::Path;

pub struct Index {
    pub conn: Connection,
}

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS notes (
  id      TEXT PRIMARY KEY,
  path    TEXT NOT NULL,
  type    TEXT NOT NULL,
  title   TEXT NOT NULL,
  tags    TEXT NOT NULL,
  created TEXT,
  asset   TEXT,
  body    TEXT NOT NULL,
  mtime   INTEGER NOT NULL
);
CREATE VIRTUAL TABLE IF NOT EXISTS notes_fts USING fts5(
  id UNINDEXED, title, body, tags
);
CREATE TABLE IF NOT EXISTS edges (
  src  TEXT NOT NULL,
  dst  TEXT NOT NULL,
  kind TEXT NOT NULL,
  PRIMARY KEY (src, dst, kind)
);
";

impl Index {
    fn init(conn: Connection) -> Result<Index> {
        conn.execute_batch(SCHEMA)?;
        Ok(Index { conn })
    }

    pub fn open(db_path: &Path) -> Result<Index> {
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        Index::init(Connection::open(db_path)?)
    }

    pub fn open_in_memory() -> Result<Index> {
        Index::init(Connection::open_in_memory()?)
    }
}
```

- [ ] **Step 4: 테스트 통과 확인**

Run: `cargo test -p pack-core index::tests`
Expected: PASS. (bundled rusqlite는 FTS5 포함 — 가상테이블 생성 성공해야 함)

- [ ] **Step 5: 커밋**

```bash
git add crates/pack-core/src/index.rs
git commit -m "feat(core): Index 스키마(notes/notes_fts/edges) 생성"
```

---

## Task 7: Index — 전체 재빌드(rebuild)

**Files:**
- Modify: `crates/pack-core/src/index.rs`

> M1은 단순·정확성 우선으로 매번 전체 재빌드(테이블 비우고 재삽입). 증분 인덱싱은 M2에서 mtime 비교로 추가.

- [ ] **Step 1: 실패하는 테스트 추가**

`index.rs`의 `mod tests`에 추가:
```rust
    use crate::note::parse_str;

    #[test]
    fn rebuild_inserts_notes_and_edges() {
        let idx = Index::open_in_memory().unwrap();
        let mut n1 = parse_str("a", "---\ntype: prompt\ntitle: 알파\ntags: [x]\nrelated:\n  - \"[[b]]\"\n---\n알파 본문").unwrap();
        n1.path = "notes/a.md".into();
        let mut n2 = parse_str("b", "베타 본문").unwrap();
        n2.path = "notes/b.md".into();
        idx.rebuild(&[n1, n2]).unwrap();

        let notes: i64 = idx.conn.query_row("SELECT count(*) FROM notes", [], |r| r.get(0)).unwrap();
        assert_eq!(notes, 2);
        let fts: i64 = idx.conn.query_row("SELECT count(*) FROM notes_fts", [], |r| r.get(0)).unwrap();
        assert_eq!(fts, 2);
        let edges: i64 = idx.conn.query_row("SELECT count(*) FROM edges WHERE src='a' AND dst='b'", [], |r| r.get(0)).unwrap();
        assert_eq!(edges, 1);
    }

    #[test]
    fn rebuild_is_idempotent() {
        let idx = Index::open_in_memory().unwrap();
        let n = parse_str("a", "본문").unwrap();
        idx.rebuild(&[n.clone()]).unwrap();
        idx.rebuild(&[n]).unwrap();
        let notes: i64 = idx.conn.query_row("SELECT count(*) FROM notes", [], |r| r.get(0)).unwrap();
        assert_eq!(notes, 1);
    }
```

- [ ] **Step 2: 테스트 실패 확인**

Run: `cargo test -p pack-core index::tests::rebuild_inserts_notes_and_edges`
Expected: `rebuild` 미정의로 실패.

- [ ] **Step 3: 구현 추가**

`index.rs`에 `use crate::note::Note;` 추가하고 `impl Index`에:
```rust
    /// 모든 테이블을 비우고 notes를 다시 채운다(전체 재빌드).
    pub fn rebuild(&self, notes: &[Note]) -> Result<()> {
        let conn = &self.conn;
        conn.execute_batch("DELETE FROM notes; DELETE FROM notes_fts; DELETE FROM edges;")?;
        let tx = conn.unchecked_transaction()?;
        {
            let mut ins = tx.prepare(
                "INSERT INTO notes (id, path, type, title, tags, created, asset, body, mtime)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            )?;
            let mut ins_fts =
                tx.prepare("INSERT INTO notes_fts (id, title, body, tags) VALUES (?1, ?2, ?3, ?4)")?;
            let mut ins_edge =
                tx.prepare("INSERT OR IGNORE INTO edges (src, dst, kind) VALUES (?1, ?2, ?3)")?;
            for n in notes {
                let tags_json = serde_json::to_string(&n.tags)?;
                let tags_text = n.tags.join(" ");
                ins.execute(rusqlite::params![
                    n.id,
                    n.path.to_string_lossy(),
                    n.note_type,
                    n.title,
                    tags_json,
                    n.created,
                    n.asset,
                    n.body,
                    n.mtime,
                ])?;
                ins_fts.execute(rusqlite::params![n.id, n.title, n.body, tags_text])?;
                for dst in &n.related {
                    ins_edge.execute(rusqlite::params![n.id, dst, "related"])?;
                }
            }
        }
        tx.commit()?;
        Ok(())
    }
```

(테스트에서 `n.clone()`을 쓰므로 `Note`에 `#[derive(Clone)]`이 있어야 한다 — Task 2에서 이미 `#[derive(Debug, Clone)]` 적용됨. serde_json은 `tags: Vec<String>`을 직렬화하므로 추가 의존성 불필요.)

- [ ] **Step 4: 테스트 통과 확인**

Run: `cargo test -p pack-core index::tests`
Expected: 3개 모두 PASS.

- [ ] **Step 5: 커밋**

```bash
git add crates/pack-core/src/index.rs
git commit -m "feat(core): Index.rebuild — notes/fts/edges 전체 재빌드"
```

---

## Task 8: 키워드 검색 (FTS5 BM25)

**Files:**
- Modify: `crates/pack-core/src/search.rs`
- Modify: `crates/pack-core/src/index.rs` (검색 메서드)

- [ ] **Step 1: 실패하는 테스트 작성**

`crates/pack-core/src/search.rs`:
```rust
#[cfg(test)]
mod tests {
    use crate::index::Index;
    use crate::note::parse_str;

    #[test]
    fn keyword_search_ranks_match() {
        let idx = Index::open_in_memory().unwrap();
        let a = parse_str("a", "---\ntitle: 고래\n---\n바다 고래 이야기").unwrap();
        let b = parse_str("b", "---\ntitle: 자동차\n---\n도로 자동차 이야기").unwrap();
        idx.rebuild(&[a, b]).unwrap();

        let hits = idx.search_keyword("고래", 10).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].id, "a");
        assert_eq!(hits[0].title, "고래");

        let none = idx.search_keyword("비행기", 10).unwrap();
        assert!(none.is_empty());
    }
}
```

- [ ] **Step 2: 테스트 실패 확인**

Run: `cargo test -p pack-core search::tests`
Expected: `search_keyword`/`NoteHit` 미정의로 실패.

- [ ] **Step 3: 구현 작성**

`crates/pack-core/src/search.rs`:
```rust
#[derive(Debug, Clone, PartialEq)]
pub struct NoteHit {
    pub id: String,
    pub title: String,
    pub note_type: String,
    pub score: f64, // bm25: 낮을수록 관련도 높음
}
```

`crates/pack-core/src/index.rs`의 `impl Index`에 추가:
```rust
    /// FTS5 키워드 검색. 결과는 bm25 점수 오름차순(관련도 높은 순).
    pub fn search_keyword(&self, query: &str, k: usize) -> Result<Vec<crate::search::NoteHit>> {
        // 사용자 입력을 FTS5 구문에서 안전하게: 공백 단위 토큰을 "..." 로 감싸 OR 결합
        let safe = sanitize_fts_query(query);
        if safe.is_empty() {
            return Ok(Vec::new());
        }
        let mut stmt = self.conn.prepare(
            "SELECT n.id, n.title, n.type, bm25(notes_fts) AS score
             FROM notes_fts JOIN notes n ON n.id = notes_fts.id
             WHERE notes_fts MATCH ?1
             ORDER BY score
             LIMIT ?2",
        )?;
        let rows = stmt.query_map(rusqlite::params![safe, k as i64], |r| {
            Ok(crate::search::NoteHit {
                id: r.get(0)?,
                title: r.get(1)?,
                note_type: r.get(2)?,
                score: r.get(3)?,
            })
        })?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }
```

`index.rs` 하단(또는 모듈 내)에 헬퍼:
```rust
/// 사용자 질의를 FTS5 MATCH에 넣기 안전하게 변환: 토큰을 "..."로 감싸 OR.
fn sanitize_fts_query(query: &str) -> String {
    query
        .split_whitespace()
        .map(|t| format!("\"{}\"", t.replace('"', "")))
        .collect::<Vec<_>>()
        .join(" OR ")
}
```

- [ ] **Step 4: 테스트 통과 확인**

Run: `cargo test -p pack-core`
Expected: 모든 pack-core 테스트 PASS.

- [ ] **Step 5: 커밋**

```bash
git add crates/pack-core/src/search.rs crates/pack-core/src/index.rs
git commit -m "feat(core): FTS5 키워드 검색(search_keyword) + NoteHit"
```

---

## Task 9: CLI — clap 골격 + `init`

**Files:**
- Modify: `crates/pack-cli/src/main.rs`

- [ ] **Step 1: 실패하는 통합 테스트 작성**

`crates/pack-cli/tests/cli.rs` 생성:
```rust
use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::tempdir;

#[test]
fn init_creates_pack() {
    let dir = tempdir().unwrap();
    let root = dir.path().join("p");
    Command::cargo_bin("pack")
        .unwrap()
        .args(["init", root.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("초기화"));
    assert!(root.join("pack.toml").exists());
    assert!(root.join("notes").is_dir());
}
```

- [ ] **Step 2: 테스트 실패 확인**

Run: `cargo test -p pack-cli`
Expected: 실패(현재 main은 "pack"만 출력, init 미구현).

- [ ] **Step 3: 구현 작성**

`crates/pack-cli/src/main.rs` 전체 교체:
```rust
use anyhow::Result;
use clap::{Parser, Subcommand};
use pack_core::pack::{find_pack_root, Pack};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "pack", about = "ontopack — 로컬 지식 팩 CLI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// 새 팩 골격을 만든다
    Init {
        /// 팩 경로 (기본: 현재 디렉터리)
        path: Option<PathBuf>,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Init { path } => {
            let root = path.unwrap_or_else(|| PathBuf::from("."));
            let name = root
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("my-pack")
                .to_string();
            Pack::init(&root, &name)?;
            println!("팩 초기화 완료: {}", root.display());
        }
    }
    Ok(())
}
```

(`find_pack_root` import는 다음 태스크에서 사용하므로 미리 둔다. 미사용 경고가 거슬리면 다음 태스크까지 `#[allow(unused_imports)]` 없이 두고 진행해도 됨.)

- [ ] **Step 4: 테스트 통과 확인**

Run: `cargo test -p pack-cli`
Expected: `init_creates_pack` PASS.

- [ ] **Step 5: 커밋**

```bash
git add crates/pack-cli/src/main.rs crates/pack-cli/tests/cli.rs
git commit -m "feat(cli): clap 골격 + pack init"
```

---

## Task 10: CLI — `add`

**Files:**
- Modify: `crates/pack-cli/src/main.rs`

> M1의 `add`는 최소 흐름: 텍스트/마크다운 → `notes/`에 노트로, 그 외 파일 → `assets/`에 복사 + `notes/`에 사이드카 노트 스텁 생성. (`_inbox`/`process`는 M2.)

- [ ] **Step 1: 실패하는 통합 테스트 추가**

`crates/pack-cli/tests/cli.rs`에 추가:
```rust
#[test]
fn add_markdown_creates_note() {
    let dir = tempdir().unwrap();
    let root = dir.path().join("p");
    Command::cargo_bin("pack").unwrap()
        .args(["init", root.to_str().unwrap()]).assert().success();

    let src = dir.path().join("hello.md");
    std::fs::write(&src, "---\ntype: prompt\ntitle: 헬로\n---\n본문").unwrap();

    Command::cargo_bin("pack").unwrap()
        .current_dir(&root)
        .args(["add", src.to_str().unwrap()])
        .assert().success();

    assert!(root.join("notes/hello.md").exists());
}

#[test]
fn add_binary_creates_asset_and_sidecar() {
    let dir = tempdir().unwrap();
    let root = dir.path().join("p");
    Command::cargo_bin("pack").unwrap()
        .args(["init", root.to_str().unwrap()]).assert().success();

    let img = dir.path().join("pic.png");
    std::fs::write(&img, [0x89, 0x50, 0x4e, 0x47]).unwrap(); // 가짜 PNG 헤더

    Command::cargo_bin("pack").unwrap()
        .current_dir(&root)
        .args(["add", img.to_str().unwrap(), "--type", "image"])
        .assert().success();

    assert!(root.join("assets/pic.png").exists());
    let sidecar = std::fs::read_to_string(root.join("notes/pic.md")).unwrap();
    assert!(sidecar.contains("type: image"));
    assert!(sidecar.contains("asset: assets/pic.png"));
}
```

- [ ] **Step 2: 테스트 실패 확인**

Run: `cargo test -p pack-cli`
Expected: add 관련 2개 실패(서브커맨드 없음).

- [ ] **Step 3: 구현 작성**

`main.rs`의 `Commands` enum에 추가:
```rust
    /// 파일을 팩에 추가한다 (md→notes/, 그 외→assets/+사이드카)
    Add {
        /// 추가할 파일 경로
        file: PathBuf,
        /// 개체 타입 (기본: note)
        #[arg(long, default_value = "note")]
        r#type: String,
    },
```

`main` match에 분기 추가:
```rust
        Commands::Add { file, r#type } => {
            let root = find_pack_root(&std::env::current_dir()?)?;
            let stem = file.file_stem().and_then(|s| s.to_str()).unwrap_or("untitled");
            let ext = file.extension().and_then(|s| s.to_str()).unwrap_or("");
            if ext == "md" || ext == "markdown" {
                let dst = root.join("notes").join(format!("{stem}.md"));
                std::fs::copy(&file, &dst)?;
                println!("노트 추가: {}", dst.display());
            } else {
                let file_name = file.file_name().unwrap().to_string_lossy().to_string();
                let asset_rel = format!("assets/{file_name}");
                std::fs::copy(&file, root.join(&asset_rel))?;
                let note_path = root.join("notes").join(format!("{stem}.md"));
                let body = format!(
                    "---\ntype: {ty}\ntitle: {stem}\nasset: {asset_rel}\ntags: []\n---\n캡션을 적어주세요(검색 대상).\n",
                    ty = r#type
                );
                std::fs::write(&note_path, body)?;
                println!("자산+사이드카 추가: {}", note_path.display());
            }
        }
```

- [ ] **Step 4: 테스트 통과 확인**

Run: `cargo test -p pack-cli`
Expected: add 2개 PASS.

- [ ] **Step 5: 커밋**

```bash
git add crates/pack-cli/src/main.rs crates/pack-cli/tests/cli.rs
git commit -m "feat(cli): pack add (md→notes, 그외→assets+사이드카)"
```

---

## Task 11: CLI — `build` + `search`

**Files:**
- Modify: `crates/pack-cli/src/main.rs`

- [ ] **Step 1: 실패하는 통합 테스트(엔드투엔드) 추가**

`crates/pack-cli/tests/cli.rs`에 추가:
```rust
#[test]
fn end_to_end_build_and_search() {
    let dir = tempdir().unwrap();
    let root = dir.path().join("p");
    Command::cargo_bin("pack").unwrap()
        .args(["init", root.to_str().unwrap()]).assert().success();

    std::fs::write(root.join("notes/whale.md"),
        "---\ntype: prompt\ntitle: 고래\ntags: [sea]\n---\n바다 고래 이야기").unwrap();
    std::fs::write(root.join("notes/car.md"),
        "---\ntype: prompt\ntitle: 자동차\n---\n도로 자동차 이야기").unwrap();

    Command::cargo_bin("pack").unwrap()
        .current_dir(&root).args(["build"])
        .assert().success()
        .stdout(predicate::str::contains("2"));

    Command::cargo_bin("pack").unwrap()
        .current_dir(&root).args(["search", "고래"])
        .assert().success()
        .stdout(predicate::str::contains("고래"))
        .stdout(predicate::str::contains("자동차").not());
}
```

- [ ] **Step 2: 테스트 실패 확인**

Run: `cargo test -p pack-cli end_to_end_build_and_search`
Expected: 실패(build/search 미구현).

- [ ] **Step 3: 구현 작성**

`main.rs` import에 추가: `use pack_core::index::Index;`

`Commands` enum에 추가:
```rust
    /// 인덱스를 (재)빌드한다
    Build,
    /// 키워드 검색
    Search {
        /// 검색어
        query: String,
        /// 최대 결과 수
        #[arg(short, default_value_t = 10)]
        k: usize,
    },
```

`main` match에 분기 추가:
```rust
        Commands::Build => {
            let root = find_pack_root(&std::env::current_dir()?)?;
            let pack = Pack::open(&root)?;
            let notes = pack.scan_notes()?;
            let idx = Index::open(&pack.index_path())?;
            idx.rebuild(&notes)?;
            println!("인덱스 빌드 완료: 노트 {}개", notes.len());
        }
        Commands::Search { query, k } => {
            let root = find_pack_root(&std::env::current_dir()?)?;
            let pack = Pack::open(&root)?;
            let idx = Index::open(&pack.index_path())?;
            let hits = idx.search_keyword(&query, k)?;
            if hits.is_empty() {
                println!("(결과 없음)");
            }
            for h in hits {
                println!("[{}] {}  ({})", h.note_type, h.title, h.id);
            }
        }
```

- [ ] **Step 4: 테스트 통과 확인**

Run: `cargo test -p pack-cli`
Expected: 모든 CLI 테스트 PASS.

- [ ] **Step 5: 전체 테스트 + 빌드**

Run: `cargo test && cargo build --release`
Expected: 전부 PASS, 릴리스 바이너리 `target/release/pack` 생성.

- [ ] **Step 6: 커밋**

```bash
git add crates/pack-cli/src/main.rs crates/pack-cli/tests/cli.rs
git commit -m "feat(cli): pack build + search (엔드투엔드 검색 동작)"
```

---

## Task 12: 수동 스모크 테스트 + README 시드

**Files:**
- Create: `README.md`

- [ ] **Step 1: 실제 팩으로 손맛 확인 (수동)**

```bash
cd /tmp
./<repo>/target/release/pack init /tmp/smoke-pack
cd /tmp/smoke-pack
printf -- "---\ntype: prompt\ntitle: 썸네일 훅\ntags: [thumbnail]\n---\n클릭을 부르는 훅 카피.\n" > notes/hook.md
pack build      # 또는 절대경로 바이너리
pack search 훅
```
Expected: `[prompt] 썸네일 훅 (hook)` 출력.

- [ ] **Step 2: README 시드 작성**

`README.md`:
```markdown
# ontopack

로컬 멀티모달 지식 팩 엔진. 평문 파일이 진실, SQLite+FTS5가 빠른 인덱스.

## M1 (현재)
- `pack init [경로]` — 새 팩
- `pack add <파일> [--type T]` — md→notes/, 그 외→assets/+사이드카
- `pack build` — 인덱스 (재)빌드
- `pack search "<질의>"` — 키워드(BM25) 검색

## 다음 (M2~)
임베딩(BGE-M3)+sqlite-vec+하이브리드/RRF, MCP 서버, 위키 뷰어.
```

- [ ] **Step 3: 커밋**

```bash
git add README.md
git commit -m "docs: README 시드 + M1 스모크 확인"
```

---

## Self-Review (작성자 체크)

**1. 스펙 커버리지 (M1 범위 한정):**
- 평문 파일=진실 / 인덱스=파생 → Task 6~7(전체 재빌드로 재생성 가능) ✓
- 노트=개체 frontmatter 스키마(type/title/tags/created/asset/related) → Task 2 ✓
- 관계=온톨로지 엣지(위키링크) → Task 3, 7(edges) ✓
- 사이드카 멀티모달 → Task 10(add 바이너리) ✓
- SQLite+FTS5 인덱스 → Task 6 ✓
- 키워드(BM25) 검색 → Task 8 ✓
- CLI(init/add/build/search) → Task 9~11 ✓
- (M1 범위 밖, 의도적 제외) 임베딩/벡터/RRF=M2, `_inbox`/`process`·증분 인덱싱=M2, MCP=M3, 뷰어=M4 ✓ 명시됨

**2. 플레이스홀더 스캔:** "TBD/적절히 처리" 없음. 모든 코드 단계에 실제 코드 포함 ✓

**3. 타입 일관성:**
- `Note`(note.rs) 필드 = index.rebuild 삽입 컬럼 = 일치 ✓
- `parse_str`/`parse_file`/`extract_wikilinks`(note.rs) — Task 2/3에서 정의, 이후 사용 ✓
- `Pack::{init,open,scan_notes,index_path}` + `find_pack_root`(pack.rs) — Task 5 정의, Task 11 사용 ✓
- `Index::{open,open_in_memory,rebuild,search_keyword}`(index.rs) — Task 6/7/8 정의, Task 11 사용 ✓
- `NoteHit{id,title,note_type,score}`(search.rs) — Task 8 정의, search_keyword 반환 ✓
- `PackConfig::{load,default_toml}`(config.rs) — Task 4 정의, Pack에서 사용 ✓

이상 없음.
