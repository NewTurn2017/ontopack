#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TMPDIR="${TMPDIR:-/tmp}"
PACK_DIR="$(mktemp -d "$TMPDIR/ontopack-mvp-smoke.XXXXXX")"
cleanup() {
  rm -rf "$PACK_DIR"
}
trap cleanup EXIT

PACK_BIN="${PACK_BIN:-$ROOT/target/debug/pack}"
MCP_BIN="${MCP_BIN:-$ROOT/target/debug/pack-mcp}"

echo "[1/8] build debug binaries"
cargo build --quiet -p pack-cli -p pack-mcp --manifest-path "$ROOT/Cargo.toml"

echo "[2/8] init temp pack: $PACK_DIR"
"$PACK_BIN" init "$PACK_DIR" >/tmp/ontopack-smoke-init.out

cat > "$PACK_DIR/_inbox/hook.md" <<'NOTE'
---
type: prompt
title: 썸네일 훅
tags: [youtube, hook]
created: 2026-03-02
related: [gallery-pic]
---
클릭을 부르는 훅 카피와 강의 오프닝 구조.
NOTE

printf '\x89PNG\r\n' > "$PACK_DIR/gallery.png"
(cd "$PACK_DIR" && "$PACK_BIN" add "$PACK_DIR/gallery.png" --type image >/tmp/ontopack-smoke-add.out)
cat > "$PACK_DIR/notes/gallery.md.tmp" <<'NOTE'
---
type: image
title: 갤러리 이미지
tags: [gallery]
created: 2026-03-01
asset: assets/gallery.png
---
훅 이미지 카드.
NOTE
mv "$PACK_DIR/notes/gallery.md.tmp" "$PACK_DIR/notes/gallery-pic.md"

echo "[3/8] process inbox + build index"
(cd "$PACK_DIR" && "$PACK_BIN" process >/tmp/ontopack-smoke-process.out)
(cd "$PACK_DIR" && "$PACK_BIN" build --incremental >/tmp/ontopack-smoke-build.out)

echo "[4/8] CLI search source card"
CLI_SEARCH="$(cd "$PACK_DIR" && "$PACK_BIN" search "훅" --mode keyword)"
printf '%s\n' "$CLI_SEARCH" | grep -q '\[keyword\]'
printf '%s\n' "$CLI_SEARCH" | grep -q 'hook#0000'

echo "[5/8] MCP initialize + tools/list + search + ask"
MCP_OUT="$(printf '%s\n' \
  '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25"}}' \
  '{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}' \
  '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"search","arguments":{"query":"훅","k":3}}}' \
  '{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"ask","arguments":{"question":"훅 자료?","k":3}}}' \
  | "$MCP_BIN" --pack-root "$PACK_DIR")"
printf '%s\n' "$MCP_OUT" | grep -q '"serverInfo"'
printf '%s\n' "$MCP_OUT" | grep -q 'name.*search'
printf '%s\n' "$MCP_OUT" | grep -q 'context_blocks'

echo "[6/8] viewer API /api/search"
SEARCH_JSON="$(cd "$PACK_DIR" && "$PACK_BIN" serve --port 0 --once --request $'GET /api/search?q=%ED%9B%85&type=prompt&tag=hook HTTP/1.1\r\nHost: localhost\r\n\r\n')"
printf '%s\n' "$SEARCH_JSON" | grep -q '"note_id":"hook"'

echo "[7/8] viewer API /api/ask + /api/facets"
ASK_JSON="$(cd "$PACK_DIR" && "$PACK_BIN" serve --port 0 --once --request $'GET /api/ask?q=%ED%9B%85&k=3 HTTP/1.1\r\nHost: localhost\r\n\r\n')"
printf '%s\n' "$ASK_JSON" | grep -q '"answer_mode":"external_llm_required"'
FACETS_JSON="$(cd "$PACK_DIR" && "$PACK_BIN" serve --port 0 --once --request $'GET /api/facets HTTP/1.1\r\nHost: localhost\r\n\r\n')"
printf '%s\n' "$FACETS_JSON" | grep -q '"types"'
printf '%s\n' "$FACETS_JSON" | grep -q 'hook'

echo "[8/8] viewer API /api/gallery + notes/related/timeline/graph + open URL smoke"
GALLERY_JSON="$(cd "$PACK_DIR" && "$PACK_BIN" serve --port 0 --once --request $'GET /api/gallery?k=5 HTTP/1.1\r\nHost: localhost\r\n\r\n')"
printf '%s\n' "$GALLERY_JSON" | grep -q '"asset":"assets/gallery.png"'
NOTE_JSON="$(cd "$PACK_DIR" && "$PACK_BIN" serve --port 0 --once --request $'GET /api/notes/hook HTTP/1.1\r\nHost: localhost\r\n\r\n')"
printf '%s\n' "$NOTE_JSON" | grep -q '"id":"hook"'
RELATED_JSON="$(cd "$PACK_DIR" && "$PACK_BIN" serve --port 0 --once --request $'GET /api/related/hook?depth=1 HTTP/1.1\r\nHost: localhost\r\n\r\n')"
printf '%s\n' "$RELATED_JSON" | grep -q 'gallery-pic'
TIMELINE_JSON="$(cd "$PACK_DIR" && "$PACK_BIN" serve --port 0 --once --request $'GET /api/timeline?k=5 HTTP/1.1\r\nHost: localhost\r\n\r\n')"
printf '%s\n' "$TIMELINE_JSON" | grep -q '"notes"'
GRAPH_JSON="$(cd "$PACK_DIR" && "$PACK_BIN" serve --port 0 --once --request $'GET /api/graph?limit=5 HTTP/1.1\r\nHost: localhost\r\n\r\n')"
printf '%s\n' "$GRAPH_JSON" | grep -q '"edges"'
OPEN_URL="$(cd "$PACK_DIR" && "$PACK_BIN" open --port 0 --no-browser --print-url)"
printf '%s\n' "$OPEN_URL" | grep -q '^http://127\.0\.0\.1:'

echo "MVP smoke passed: CLI + MCP + viewer API + open URL"
