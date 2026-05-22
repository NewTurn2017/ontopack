# ontopack M3 MCP Server Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:test-driven-development` plus inline RED/GREEN execution. Steps use checkbox (`- [ ]`) syntax for tracking. **Iron law:** no production code without first watching the matching test fail.

**Goal:** Add an agent-first MCP stdio server exposing ontopack search/ask/related/add/timeline tools over stable JSON schemas.

**Architecture:** `pack-core` owns deterministic pack behavior. `pack-mcp` is a thin JSON-RPC/MCP stdio adapter with pure handler tests. The server implements core MCP methods manually (`initialize`, `notifications/initialized`, `tools/list`, `tools/call`) to avoid SDK churn; tool payloads return JSON text blocks for Claude/Codex adapters. LLM answer synthesis remains outside deterministic core: `ask` returns citation-ready context blocks.

**Tech Stack:** Rust, serde/serde_json/anyhow, clap, existing pack-core APIs. Tests use temp packs and direct JSON-RPC handler calls; no network/model downloads.

---

## File Structure

- Modify `Cargo.toml`: add `crates/pack-mcp` workspace member; add workspace `clap` already present.
- Create `crates/pack-mcp/Cargo.toml`.
- Create `crates/pack-mcp/src/main.rs`: stdio loop and CLI `--pack-root`.
- Create `crates/pack-mcp/src/lib.rs`: exports server module.
- Create `crates/pack-mcp/src/server.rs`: JSON-RPC/MCP handler and tool schemas.
- Modify `crates/pack-core/src/pack.rs`: add deterministic `related_notes`, `timeline_notes`, and `add_content_note` helpers.
- Create `docs/mcp.md`: install/config examples for Claude/Codex.
- Modify `README.md`: M3 section.

---

## Task 1: pack-mcp crate skeleton + MCP initialize/list

- [ ] Write failing tests for `initialize` and `tools/list` returning all five tool names.
- [ ] Run `cargo test -p pack-mcp initialize_and_lists_tools` and observe missing crate/failure.
- [ ] Create crate, server handler, and schemas.
- [ ] Verify `cargo test -p pack-mcp` and commit.

## Task 2: search and ask tools

- [ ] Write failing tests that build a temp pack and call `tools/call search` and `tools/call ask`.
- [ ] Implement search tool using `Pack::search_keyword_chunks`; ask returns context blocks from same search.
- [ ] Verify `cargo test -p pack-mcp search_tool_returns_source_cards ask_tool_returns_context_blocks` and commit.

## Task 3: related, timeline, add core behavior + tools

- [ ] Write failing pack-core tests for related/timeline/add_content_note.
- [ ] Implement pack-core helpers.
- [ ] Write failing pack-mcp tests for related/timeline/add.
- [ ] Implement tool calls.
- [ ] Verify `cargo test -p pack-core pack::tests::{...}` and `cargo test -p pack-mcp`, then commit.

## Task 4: stdio binary docs + smoke

- [ ] Write/verify a CLI smoke test or direct stdin smoke for `initialize`.
- [ ] Implement stdio loop and `--pack-root` CLI.
- [ ] Add `docs/mcp.md` with Claude/Codex config examples and README M3 notes.
- [ ] Verify full suite and commit.

## Final Ultragoal Gate

Run:

```bash
cargo fmt --check
cargo test
cargo clippy --all-targets -- -D warnings
cargo check -p pack-core --features real-embed
cargo check -p pack-cli --features real-embed
cargo build --release
cargo test -p pack-mcp
# smoke: echo initialize/tools/list/tools/call JSON into target/release/pack-mcp --pack-root <tmp-pack>
git diff --check
git status --short
```

Then run final ai-slop-cleaner and code-review gate before `update_goal`/ultragoal checkpoint.
