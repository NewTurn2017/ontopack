# ontopack M4 Serve/Open Viewer Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking. **Iron law:** no production code without first watching the matching test fail.

**Goal:** Add a local human-facing wiki viewer over the existing pack-core APIs so `pack serve` and `pack open` expose search, notes, related notes, timeline, and a lightweight graph in the browser.

**Architecture:** `pack-core` remains the source of deterministic pack behavior. A new `pack-server` crate owns a tiny localhost HTTP server and embedded static viewer assets; `pack-cli` only parses `serve`/`open` flags and delegates to `pack-server`. The first viewer is framework-free HTML/CSS/JS embedded at compile time, with JSON APIs designed for tests and future replacement.

**Tech Stack:** Rust std `TcpListener`/`TcpStream`, `serde`/`serde_json`, existing `pack-core`, `clap`, `assert_cmd`/`tempfile` for CLI tests. No heavy web framework or frontend build step.

---

## File Structure

- Modify `Cargo.toml`: add `crates/pack-server` workspace member.
- Create `crates/pack-server/Cargo.toml`: depends on `pack-core`, `anyhow`, `serde`, `serde_json`.
- Create `crates/pack-server/src/lib.rs`: exports `api`, `http`, and `viewer` modules plus `serve_forever`/`serve_once` entrypoints.
- Create `crates/pack-server/src/api.rs`: JSON response models and pure handlers for `search`, `note`, `related`, `timeline`, and `graph`.
- Create `crates/pack-server/src/http.rs`: small HTTP parser/router over localhost `TcpListener`; route JSON APIs and viewer assets.
- Create `crates/pack-server/src/viewer.rs`: embedded `index.html`, `app.js`, and `style.css` strings.
- Modify `crates/pack-cli/Cargo.toml`: add `pack-server` dependency.
- Modify `crates/pack-cli/src/main.rs`: add `serve` and `open` subcommands.
- Modify `crates/pack-cli/tests/cli.rs`: add CLI smoke for `serve --once` and `open --print-url --no-browser`.
- Create `docs/viewer.md`: viewer usage and API contract.
- Modify `README.md`: document M4 commands.

---

## Task 1: pack-server crate + pure API contracts

**Files:**
- Modify: `Cargo.toml`
- Create: `crates/pack-server/Cargo.toml`
- Create: `crates/pack-server/src/lib.rs`
- Create: `crates/pack-server/src/api.rs`

- [ ] **Step 1: Write failing API tests**

Add tests in `crates/pack-server/src/api.rs`:

```rust
#[test]
fn search_api_returns_source_cards() {
    let dir = tempdir().unwrap();
    let root = dir.path().join("p");
    Pack::init(&root, "p").unwrap();
    std::fs::write(
        root.join("notes/hook.md"),
        "---\ntype: prompt\ntitle: 썸네일 훅\ntags: [youtube]\ncreated: 2026-02-01\n---\n클릭을 부르는 훅 카피.",
    )
    .unwrap();
    let pack = Pack::open(&root).unwrap();
    pack.build_index().unwrap();

    let response = search(&pack, "훅", None, 10).unwrap();
    assert_eq!(response.query, "훅");
    assert_eq!(response.hits[0].note_id, "hook");
    assert_eq!(response.hits[0].chunk_id, "hook#0000");
    assert_eq!(response.hits[0].rank_source, "keyword");
}

#[test]
fn note_api_returns_note_detail() {
    let dir = tempdir().unwrap();
    let root = dir.path().join("p");
    Pack::init(&root, "p").unwrap();
    std::fs::write(
        root.join("notes/a.md"),
        "---\ntype: note\ntitle: A\ntags: [x]\ncreated: 2026-01-01\n---\n본문 [[b]]",
    )
    .unwrap();
    let pack = Pack::open(&root).unwrap();

    let note = note(&pack, "a").unwrap();
    assert_eq!(note.id, "a");
    assert_eq!(note.title, "A");
    assert_eq!(note.tags, vec!["x"]);
    assert_eq!(note.related, vec!["b"]);
    assert!(note.body.contains("본문"));
}

#[test]
fn graph_api_returns_nodes_and_edges() {
    let dir = tempdir().unwrap();
    let root = dir.path().join("p");
    Pack::init(&root, "p").unwrap();
    std::fs::write(root.join("notes/a.md"), "A [[b]]").unwrap();
    std::fs::write(root.join("notes/b.md"), "B").unwrap();
    let pack = Pack::open(&root).unwrap();

    let graph = graph(&pack, None, 50).unwrap();
    assert_eq!(graph.nodes.len(), 2);
    assert_eq!(graph.edges[0].from, "a");
    assert_eq!(graph.edges[0].to, "b");
}
```

- [ ] **Step 2: Run RED**

```bash
cargo test -p pack-server search_api_returns_source_cards
```

Expected: fail because `pack-server` crate and API functions do not exist.

- [ ] **Step 3: Implement minimal API models and handlers**

Implement serializable response structs:

```rust
#[derive(Debug, Serialize, PartialEq)]
pub struct SearchResponse { pub query: String, pub hits: Vec<SearchCard> }
#[derive(Debug, Serialize, PartialEq)]
pub struct SearchCard { pub note_id: String, pub chunk_id: String, pub title: String, pub note_type: String, pub snippet: String, pub path: String, pub score: f64, pub rank_source: String }
#[derive(Debug, Serialize, PartialEq)]
pub struct NoteDetail { pub id: String, pub title: String, pub note_type: String, pub tags: Vec<String>, pub created: Option<String>, pub asset: Option<String>, pub related: Vec<String>, pub body: String, pub path: String }
#[derive(Debug, Serialize, PartialEq)]
pub struct GraphResponse { pub nodes: Vec<GraphNode>, pub edges: Vec<GraphEdge> }
```

Use `Pack::search_keyword_chunks`, `Pack::scan_notes`, `Pack::related_notes`, and `Pack::timeline_notes`. Return `anyhow::bail!("note not found: {id}")` for missing notes.

- [ ] **Step 4: Verify GREEN and commit**

```bash
cargo test -p pack-server
cargo test -p pack-core
cargo fmt --check
git add Cargo.toml Cargo.lock crates/pack-server
git commit -m "Add viewer API contracts"
```

---

## Task 2: HTTP router + API endpoint smoke

**Files:**
- Modify: `crates/pack-server/src/lib.rs`
- Create: `crates/pack-server/src/http.rs`

- [ ] **Step 1: Write failing HTTP tests**

Add tests in `crates/pack-server/src/http.rs`:

```rust
#[test]
fn api_search_http_returns_json() {
    let dir = tempdir().unwrap();
    let root = dir.path().join("p");
    Pack::init(&root, "p").unwrap();
    std::fs::write(root.join("notes/hook.md"), "---\ntitle: 훅\n---\n클릭 훅").unwrap();
    let pack = Pack::open(&root).unwrap();
    pack.build_index().unwrap();

    let response = handle_request(&pack, "GET /api/search?q=%ED%9B%85&k=5 HTTP/1.1\r\nHost: localhost\r\n\r\n").unwrap();
    assert_eq!(response.status, 200);
    assert_eq!(response.content_type, "application/json; charset=utf-8");
    let body: serde_json::Value = serde_json::from_slice(&response.body).unwrap();
    assert_eq!(body["hits"][0]["note_id"], "hook");
}

#[test]
fn api_note_http_returns_404_for_missing_note() {
    let dir = tempdir().unwrap();
    let root = dir.path().join("p");
    Pack::init(&root, "p").unwrap();
    let pack = Pack::open(&root).unwrap();

    let response = handle_request(&pack, "GET /api/notes/missing HTTP/1.1\r\nHost: localhost\r\n\r\n").unwrap();
    assert_eq!(response.status, 404);
    let body: serde_json::Value = serde_json::from_slice(&response.body).unwrap();
    assert!(body["error"].as_str().unwrap().contains("note not found"));
}
```

- [ ] **Step 2: Run RED**

```bash
cargo test -p pack-server api_search_http_returns_json
```

Expected: fail because HTTP module does not exist.

- [ ] **Step 3: Implement minimal router**

Implement:
- `HttpResponse { status, content_type, body }`
- `handle_request(pack, raw_request)` for `GET` requests only.
- Routes:
  - `/api/search?q=<query>&k=<n>&type=<type>`
  - `/api/notes/<id>`
  - `/api/related/<id>?depth=<n>`
  - `/api/timeline?from=&to=&type=&k=`
  - `/api/graph?type=&limit=`
- Percent-decode query/path values without adding a dependency.
- JSON error body: `{ "error": "..." }`.

- [ ] **Step 4: Verify GREEN and commit**

```bash
cargo test -p pack-server
cargo fmt --check
git add crates/pack-server/src/lib.rs crates/pack-server/src/http.rs
git commit -m "Serve viewer JSON APIs over HTTP"
```

---

## Task 3: Embedded static viewer assets

**Files:**
- Create: `crates/pack-server/src/viewer.rs`
- Modify: `crates/pack-server/src/http.rs`

- [ ] **Step 1: Write failing viewer asset tests**

Add tests in `http.rs`:

```rust
#[test]
fn serves_static_viewer_shell() {
    let dir = tempdir().unwrap();
    let root = dir.path().join("p");
    Pack::init(&root, "p").unwrap();
    let pack = Pack::open(&root).unwrap();

    let response = handle_request(&pack, "GET / HTTP/1.1\r\nHost: localhost\r\n\r\n").unwrap();
    assert_eq!(response.status, 200);
    assert_eq!(response.content_type, "text/html; charset=utf-8");
    let html = String::from_utf8(response.body).unwrap();
    assert!(html.contains("ontopack"));
    assert!(html.contains("/app.js"));
}
```

- [ ] **Step 2: Run RED**

```bash
cargo test -p pack-server serves_static_viewer_shell
```

Expected: fail because `/` is not served.

- [ ] **Step 3: Implement viewer assets**

Embed three asset functions:
- `index_html()` — Korean-first shell with search input, results, note detail, timeline, graph sections.
- `app_js()` — calls `/api/search`, `/api/notes/:id`, `/api/related/:id`, `/api/timeline`, `/api/graph`; renders clickable source cards and note detail.
- `style_css()` — readable Korean body text at 15px+ with modest layout and no framework.

- [ ] **Step 4: Verify GREEN and commit**

```bash
cargo test -p pack-server
cargo fmt --check
git add crates/pack-server/src/viewer.rs crates/pack-server/src/http.rs
git commit -m "Embed local wiki viewer shell"
```

---

## Task 4: CLI serve/open commands

**Files:**
- Modify: `crates/pack-cli/Cargo.toml`
- Modify: `crates/pack-cli/src/main.rs`
- Modify: `crates/pack-cli/tests/cli.rs`

- [ ] **Step 1: Write failing CLI tests**

Add tests:

```rust
#[test]
fn serve_once_prints_local_url_and_handles_one_request() {
    let dir = tempdir().unwrap();
    let root = dir.path().join("p");
    Command::cargo_bin("pack").unwrap().args(["init", root.to_str().unwrap()]).assert().success();
    std::fs::write(root.join("notes/a.md"), "hello").unwrap();
    Command::cargo_bin("pack").unwrap().current_dir(&root).args(["build"]).assert().success();

    Command::cargo_bin("pack")
        .unwrap()
        .current_dir(&root)
        .args(["serve", "--port", "0", "--once", "--request", "GET /api/search?q=hello HTTP/1.1\\r\\nHost: localhost\\r\\n\\r\\n"])
        .assert()
        .success()
        .stdout(predicate::str::contains("http://127.0.0.1:"))
        .stdout(predicate::str::contains("\"note_id\":\"a\""));
}

#[test]
fn open_no_browser_prints_viewer_url() {
    let dir = tempdir().unwrap();
    let root = dir.path().join("p");
    Command::cargo_bin("pack").unwrap().args(["init", root.to_str().unwrap()]).assert().success();

    Command::cargo_bin("pack")
        .unwrap()
        .current_dir(&root)
        .args(["open", "--port", "0", "--no-browser", "--print-url"])
        .assert()
        .success()
        .stdout(predicate::str::contains("http://127.0.0.1:"));
}
```

- [ ] **Step 2: Run RED**

```bash
cargo test -p pack-cli serve_once_prints_local_url_and_handles_one_request
```

Expected: fail because `serve` command does not exist.

- [ ] **Step 3: Implement CLI delegation**

Add subcommands:
- `pack serve --port <u16> [--once --request <raw-http-request>]`
- `pack open --port <u16> [--no-browser] [--print-url]`

For normal `serve`, bind localhost and serve forever. For `--once --request`, use the pure HTTP router for deterministic CLI testing and print the URL plus raw HTTP response body.

For `open`, bind a local URL. With `--no-browser --print-url`, print URL and exit for tests. Without `--no-browser`, call `open` on macOS, `xdg-open` on Linux, or `cmd /C start` on Windows, then serve forever.

- [ ] **Step 4: Verify GREEN and commit**

```bash
cargo test -p pack-cli serve_once_prints_local_url_and_handles_one_request
cargo test -p pack-cli open_no_browser_prints_viewer_url
cargo test -p pack-cli
cargo fmt --check
git add crates/pack-cli/Cargo.toml crates/pack-cli/src/main.rs crates/pack-cli/tests/cli.rs
git commit -m "Add pack serve and open commands"
```

---

## Task 5: Docs, full validation, and browser-equivalent smoke

**Files:**
- Create: `docs/viewer.md`
- Modify: `README.md`

- [ ] **Step 1: Write docs**

Document:
- `pack serve --port 8787`
- `pack open`
- API endpoints and example JSON.
- Viewer limitations: keyword search only by default; vector/hybrid remains CLI/real-embed path until wired into server.
- Browser smoke command using `pack serve --once --request`.

- [ ] **Step 2: Run full validation**

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

- [ ] **Step 3: Commit docs**

```bash
git add README.md docs/viewer.md
git commit -m "Document local viewer workflow"
```

---

## Final Gate

Before reporting complete:

```bash
cargo fmt --check
cargo test
cargo clippy --all-targets -- -D warnings
cargo check -p pack-core --features real-embed
cargo check -p pack-cli --features real-embed
cargo build --release
target/release/pack serve --port 0 --once --request $'GET /api/search?q=hello HTTP/1.1\r\nHost: localhost\r\n\r\n'
git diff --check
git status --short
```

Stop only when all commands pass and every task is committed.
