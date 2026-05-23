#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TMPDIR="${TMPDIR:-/tmp}"
PACK_DIR="${PACK_DIR:-$(mktemp -d "$TMPDIR/ontopack-media-intel.XXXXXX")}"
WHISPER_PACK_DIR="$PACK_DIR-whisper"
KEEP_MEDIA_TEST_PACK="${KEEP_MEDIA_TEST_PACK:-0}"
RUN_REAL_WHISPER="${RUN_REAL_WHISPER:-0}"
PACK_BIN="${PACK_BIN:-$ROOT/target/debug/pack}"

cleanup() {
  if [[ "$KEEP_MEDIA_TEST_PACK" != "1" ]]; then
    rm -rf "$PACK_DIR"
    rm -rf "$WHISPER_PACK_DIR"
  else
    echo "media intelligence test pack kept: $PACK_DIR"
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

if ! command -v ffmpeg >/dev/null 2>&1; then
  echo "ffmpeg is required for media intelligence test" >&2
  exit 2
fi

echo "[1/7] build debug pack binary"
cargo build --quiet -p pack-cli --manifest-path "$ROOT/Cargo.toml"

echo "[2/7] seed pack: $PACK_DIR"
"$PACK_BIN" init "$PACK_DIR" >/tmp/ontopack-media-init.out
SOURCE_MP4="$PACK_DIR/source-media-intel.mp4"
ffmpeg -y \
  -f lavfi -i testsrc2=size=160x90:rate=1:duration=3 \
  -f lavfi -i sine=frequency=880:duration=3 \
  -shortest -pix_fmt yuv420p \
  "$SOURCE_MP4" >/tmp/ontopack-media-ffmpeg-generate.out 2>&1

(cd "$PACK_DIR" && "$PACK_BIN" add "$SOURCE_MP4" --type video >/tmp/ontopack-media-add.out)
grep -q '추가:' /tmp/ontopack-media-add.out
(cd "$PACK_DIR" && "$PACK_BIN" build --no-embed >/tmp/ontopack-media-build.out)

echo "[3/7] local provider enriches video into metadata + keyframes"
(cd "$PACK_DIR" && \
  ONTOPACK_PROVIDER_MODE=local \
  OPENAI_API_KEY="" \
  ONTOPACK_EXTRACT_KEYFRAMES=1 \
  "$PACK_BIN" enrich-pending --provider-command "$ROOT/scripts/providers/auto_media_worker.py" --limit 1 \
  >/tmp/ontopack-media-enrich.out)
grep -q 'processed=1' /tmp/ontopack-media-enrich.out
grep -q 'indexed=' /tmp/ontopack-media-enrich.out

SIDECAR="$PACK_DIR/notes/source-media-intel.md"
grep -q '## Keyframes' "$SIDECAR"
grep -q 'assets/.derived/source-media-intel/keyframe-0000.jpg' "$SIDECAR"
test -s "$PACK_DIR/assets/.derived/source-media-intel/keyframe-0000.jpg"

echo "[4/7] CLI search sees generated media intelligence"
SEARCH_OUT="$(cd "$PACK_DIR" && "$PACK_BIN" search "Representative video frame" --mode keyword -k 3)"
printf '%s\n' "$SEARCH_OUT" | grep -q 'source-media-intel#'

echo "[5/7] viewer APIs expose keyframe asset URLs"
NOTE_JSON="$(serve_json $'GET /api/notes/source-media-intel HTTP/1.1\r\nHost: localhost\r\n\r\n')"
assert_json "note API exposes keyframe asset_url" "$NOTE_JSON" 'v["media_kind"] == "video" and len(v["keyframes"]) >= 1 and v["keyframes"][0]["asset_url"].startswith("/assets/.derived/source-media-intel/keyframe-")'
GALLERY_JSON="$(serve_json $'GET /api/gallery?k=5 HTTP/1.1\r\nHost: localhost\r\n\r\n')"
assert_json "gallery exposes keyframes for video card" "$GALLERY_JSON" 'any(item["id"] == "source-media-intel" and len(item["keyframes"]) >= 1 for item in v["items"])'
(cd "$PACK_DIR" && "$PACK_BIN" serve --port 0 --once --request $'GET /assets/.derived/source-media-intel/keyframe-0000.jpg HTTP/1.1\r\nHost: localhost\r\n\r\n' >/tmp/ontopack-media-keyframe-response.bin)
test -s /tmp/ontopack-media-keyframe-response.bin

echo "[6/7] optional real whisper gate"
if [[ "$RUN_REAL_WHISPER" == "1" ]]; then
  if [[ -z "${WHISPER_MODEL:-}" && -z "${WHISPER_CPP_MODEL:-}" ]]; then
    echo "RUN_REAL_WHISPER=1 requires WHISPER_MODEL or WHISPER_CPP_MODEL" >&2
    exit 2
  fi
  "$PACK_BIN" init "$WHISPER_PACK_DIR" >/dev/null
  cp "$SOURCE_MP4" "$WHISPER_PACK_DIR/source-media-intel.mp4"
  (cd "$WHISPER_PACK_DIR" && "$PACK_BIN" add "$WHISPER_PACK_DIR/source-media-intel.mp4" --type video >/dev/null)
  (cd "$WHISPER_PACK_DIR" && "$PACK_BIN" build --no-embed >/dev/null)
  (cd "$WHISPER_PACK_DIR" && \
    ONTOPACK_PROVIDER_MODE=local OPENAI_API_KEY="" "$PACK_BIN" enrich-pending --provider-command "$ROOT/scripts/providers/auto_media_worker.py" --limit 1 \
    >/tmp/ontopack-media-whisper.out)
  grep -q 'processed=1' /tmp/ontopack-media-whisper.out
  grep -q '## Transcript' "$WHISPER_PACK_DIR/notes/source-media-intel.md"
else
  echo "skip real whisper runtime; set RUN_REAL_WHISPER=1 and WHISPER_MODEL=/path/to/ggml-model.bin"
fi

echo "[7/7] media intelligence summary"
echo "Ontopack media intelligence test passed: real mp4 + local provider + derived keyframes + search + API"
