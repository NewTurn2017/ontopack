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

echo "[1/11] build debug binaries"
cargo build --quiet -p pack-cli -p pack-mcp --manifest-path "$ROOT/Cargo.toml"

echo "[2/11] seed realistic pack: $PACK_DIR"
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

cat > "$PACK_DIR/notes/duplicate-a.md" <<'NOTE'
---
type: note
title: 중복 후보 A
tags: [ops]
---
반복되는 운영 메모 본문이다.
NOTE
cat > "$PACK_DIR/notes/duplicate-b.md" <<'NOTE'
---
type: note
title: 중복 후보 B
tags: [ops]
---
반복되는   운영 메모 본문이다.
NOTE

cat > "$PACK_DIR/notes/orphan-gap.md" <<'NOTE'
---
type: note
title: 연결 점검용 외톨이 노트
tags: [ops, hygiene]
---
어떤 노트와도 연결되지 않은 유지보수 점검용 노트다.
NOTE

cat > "$PACK_DIR/notes/dangling-gap.md" <<'NOTE'
---
type: note
title: 깨진 링크 점검 노트
tags: [ops, hygiene]
---
이 노트는 아직 만들지 않은 [[missing-hygiene-target]]을 가리킨다.
NOTE

cat > "$PACK_DIR/notes/recommend-a.md" <<'NOTE'
---
type: note
title: 추천 소스 A
tags: [recommend, ontology]
---
태그가 겹치는 관련 노트 후보를 찾기 위한 소스 노트다.
NOTE
cat > "$PACK_DIR/notes/recommend-b.md" <<'NOTE'
---
type: note
title: 추천 후보 B
tags: [recommend, ontology]
---
아직 링크되지는 않았지만 recommend-a와 명시 태그가 겹친다.
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

echo "[3/11] process inbox and build index twice"
(cd "$PACK_DIR" && "$PACK_BIN" process >/tmp/ontopack-real-process.out)
(cd "$PACK_DIR" && "$PACK_BIN" build --no-embed >/tmp/ontopack-real-build.out)
(cd "$PACK_DIR" && "$PACK_BIN" build --incremental --no-embed >/tmp/ontopack-real-build-incremental.out)
grep -q 'skipped=' /tmp/ontopack-real-build-incremental.out
(cd "$PACK_DIR" && "$PACK_BIN" status >/tmp/ontopack-real-status-before.out)
grep -q 'pending_enrichment=' /tmp/ontopack-real-status-before.out
(cd "$PACK_DIR" && "$PACK_BIN" list --pending-enrichment >/tmp/ontopack-real-pending.out)
grep -q 'demo-video' /tmp/ontopack-real-pending.out
cat > "$PACK_DIR/demo-video-transcript.txt" <<'NOTE'
[00:00:07] AI generated cockpit walkthrough for OntoPack retrieval timelinejump
NOTE
(cd "$PACK_DIR" && "$PACK_BIN" enrich-note demo-video --caption 'AI generated cockpit walkthrough for OntoPack retrieval' --transcript "$PACK_DIR/demo-video-transcript.txt" --tag enriched --provider real-test --model deterministic >/tmp/ontopack-real-enrich.out)
grep -q 'enrichment 업데이트' /tmp/ontopack-real-enrich.out
(cd "$PACK_DIR" && "$PACK_BIN" build --incremental --no-embed >/tmp/ontopack-real-build-enriched.out)
(cd "$PACK_DIR" && "$PACK_BIN" status >/tmp/ontopack-real-status-after.out)
grep -q 'done_enrichment=1' /tmp/ontopack-real-status-after.out
(cd "$PACK_DIR" && "$PACK_BIN" doctor >/tmp/ontopack-real-doctor.out)
grep -q 'doctor: ok=true' /tmp/ontopack-real-doctor.out
grep -q -- '- ok index' /tmp/ontopack-real-doctor.out
(cd "$PACK_DIR" && "$PACK_BIN" doctor --json >/tmp/ontopack-real-doctor.json)
grep -q '"ok": true' /tmp/ontopack-real-doctor.json
"$PACK_BIN" completions zsh >/tmp/ontopack-real-completions.zsh
grep -q '#compdef pack' /tmp/ontopack-real-completions.zsh
grep -q 'doctor' /tmp/ontopack-real-completions.zsh
"$PACK_BIN" completions bash >/tmp/ontopack-real-completions.bash
grep -q 'complete -F _pack_completions pack' /tmp/ontopack-real-completions.bash
rm -rf /tmp/ontopack-real-install
"$ROOT/scripts/install.sh" --prefix /tmp/ontopack-real-install --bin "$PACK_BIN" --no-build --completion-shell zsh >/tmp/ontopack-real-install.out
grep -q 'installed pack:' /tmp/ontopack-real-install.out
test -x /tmp/ontopack-real-install/bin/pack
grep -q '#compdef pack' /tmp/ontopack-real-install/share/zsh/site-functions/_pack
"$ROOT/scripts/install-launch-agent.sh" --pack-root "$PACK_DIR" --pack-bin /tmp/ontopack-real-install/bin/pack --label com.ontopack.real-test --interval-ms 1200 --output /tmp/ontopack-real-launch-agent.plist >/tmp/ontopack-real-launch-agent.out
grep -q 'wrote launch agent plist:' /tmp/ontopack-real-launch-agent.out
python3 - /tmp/ontopack-real-launch-agent.plist "$PACK_DIR" <<'PY'
import os, plistlib, sys
plist_path, pack_dir = sys.argv[1], sys.argv[2]
with open(plist_path, 'rb') as f:
    data = plistlib.load(f)
assert data['Label'] == 'com.ontopack.real-test', data
assert data['ProgramArguments'] == ['/tmp/ontopack-real-install/bin/pack', 'watch', '--interval-ms', '1200'], data
assert data['WorkingDirectory'] == os.path.normpath(pack_dir), data
assert data['RunAtLoad'] is True and data['KeepAlive'] is True, data
assert data['StandardOutPath'].endswith('/.pack/watch.out.log'), data
assert data['StandardErrorPath'].endswith('/.pack/watch.err.log'), data
PY
SKIP_BUILD=1 PACK_BIN="$PACK_BIN" NOTE_COUNT=80 MEDIA_COUNT=8 REPEATS=1 WARMUP=0 P95_MAX_MS=1000 OUT_JSON=/tmp/ontopack-real-perf-smoke.json OUT_MD=/tmp/ontopack-real-perf-smoke.md "$ROOT/scripts/perf-smoke.sh" >/tmp/ontopack-real-perf-smoke.out
grep -q 'perf smoke ok:' /tmp/ontopack-real-perf-smoke.out
(cd "$PACK_DIR" && ONTOPACK_LOCAL_WORKER="$ROOT/scripts/providers/fixture_media_worker.py" OPENAI_API_KEY="" "$PACK_BIN" enrich-pending --provider-command "$ROOT/scripts/providers/auto_media_worker.py" --limit 1 >/tmp/ontopack-real-enrich-pending.out)
grep -q 'processed=1' /tmp/ontopack-real-enrich-pending.out
grep -q 'indexed=' /tmp/ontopack-real-enrich-pending.out
cat > "$PACK_DIR/_inbox/watch-real.md" <<'NOTE'
---
type: note
title: Watch Real Smoke
tags: [watch, ops]
---
watchword 실사용 폴링 스모크 노트.
NOTE
(cd "$PACK_DIR" && "$PACK_BIN" watch --once >/tmp/ontopack-real-watch.out)
grep -q 'watch tick: cycle=1 processed=1' /tmp/ontopack-real-watch.out
grep -q 'indexed=' /tmp/ontopack-real-watch.out

echo "[4/11] CLI real-user keyword searches"
CLI_ONTOLOGY="$(cd "$PACK_DIR" && "$PACK_BIN" search "온톨로지" --mode keyword -k 5)"
printf '%s\n' "$CLI_ONTOLOGY" | grep -q '\[keyword\]'
printf '%s\n' "$CLI_ONTOLOGY" | grep -q 'lecture-outline#0000\|thumbnail-hook#0000\|evidence-image#0000'
CLI_TRANSCRIPT="$(cd "$PACK_DIR" && "$PACK_BIN" search "모델 다운로드" --mode keyword -k 3)"
printf '%s\n' "$CLI_TRANSCRIPT" | grep -q 'transcript#0000'
CLI_ENRICHED="$(cd "$PACK_DIR" && "$PACK_BIN" search "cockpit" --mode keyword -k 3)"
printf '%s\n' "$CLI_ENRICHED" | grep -q 'demo-video#0000'
CLI_WORKER="$(cd "$PACK_DIR" && "$PACK_BIN" search "fixture-provider" --mode keyword -k 3)"
printf '%s\n' "$CLI_WORKER" | grep -q '#0000'
CLI_WATCH="$(cd "$PACK_DIR" && "$PACK_BIN" search "watchword" --mode keyword -k 3)"
printf '%s\n' "$CLI_WATCH" | grep -q 'watch-real#0000'
(cd "$PACK_DIR" && "$PACK_BIN" duplicates >/tmp/ontopack-real-duplicates.out)
grep -q '중복 후보: groups=1' /tmp/ontopack-real-duplicates.out
grep -q 'duplicate-a' /tmp/ontopack-real-duplicates.out
grep -q 'duplicate-b' /tmp/ontopack-real-duplicates.out
(cd "$PACK_DIR" && "$PACK_BIN" orphans >/tmp/ontopack-real-orphans.out)
grep -q '외톨이 노트: count=' /tmp/ontopack-real-orphans.out
grep -q 'orphan-gap' /tmp/ontopack-real-orphans.out
(cd "$PACK_DIR" && "$PACK_BIN" orphans --json >/tmp/ontopack-real-orphans.json)
grep -q '"note_id": "orphan-gap"' /tmp/ontopack-real-orphans.json
(cd "$PACK_DIR" && "$PACK_BIN" gaps >/tmp/ontopack-real-gaps.out)
grep -q '깨진 링크: count=' /tmp/ontopack-real-gaps.out
grep -q 'dangling-gap -> missing-hygiene-target' /tmp/ontopack-real-gaps.out
(cd "$PACK_DIR" && "$PACK_BIN" gaps --json >/tmp/ontopack-real-gaps.json)
grep -q '"missing_target": "missing-hygiene-target"' /tmp/ontopack-real-gaps.json
(cd "$PACK_DIR" && "$PACK_BIN" topics --min-count 3 >/tmp/ontopack-real-topics.out)
grep -q '토픽맵: topics=' /tmp/ontopack-real-topics.out
grep -q 'topic ontology count=' /tmp/ontopack-real-topics.out
(cd "$PACK_DIR" && "$PACK_BIN" topics --min-count 3 --json >/tmp/ontopack-real-topics.json)
grep -q '"topic": "ontology"' /tmp/ontopack-real-topics.json
(cd "$PACK_DIR" && "$PACK_BIN" recommend recommend-a -k 1 >/tmp/ontopack-real-recommend.out)
grep -q '관련 노트 추천: count=1' /tmp/ontopack-real-recommend.out
grep -q 'recommend-a -> recommend-b score=2' /tmp/ontopack-real-recommend.out
(cd "$PACK_DIR" && "$PACK_BIN" recommend recommend-a --json >/tmp/ontopack-real-recommend.json)
grep -q '"candidate_id": "recommend-b"' /tmp/ontopack-real-recommend.json

echo "[5/11] portable context exports"
(cd "$PACK_DIR" && "$PACK_BIN" export --format jsonl >/tmp/ontopack-real-export.jsonl)
grep -q '"note_id":"lecture-outline"' /tmp/ontopack-real-export.jsonl
grep -q '"asset_path":"assets/evidence.png"' /tmp/ontopack-real-export.jsonl
rm -rf /tmp/ontopack-real-export-assets
(cd "$PACK_DIR" && "$PACK_BIN" export --format markdown-bundle --output /tmp/ontopack-real-export.md --copy-assets /tmp/ontopack-real-export-assets >/tmp/ontopack-real-export-file.out)
grep -q 'export 완료' /tmp/ontopack-real-export-file.out
grep -q 'assets copied=' /tmp/ontopack-real-export-file.out
grep -q 'Citation: `note:lecture-outline`' /tmp/ontopack-real-export.md
grep -q 'Asset: `assets/evidence.png`' /tmp/ontopack-real-export.md
test -s /tmp/ontopack-real-export-assets/assets/evidence.png
(cd "$PACK_DIR" && "$PACK_BIN" export --format mcp-context >/tmp/ontopack-real-mcp-context.json)
grep -q '"type":"ontopack.mcp_context"' /tmp/ontopack-real-mcp-context.json
grep -q '"citation":"note:demo-video"' /tmp/ontopack-real-mcp-context.json
rm -rf /tmp/ontopack-real-import-pack
"$PACK_BIN" init /tmp/ontopack-real-import-pack >/tmp/ontopack-real-import-init.out
(cd /tmp/ontopack-real-import-pack && "$PACK_BIN" import /tmp/ontopack-real-export.jsonl --format jsonl --asset-root /tmp/ontopack-real-export-assets >/tmp/ontopack-real-import.out)
grep -q 'import 완료:' /tmp/ontopack-real-import.out
test -s /tmp/ontopack-real-import-pack/assets/evidence.png
(cd /tmp/ontopack-real-import-pack && "$PACK_BIN" build --no-embed >/tmp/ontopack-real-import-build.out)
(cd /tmp/ontopack-real-import-pack && "$PACK_BIN" search "온톨로지" --mode keyword -k 3 >/tmp/ontopack-real-import-search.out)
grep -q '#0000' /tmp/ontopack-real-import-search.out
rm -rf /tmp/ontopack-real-bundle-pack /tmp/ontopack-real-bundle-restore /tmp/ontopack-real-bundle-archive-restore /tmp/ontopack-real-broken-bundle /tmp/ontopack-real-broken-bundle-restore
rm -f /tmp/ontopack-real-bundle.tar.gz
(cd "$PACK_DIR" && "$PACK_BIN" bundle /tmp/ontopack-real-bundle-pack --archive /tmp/ontopack-real-bundle.tar.gz >/tmp/ontopack-real-bundle.out)
grep -q 'bundle 완료:' /tmp/ontopack-real-bundle.out
grep -q 'archive 완료:' /tmp/ontopack-real-bundle.out
test -s /tmp/ontopack-real-bundle-pack/context.jsonl
test -s /tmp/ontopack-real-bundle-pack/bundle.json
test -s /tmp/ontopack-real-bundle-pack/assets/evidence.png
test -s /tmp/ontopack-real-bundle.tar.gz
"$PACK_BIN" init /tmp/ontopack-real-bundle-restore >/tmp/ontopack-real-bundle-restore-init.out
(cd /tmp/ontopack-real-bundle-restore && "$PACK_BIN" import /tmp/ontopack-real-bundle-pack >/tmp/ontopack-real-bundle-import.out)
grep -q 'import 완료:' /tmp/ontopack-real-bundle-import.out
test -s /tmp/ontopack-real-bundle-restore/assets/evidence.png
cmp "$PACK_DIR/assets/evidence.png" /tmp/ontopack-real-bundle-restore/assets/evidence.png
if (cd /tmp/ontopack-real-bundle-restore && "$PACK_BIN" import /tmp/ontopack-real-bundle-pack >/tmp/ontopack-real-bundle-reimport.out 2>/tmp/ontopack-real-bundle-reimport.err); then
  echo "expected bundle re-import without --overwrite to fail" >&2
  exit 1
fi
grep -q 'import note already exists' /tmp/ontopack-real-bundle-reimport.err
(cd /tmp/ontopack-real-bundle-restore && "$PACK_BIN" import /tmp/ontopack-real-bundle-pack --overwrite >/tmp/ontopack-real-bundle-overwrite.out)
grep -q 'import 완료:' /tmp/ontopack-real-bundle-overwrite.out
"$PACK_BIN" init /tmp/ontopack-real-bundle-archive-restore >/tmp/ontopack-real-bundle-archive-restore-init.out
(cd /tmp/ontopack-real-bundle-archive-restore && "$PACK_BIN" import /tmp/ontopack-real-bundle.tar.gz >/tmp/ontopack-real-bundle-archive-import.out)
grep -q 'import 완료:' /tmp/ontopack-real-bundle-archive-import.out
cmp "$PACK_DIR/assets/evidence.png" /tmp/ontopack-real-bundle-archive-restore/assets/evidence.png
cp -R /tmp/ontopack-real-bundle-pack /tmp/ontopack-real-broken-bundle
rm -f /tmp/ontopack-real-broken-bundle/assets/evidence.png
"$PACK_BIN" init /tmp/ontopack-real-broken-bundle-restore >/tmp/ontopack-real-broken-bundle-restore-init.out
if (cd /tmp/ontopack-real-broken-bundle-restore && "$PACK_BIN" import /tmp/ontopack-real-broken-bundle >/tmp/ontopack-real-broken-bundle-import.out 2>/tmp/ontopack-real-broken-bundle-import.err); then
  echo "expected broken bundle import to fail" >&2
  exit 1
fi
grep -q 'import asset missing: assets/evidence.png' /tmp/ontopack-real-broken-bundle-import.err
test ! -e /tmp/ontopack-real-broken-bundle-restore/notes/evidence-image.md

echo "[6/11] viewer API filtered search, including >100 distractors"
SEARCH_JSON="$(serve_json $'GET /api/search?q=%EA%B3%B5%ED%86%B5%EC%A7%88%EB%AC%B8&type=prompt&tag=needle&from=2026-05-01&to=2026-05-31&k=1 HTTP/1.1\r\nHost: localhost\r\n\r\n')"
assert_json "filtered search returns target with timing" "$SEARCH_JSON" 'len(v["hits"]) == 1 and v["hits"][0]["note_id"] == "filter-target" and v["mode"] == "keyword" and v["source"] == "sqlite_fts" and isinstance(v["elapsed_ms"], int)'
MEDIA_CITATION_JSON="$(serve_json $'GET /api/search?q=timelinejump&k=1 HTTP/1.1\r\nHost: localhost\r\n\r\n')"
assert_json "video transcript search exposes media citation time" "$MEDIA_CITATION_JSON" 'v["hits"][0]["note_id"] == "demo-video" and v["hits"][0]["media_citation"]["time"] == "00:00:07" and v["hits"][0]["media_citation"]["seconds"] == 7 and v["hits"][0]["media_citation"]["asset_url"].endswith("#t=7")'
VECTOR_ERROR_JSON="$(serve_json $'GET /api/search?q=%EA%B3%B5%ED%86%B5%EC%A7%88%EB%AC%B8&mode=vector HTTP/1.1\r\nHost: localhost\r\n\r\n')"
assert_json "server rejects unavailable vector search honestly" "$VECTOR_ERROR_JSON" '"search mode unavailable" in v["error"]'

ASK_JSON="$(serve_json $'GET /api/ask?q=%EC%98%A8%ED%86%A8%EB%A1%9C%EC%A7%80&k=4 HTTP/1.1\r\nHost: localhost\r\n\r\n')"
assert_json "ask returns context blocks with timing" "$ASK_JSON" 'v["answer_mode"] == "external_llm_required" and len(v["context_blocks"]) >= 1 and isinstance(v["elapsed_ms"], int)'

ERROR_JSON="$(serve_json $'GET /api/search HTTP/1.1\r\nHost: localhost\r\n\r\n')"
assert_json "missing q returns json error" "$ERROR_JSON" '"missing query parameter: q" in v["error"]'

echo "[7/11] viewer API facets/gallery/timeline/graph/note/related"
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

echo "[8/11] MCP stdio tools against realistic pack"
MCP_OUT="$(printf '%s\n' \
  '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25"}}' \
  '{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}' \
  '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"search","arguments":{"query":"공통질문","type":"prompt","k":1}}}' \
  '{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"timeline","arguments":{"type":"prompt","from":"2026-05-01","to":"2026-05-31","k":5}}}' \
  '{"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"ask","arguments":{"question":"온톨로지 강의 핵심?","k":3}}}' \
  '{"jsonrpc":"2.0","id":6,"method":"tools/call","params":{"name":"media/list_pending","arguments":{"k":10}}}' \
  '{"jsonrpc":"2.0","id":7,"method":"tools/call","params":{"name":"media/read_note","arguments":{"note_id":"diagram-image"}}}' \
  '{"jsonrpc":"2.0","id":8,"method":"tools/call","params":{"name":"media/write_enrichment","arguments":{"note_id":"diagram-image","caption":"MCP generated graph lattice caption","tags":["mcp-enriched"],"provider":"real-test","model":"deterministic"}}}' \
  '{"jsonrpc":"2.0","id":9,"method":"tools/call","params":{"name":"index/rebuild","arguments":{}}}' \
  '{"jsonrpc":"2.0","id":10,"method":"tools/call","params":{"name":"search","arguments":{"query":"lattice","k":3}}}' \
  | "$MCP_BIN" --pack-root "$PACK_DIR")"
printf '%s\n' "$MCP_OUT" | grep -q '"serverInfo"'
printf '%s\n' "$MCP_OUT" | grep -q 'media/list_pending'
printf '%s\n' "$MCP_OUT" | grep -q 'filter-target'
printf '%s\n' "$MCP_OUT" | grep -q 'context_blocks'
printf '%s\n' "$MCP_OUT" | grep -q 'diagram-image'
printf '%s\n' "$MCP_OUT" | grep -q 'MCP generated graph lattice caption'

echo "[9/11] open URL smoke"
OPEN_URL="$(cd "$PACK_DIR" && "$PACK_BIN" open --port 0 --no-browser --print-url)"
printf '%s\n' "$OPEN_URL" | grep -q '^http://127\.0\.0\.1:'

echo "[10/11] optional real embedding gate"
if [[ "$RUN_REAL_EMBED" == "1" ]]; then
  REAL_PACK_BIN="${REAL_PACK_BIN:-$ROOT/target/release/pack}"
  cargo build --quiet --release -p pack-cli --features real-embed --manifest-path "$ROOT/Cargo.toml"
  (cd "$PACK_DIR" && "$REAL_PACK_BIN" embed --skip-build >/tmp/ontopack-real-embed.out)
  (cd "$PACK_DIR" && "$REAL_PACK_BIN" search "강의 자료 연결" --mode hybrid -k 5 >/tmp/ontopack-real-hybrid.out)
  grep -q '\[hybrid\]\|\[keyword\]\|\[vector\]' /tmp/ontopack-real-hybrid.out
else
  echo "skip real embedding download/runtime; set RUN_REAL_EMBED=1 to exercise BGE-M3 path"
fi

echo "[11/11] real test summary"
echo "Ontopack real test passed: realistic pack + CLI + exports + MCP + viewer APIs + filter stress + open URL"
