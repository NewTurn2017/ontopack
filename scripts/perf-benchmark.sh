#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TMPDIR="${TMPDIR:-/tmp}"
PACK_DIR="${PACK_DIR:-$(mktemp -d "$TMPDIR/ontopack-perf.XXXXXX")}"
KEEP_PERF_PACK="${KEEP_PERF_PACK:-0}"
NOTE_COUNT="${NOTE_COUNT:-2000}"
MEDIA_COUNT="${MEDIA_COUNT:-160}"
REPEATS="${REPEATS:-9}"
WARMUP="${WARMUP:-2}"
PACK_BIN="${PACK_BIN:-$ROOT/target/debug/pack}"
SKIP_BUILD="${SKIP_BUILD:-0}"
OUT_DIR="${OUT_DIR:-$ROOT/output/perf}"
STAMP="${STAMP:-$(date +%Y%m%d-%H%M%S)}"
OUT_JSON="${OUT_JSON:-$OUT_DIR/ontopack-perf-$STAMP.json}"
OUT_MD="${OUT_MD:-$ROOT/docs/test-results/$(date +%Y-%m-%d)-perf-benchmark.md}"
SERVER_LOG="${SERVER_LOG:-/tmp/ontopack-perf-server.$STAMP.log}"
SERVER_PID_FILE="${SERVER_PID_FILE:-/tmp/ontopack-perf-server.$STAMP.pid}"

cleanup() {
  if [[ -f "$SERVER_PID_FILE" ]]; then
    kill "$(cat "$SERVER_PID_FILE")" >/dev/null 2>&1 || true
    rm -f "$SERVER_PID_FILE"
  fi
  if [[ "$KEEP_PERF_PACK" != "1" ]]; then
    rm -rf "$PACK_DIR"
  else
    echo "perf pack kept: $PACK_DIR"
  fi
}
trap cleanup EXIT

echo "[1/7] build debug pack binary"
if [[ "$SKIP_BUILD" == "1" ]]; then
  test -x "$PACK_BIN"
else
  cargo build --quiet -p pack-cli --manifest-path "$ROOT/Cargo.toml"
fi

echo "[2/7] seed synthetic pack: $PACK_DIR ($NOTE_COUNT notes, $MEDIA_COUNT media notes)"
"$PACK_BIN" init "$PACK_DIR" >/tmp/ontopack-perf-init.out
mkdir -p "$PACK_DIR/notes" "$PACK_DIR/assets"
PACK_DIR="$PACK_DIR" NOTE_COUNT="$NOTE_COUNT" MEDIA_COUNT="$MEDIA_COUNT" python3 <<'PY'
import os
from pathlib import Path

root = Path(os.environ["PACK_DIR"])
note_count = int(os.environ["NOTE_COUNT"])
media_count = min(int(os.environ["MEDIA_COUNT"]), note_count)
types = ["prompt", "lecture", "research", "note", "project"]
tags = ["bench", "ontology", "lecture", "media", "needle", "timeline", "graph"]

for i in range(media_count):
    asset = root / "assets" / f"bench-{i:04}.svg"
    hue = (i * 37) % 360
    asset.write_text(
        f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 160 90">'
        f'<rect width="160" height="90" fill="#071012"/>'
        f'<circle cx="{30 + (i % 80)}" cy="45" r="18" fill="hsl({hue},80%,52%)"/>'
        f'<text x="82" y="50" fill="#d8e5e0" font-size="12">BENCH {i:04}</text>'
        f'</svg>\n',
        encoding="utf-8",
    )

for i in range(note_count):
    note_type = "image" if i < media_count else types[i % len(types)]
    created = f"2026-05-{(i % 28) + 1:02}"
    tag_set = ["bench", tags[i % len(tags)], f"group-{i % 20:02}"]
    if i % 37 == 0:
        tag_set.append("needle")
    related = []
    if i > 0:
        related.append(f"bench-{i-1:04}")
    if i + 1 < note_count and i % 3 == 0:
        related.append(f"bench-{i+1:04}")
    asset_line = f"asset: assets/bench-{i:04}.svg\n" if i < media_count else ""
    needle = " 성능 니들 검색어가 이 뒤쪽 문장에 포함된다." if i % 37 == 0 else ""
    repeated = " ".join(
        [
            "로컬 온톨로지 벤치마크",
            f"문서 {i:04}",
            "검색 가능한 공통질문",
            "타임라인 그래프 갤러리 대시보드",
        ]
        * 9
    )
    body = f"{repeated}\n\n세부 설명 {i:04}.{needle}\n"
    frontmatter = (
        "---\n"
        f"type: {note_type}\n"
        f"title: 벤치 노트 {i:04}\n"
        f"tags: [{', '.join(tag_set)}]\n"
        f"created: {created}\n"
        f"related: [{', '.join(related)}]\n"
        f"{asset_line}"
        "---\n"
    )
    (root / "notes" / f"bench-{i:04}.md").write_text(frontmatter + body, encoding="utf-8")
PY

echo "[3/7] build SQLite/FTS index"
(cd "$PACK_DIR" && "$PACK_BIN" build --no-embed >/tmp/ontopack-perf-build.out)

echo "[4/7] start persistent local server"
(cd "$PACK_DIR" && "$PACK_BIN" serve --port 0 >"$SERVER_LOG" 2>&1 & echo $! >"$SERVER_PID_FILE")
for _ in {1..100}; do
  if grep -q 'http://127\.0\.0\.1:' "$SERVER_LOG" 2>/dev/null; then
    break
  fi
  sleep 0.1
done
URL="$(grep -Eo 'http://127\.0\.0\.1:[0-9]+' "$SERVER_LOG" | tail -1)"
if [[ -z "$URL" ]]; then
  echo "server did not print a URL" >&2
  cat "$SERVER_LOG" >&2 || true
  exit 1
fi

echo "[5/7] measure viewer API latency at $URL"
mkdir -p "$OUT_DIR" "$(dirname "$OUT_MD")"
URL="$URL" OUT_JSON="$OUT_JSON" OUT_MD="$OUT_MD" PACK_DIR="$PACK_DIR" NOTE_COUNT="$NOTE_COUNT" MEDIA_COUNT="$MEDIA_COUNT" REPEATS="$REPEATS" WARMUP="$WARMUP" python3 <<'PY'
import json
import os
import statistics
import time
import urllib.request
from datetime import datetime, timezone

url = os.environ["URL"].rstrip("/")
repeats = int(os.environ["REPEATS"])
warmup = int(os.environ["WARMUP"])

endpoints = [
    ("capabilities", "/api/capabilities"),
    ("dashboard_all", "/api/dashboard?gallery_k=12&timeline_k=10&graph_limit=80"),
    ("dashboard_image", "/api/dashboard?type=image&gallery_k=12&timeline_k=10&graph_limit=80"),
    ("search_needle", "/api/search?q=%EC%84%B1%EB%8A%A5%20%EB%8B%88%EB%93%A4&type=prompt&tag=needle&k=12"),
    ("search_common", "/api/search?q=%EA%B3%B5%ED%86%B5%EC%A7%88%EB%AC%B8&k=12"),
    ("gallery", "/api/gallery?k=24"),
    ("timeline", "/api/timeline?from=2026-05-01&to=2026-05-28&k=24"),
    ("graph", "/api/graph?limit=160"),
    ("note_detail", "/api/notes/bench-0000"),
    ("related", "/api/related/bench-0100?depth=1"),
]

def fetch(path):
    started = time.perf_counter()
    with urllib.request.urlopen(url + path, timeout=15) as response:
        body = response.read()
        status = response.status
    wall_ms = (time.perf_counter() - started) * 1000.0
    payload = json.loads(body)
    if status != 200:
        raise RuntimeError(f"{path} returned {status}: {payload}")
    return wall_ms, len(body), payload

results = {}
for name, path in endpoints:
    for _ in range(warmup):
        fetch(path)
    wall = []
    payload_elapsed = []
    sizes = []
    for _ in range(repeats):
        wall_ms, size, payload = fetch(path)
        wall.append(wall_ms)
        sizes.append(size)
        if isinstance(payload, dict) and isinstance(payload.get("elapsed_ms"), int):
            payload_elapsed.append(payload["elapsed_ms"])
    ordered = sorted(wall)
    p95_index = min(len(ordered) - 1, int(round((len(ordered) - 1) * 0.95)))
    results[name] = {
        "path": path,
        "repeats": repeats,
        "wall_ms": {
            "min": round(min(wall), 3),
            "p50": round(statistics.median(wall), 3),
            "p95": round(ordered[p95_index], 3),
            "max": round(max(wall), 3),
        },
        "payload_elapsed_ms": {
            "min": min(payload_elapsed) if payload_elapsed else None,
            "p50": statistics.median(payload_elapsed) if payload_elapsed else None,
            "max": max(payload_elapsed) if payload_elapsed else None,
        },
        "bytes": {
            "min": min(sizes),
            "max": max(sizes),
        },
    }

report = {
    "generated_at": datetime.now(timezone.utc).isoformat(),
    "pack_dir": os.environ["PACK_DIR"],
    "server_url": url,
    "note_count": int(os.environ["NOTE_COUNT"]),
    "media_count": int(os.environ["MEDIA_COUNT"]),
    "warmup": warmup,
    "repeats": repeats,
    "results": results,
}

with open(os.environ["OUT_JSON"], "w", encoding="utf-8") as f:
    json.dump(report, f, ensure_ascii=False, indent=2)

rows = []
for name, data in results.items():
    rows.append(
        f"| `{name}` | `{data['path']}` | {data['wall_ms']['p50']:.3f} | {data['wall_ms']['p95']:.3f} | {data['wall_ms']['max']:.3f} | {data['payload_elapsed_ms']['p50']} | {data['bytes']['max']} |"
    )
md = f"""# OntoPack synthetic performance benchmark — {datetime.now().date().isoformat()}

## Fixture

- Notes: {report['note_count']}
- Media sidecars: {report['media_count']}
- Warmup requests per endpoint: {warmup}
- Measured requests per endpoint: {repeats}
- Pack: `{report['pack_dir']}`
- Server: `{url}`
- Raw JSON: `{os.environ['OUT_JSON']}`

## Results

| Endpoint | Path | wall p50 ms | wall p95 ms | wall max ms | payload elapsed p50 ms | max bytes |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
{chr(10).join(rows)}

## Interpretation

- This benchmark measures a persistent local server, not `pack serve --once` startup overhead.
- `wall_ms` includes localhost HTTP and JSON transfer; `payload_elapsed_ms` is server handler timing when the endpoint exposes it.
- If dashboard/timeline/gallery p95 grows faster than search p95 as note count increases, the next optimization should be endpoint-specific SQLite queries instead of materializing all indexed notes per request.
"""
with open(os.environ["OUT_MD"], "w", encoding="utf-8") as f:
    f.write(md)

print(json.dumps({k: v["wall_ms"] for k, v in results.items()}, ensure_ascii=False, indent=2))
PY

echo "[6/7] write artifacts"
echo "json: $OUT_JSON"
echo "md:   $OUT_MD"

echo "[7/7] benchmark complete"
