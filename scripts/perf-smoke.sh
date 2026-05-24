#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TMPDIR="${TMPDIR:-/tmp}"
STAMP="${STAMP:-smoke-$(date +%Y%m%d-%H%M%S)}"
OUT_JSON="${OUT_JSON:-$TMPDIR/ontopack-perf-$STAMP.json}"
OUT_MD="${OUT_MD:-$TMPDIR/ontopack-perf-$STAMP.md}"
NOTE_COUNT="${NOTE_COUNT:-200}"
MEDIA_COUNT="${MEDIA_COUNT:-20}"
REPEATS="${REPEATS:-2}"
WARMUP="${WARMUP:-1}"
P95_MAX_MS="${P95_MAX_MS:-750}"
PACK_BIN="${PACK_BIN:-$ROOT/target/debug/pack}"
SKIP_BUILD="${SKIP_BUILD:-0}"

NOTE_COUNT="$NOTE_COUNT" \
MEDIA_COUNT="$MEDIA_COUNT" \
REPEATS="$REPEATS" \
WARMUP="$WARMUP" \
OUT_JSON="$OUT_JSON" \
OUT_MD="$OUT_MD" \
PACK_BIN="$PACK_BIN" \
SKIP_BUILD="$SKIP_BUILD" \
"$ROOT/scripts/perf-benchmark.sh" >/tmp/ontopack-perf-smoke.out

python3 - "$OUT_JSON" "$P95_MAX_MS" <<'PY'
import json, sys
path = sys.argv[1]
limit = float(sys.argv[2])
with open(path, encoding='utf-8') as f:
    report = json.load(f)
required = {'capabilities', 'dashboard_all', 'search_common', 'gallery', 'timeline', 'graph', 'note_detail', 'related'}
missing = required.difference(report['results'])
if missing:
    raise SystemExit(f'missing perf endpoints: {sorted(missing)}')
slow = {
    name: data['wall_ms']['p95']
    for name, data in report['results'].items()
    if data['wall_ms']['p95'] > limit
}
if slow:
    raise SystemExit(f'perf p95 over {limit}ms: {slow}')
print(f"perf smoke ok: notes={report['note_count']} media={report['media_count']} p95_limit_ms={limit}")
PY

echo "perf smoke artifacts: $OUT_JSON $OUT_MD"
