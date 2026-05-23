#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TMPDIR="${TMPDIR:-/tmp}"
PACK_DIR="${PACK_DIR:-$(mktemp -d "$TMPDIR/ontopack-real-test.XXXXXX")}" 
KEEP_REAL_TEST_PACK="${KEEP_REAL_TEST_PACK:-0}"
RUN_REAL_EMBED="${RUN_REAL_EMBED:-0}"
PACK_BIN="${PACK_BIN:-$ROOT/target/debug/pack}"
MCP_BIN="${MCP_BIN:-$ROOT/target/debug/pack-mcp}"

cleanup() {
  if [[ "$KEEP_REAL_TEST_PACK" != "1" ]]; then
    rm -rf "$PACK_DIR"
  else
    echo "real test pack kept: $PACK_DIR"
  fi
}
trap cleanup EXIT

json_last_line() {
  python3 -c 'import sys
lines=[line.strip() for line in sys.stdin if line.strip()]
for line in reversed(lines):
    if line.startswith("{") or line.startswith("["):
        print(line)
        raise SystemExit(0)
raise SystemExit("no JSON body found")'
}

assert_json() {
  local label="$1"
  local json="$2"
  local expr="$3"
  LABEL="$label" EXPR="$expr" python3 - "$json" <<'PY'
import json, os, sys
value = json.loads(sys.argv[1])
if not eval(os.environ["EXPR"], {}, {"v": value}):
    raise SystemExit(f"assertion failed: {os.environ['LABEL']} :: {os.environ['EXPR']}\n{json.dumps(value, ensure_ascii=False, indent=2)}")
PY
}

serve_json() {
  local request="$1"
  (cd "$PACK_DIR" && "$PACK_BIN" serve --port 0 --once --request "$request") | json_last_line
}

echo "[1/10] build debug binaries"
cargo build --quiet -p pack-cli -p pack-mcp --manifest-path "$ROOT/Cargo.toml"

echo "[2/10] seed realistic pack: $PACK_DIR"
"$PACK_BIN" init "$PACK_DIR" >/tmp/ontopack-real-init.out
mkdir -p "$PACK_DIR/_inbox" "$PACK_DIR/notes/lectures" "$PACK_DIR/notes/research"

cat > "$PACK_DIR/_inbox/lecture-outline.md" <<'NOTE'
---
type: lecture
title: 로컬 온톨로지 강의 설계
tags: [lecture, ontology, mvp]
created: 2026-05-20
related: [thumbnail-hook, evidence-image]
---
로컬 온톨로지는 사용자의 파일, 이미지 캡션, 강의 메모를 하나의 검색 가능한 지식팩으로 묶는다.
수업 흐름은 수집, 정리, 검색, 인용 가능한 답변 준비 순서로 진행한다.
NOTE

cat > "$PACK_DIR/_inbox/transcript.txt" <<'NOTE'
강의 녹취: 온톨로지 팩은 모델 다운로드 없이도 키워드 검색으로 먼저 검증한다.
실제 임베딩은 optional real-embed 단계에서만 켠다.
NOTE

cat > "$PACK_DIR/notes/thumbnail-hook.md" <<'NOTE'
---
type: prompt
title: 썸네일 훅 프롬프트
tags: [youtube, hook, ontology]
created: 2026-05-21
---
클릭을 부르는 문장: 내 파일이 스스로 연결되는 로컬 온톨로지 만들기.
NOTE

cat > "$PACK_DIR/notes/research/agent-memory.md" <<'NOTE'
---
type: research
title: 에이전트 메모리 설계 노트
tags: [agent, memory, ontology]
created: 2026-05-19
related: [lecture-outline]
---
에이전트는 사용자의 로컬 자료를 직접 읽고 출처 카드로 반환해야 한다.
NOTE

# Filter stress dataset: many higher-ranked non-prompt hits plus one matching prompt.
for i in $(seq -w 1 125); do
  cat > "$PACK_DIR/notes/filter-distractor-$i.md" <<NOTE
type: note
title: 필터 방해 노트 $i
---
공통질문 반복 데이터 $i. 이 노트는 prompt 타입이 아니므로 type=prompt 검색에서 제외되어야 한다.
NOTE
done

cat > "$PACK_DIR/notes/filter-target.md" <<'NOTE'
---
type: prompt
title: 필터 대상 프롬프트
tags: [needle, ontology]
created: 2026-05-22
---
공통질문 최종 답변은 이 프롬프트 카드에서 찾아야 한다.
NOTE

printf '\x89PNG\r\n' > "$PACK_DIR/evidence.png"
(cd "$PACK_DIR" && "$PACK_BIN" add "$PACK_DIR/evidence.png" --type image >/tmp/ontopack-real-add-image.out)
cat > "$PACK_DIR/notes/evidence-image.md" <<'NOTE'
---
type: image
title: 보드 사진 캡션
asset: assets/evidence.png
tags: [gallery, ontology]
created: 2026-05-18
---
화이트보드에 로컬 온톨로지 노드와 관계가 그려져 있다.
NOTE

# Replace sidecar auto-created from add command with deterministic id above.
rm -f "$PACK_DIR/notes/evidence.md"

cat > "$PACK_DIR/assets/diagram.svg" <<'SVG'
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 120 80"><rect width="120" height="80" fill="#071012"/><circle cx="38" cy="40" r="18" fill="#00f99a"/><path d="M58 40h34" stroke="#d8e5e0" stroke-width="6"/></svg>
SVG
cat > "$PACK_DIR/notes/diagram-image.md" <<'NOTE'
---
type: image
title: 온톨로지 다이어그램
asset: assets/diagram.svg
tags: [gallery, ontology, media]
created: 2026-05-19
---
로컬 온톨로지 그래프를 보여주는 SVG 다이어그램이다.
NOTE
printf '\x00\x00\x00\x18ftypmp42' > "$PACK_DIR/assets/demo.mp4"
cat > "$PACK_DIR/notes/demo-video.md" <<'NOTE'
---
type: video
title: 데모 비디오
asset: assets/demo.mp4
tags: [gallery, ontology, media]
created: 2026-05-20
---
지식팩 검색 흐름을 짧게 보여주는 데모 비디오 클립이다.
NOTE

echo "[3/10] process inbox and build index twice"
(cd "$PACK_DIR" && "$PACK_BIN" process >/tmp/ontopack-real-process.out)
(cd "$PACK_DIR" && "$PACK_BIN" build --no-embed >/tmp/ontopack-real-build.out)
(cd "$PACK_DIR" && "$PACK_BIN" build --incremental --no-embed >/tmp/ontopack-real-build-incremental.out)
grep -q 'skipped=' /tmp/ontopack-real-build-incremental.out
(cd "$PACK_DIR" && "$PACK_BIN" status >/tmp/ontopack-real-status-before.out)
grep -q 'pending_enrichment=' /tmp/ontopack-real-status-before.out
(cd "$PACK_DIR" && "$PACK_BIN" list --pending-enrichment >/tmp/ontopack-real-pending.out)
grep -q 'demo-video' /tmp/ontopack-real-pending.out
(cd "$PACK_DIR" && "$PACK_BIN" enrich-note demo-video --caption 'AI generated cockpit walkthrough for OntoPack retrieval' --tag enriched --provider real-test --model deterministic >/tmp/ontopack-real-enrich.out)
grep -q 'enrichment 업데이트' /tmp/ontopack-real-enrich.out
(cd "$PACK_DIR" && "$PACK_BIN" build --incremental --no-embed >/tmp/ontopack-real-build-enriched.out)
(cd "$PACK_DIR" && "$PACK_BIN" status >/tmp/ontopack-real-status-after.out)
grep -q 'done_enrichment=1' /tmp/ontopack-real-status-after.out

echo "[4/10] CLI real-user keyword searches"
CLI_ONTOLOGY="$(cd "$PACK_DIR" && "$PACK_BIN" search "온톨로지" --mode keyword -k 5)"
printf '%s\n' "$CLI_ONTOLOGY" | grep -q '\[keyword\]'
printf '%s\n' "$CLI_ONTOLOGY" | grep -q 'lecture-outline#0000\|thumbnail-hook#0000\|evidence-image#0000'
CLI_TRANSCRIPT="$(cd "$PACK_DIR" && "$PACK_BIN" search "모델 다운로드" --mode keyword -k 3)"
printf '%s\n' "$CLI_TRANSCRIPT" | grep -q 'transcript#0000'
CLI_ENRICHED="$(cd "$PACK_DIR" && "$PACK_BIN" search "cockpit" --mode keyword -k 3)"
printf '%s\n' "$CLI_ENRICHED" | grep -q 'demo-video#0000'

echo "[5/10] viewer API filtered search, including >100 distractors"
SEARCH_JSON="$(serve_json $'GET /api/search?q=%EA%B3%B5%ED%86%B5%EC%A7%88%EB%AC%B8&type=prompt&tag=needle&from=2026-05-01&to=2026-05-31&k=1 HTTP/1.1\r\nHost: localhost\r\n\r\n')"
assert_json "filtered search returns target with timing" "$SEARCH_JSON" 'len(v["hits"]) == 1 and v["hits"][0]["note_id"] == "filter-target" and v["mode"] == "keyword" and v["source"] == "sqlite_fts" and isinstance(v["elapsed_ms"], int)'
VECTOR_ERROR_JSON="$(serve_json $'GET /api/search?q=%EA%B3%B5%ED%86%B5%EC%A7%88%EB%AC%B8&mode=vector HTTP/1.1\r\nHost: localhost\r\n\r\n')"
assert_json "server rejects unavailable vector search honestly" "$VECTOR_ERROR_JSON" '"search mode unavailable" in v["error"]'

ASK_JSON="$(serve_json $'GET /api/ask?q=%EC%98%A8%ED%86%A8%EB%A1%9C%EC%A7%80&k=4 HTTP/1.1\r\nHost: localhost\r\n\r\n')"
assert_json "ask returns context blocks with timing" "$ASK_JSON" 'v["answer_mode"] == "external_llm_required" and len(v["context_blocks"]) >= 1 and isinstance(v["elapsed_ms"], int)'

ERROR_JSON="$(serve_json $'GET /api/search HTTP/1.1\r\nHost: localhost\r\n\r\n')"
assert_json "missing q returns json error" "$ERROR_JSON" '"missing query parameter: q" in v["error"]'

echo "[6/10] viewer API facets/gallery/timeline/graph/note/related"
CAPS_JSON="$(serve_json $'GET /api/capabilities HTTP/1.1\r\nHost: localhost\r\n\r\n')"
assert_json "capabilities report keyword-only server mode" "$CAPS_JSON" 'v["default_search_mode"] == "keyword" and v["semantic_search"] is False and any(m["mode"] == "vector" and m["available"] is False for m in v["search_modes"])'
FACETS_JSON="$(serve_json $'GET /api/facets HTTP/1.1\r\nHost: localhost\r\n\r\n')"
assert_json "facets include prompt and ontology" "$FACETS_JSON" '"prompt" in v["types"] and "ontology" in v["tags"]'
DASHBOARD_JSON="$(serve_json $'GET /api/dashboard?type=image&gallery_k=5&timeline_k=5&graph_limit=20 HTTP/1.1\r\nHost: localhost\r\n\r\n')"
assert_json "dashboard aggregates facets gallery timeline graph with timing" "$DASHBOARD_JSON" '"facets" in v and any(item["media_kind"] == "image" for item in v["gallery"]["items"]) and "notes" in v["timeline"] and "nodes" in v["graph"] and isinstance(v["elapsed_ms"], int)'
GALLERY_JSON="$(serve_json $'GET /api/gallery?k=5 HTTP/1.1\r\nHost: localhost\r\n\r\n')"
assert_json "gallery includes evidence image asset metadata" "$GALLERY_JSON" 'any(item["asset"] == "assets/evidence.png" and item["asset_url"] == "/assets/evidence.png" and item["media_kind"] == "image" for item in v["items"])'
assert_json "gallery includes video asset metadata" "$GALLERY_JSON" 'any(item["asset"] == "assets/demo.mp4" and item["asset_url"] == "/assets/demo.mp4" and item["media_kind"] == "video" for item in v["items"])'
ASSET_SVG_BODY="$(cd "$PACK_DIR" && "$PACK_BIN" serve --port 0 --once --request $'GET /assets/diagram.svg HTTP/1.1\r\nHost: localhost\r\n\r\n')"
printf '%s\n' "$ASSET_SVG_BODY" | grep -q '<svg'
TIMELINE_JSON="$(serve_json $'GET /api/timeline?type=prompt&from=2026-05-01&to=2026-05-31&k=5 HTTP/1.1\r\nHost: localhost\r\n\r\n')"
assert_json "timeline filters prompt dates" "$TIMELINE_JSON" 'any(note["id"] == "filter-target" for note in v["notes"])'
GRAPH_JSON="$(serve_json $'GET /api/graph?limit=20 HTTP/1.1\r\nHost: localhost\r\n\r\n')"
assert_json "graph includes edges" "$GRAPH_JSON" 'len(v["nodes"]) >= 1 and "edges" in v'
NOTE_JSON="$(serve_json $'GET /api/notes/lecture-outline HTTP/1.1\r\nHost: localhost\r\n\r\n')"
assert_json "note detail returns relations" "$NOTE_JSON" 'v["id"] == "lecture-outline" and "thumbnail-hook" in v["related"]'
VIDEO_NOTE_JSON="$(serve_json $'GET /api/notes/demo-video HTTP/1.1\r\nHost: localhost\r\n\r\n')"
assert_json "video note returns playable media metadata" "$VIDEO_NOTE_JSON" 'v["asset_url"] == "/assets/demo.mp4" and v["media_kind"] == "video" and v["mime"] == "video/mp4"'
RELATED_JSON="$(serve_json $'GET /api/related/lecture-outline?depth=1 HTTP/1.1\r\nHost: localhost\r\n\r\n')"
assert_json "related follows links" "$RELATED_JSON" 'any(item["id"] == "thumbnail-hook" for item in v["related"])'

echo "[7/10] MCP stdio tools against realistic pack"
MCP_OUT="$(printf '%s\n' \
  '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25"}}' \
  '{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}' \
  '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"search","arguments":{"query":"공통질문","type":"prompt","k":1}}}' \
  '{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"timeline","arguments":{"type":"prompt","from":"2026-05-01","to":"2026-05-31","k":5}}}' \
  '{"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"ask","arguments":{"question":"온톨로지 강의 핵심?","k":3}}}' \
  | "$MCP_BIN" --pack-root "$PACK_DIR")"
printf '%s\n' "$MCP_OUT" | grep -q '"serverInfo"'
printf '%s\n' "$MCP_OUT" | grep -q 'filter-target'
printf '%s\n' "$MCP_OUT" | grep -q 'context_blocks'

echo "[8/10] open URL smoke"
OPEN_URL="$(cd "$PACK_DIR" && "$PACK_BIN" open --port 0 --no-browser --print-url)"
printf '%s\n' "$OPEN_URL" | grep -q '^http://127\.0\.0\.1:'

echo "[9/10] optional real embedding gate"
if [[ "$RUN_REAL_EMBED" == "1" ]]; then
  REAL_PACK_BIN="${REAL_PACK_BIN:-$ROOT/target/release/pack}"
  cargo build --quiet --release -p pack-cli --features real-embed --manifest-path "$ROOT/Cargo.toml"
  (cd "$PACK_DIR" && "$REAL_PACK_BIN" embed --skip-build >/tmp/ontopack-real-embed.out)
  (cd "$PACK_DIR" && "$REAL_PACK_BIN" search "강의 자료 연결" --mode hybrid -k 5 >/tmp/ontopack-real-hybrid.out)
  grep -q '\[hybrid\]\|\[keyword\]\|\[vector\]' /tmp/ontopack-real-hybrid.out
else
  echo "skip real embedding download/runtime; set RUN_REAL_EMBED=1 to exercise BGE-M3 path"
fi

echo "[10/10] real test summary"
echo "Ontopack real test passed: realistic pack + CLI + MCP + viewer APIs + filter stress + open URL"
