#!/usr/bin/env bash
set -euo pipefail

LABEL="${LABEL:-com.ontopack.watch}"
PACK_BIN="${PACK_BIN:-pack}"
PACK_ROOT="${PACK_ROOT:-}"
INTERVAL_MS="${INTERVAL_MS:-1000}"
OUTPUT=""
INSTALL=0
PLIST_DIR="${PLIST_DIR:-$HOME/Library/LaunchAgents}"

usage() {
  cat <<'USAGE'
Usage: scripts/install-launch-agent.sh --pack-root DIR [options]

Options:
  --pack-root DIR       OntoPack root to watch (required)
  --pack-bin PATH       pack executable path (default: pack)
  --label LABEL         LaunchAgent label (default: com.ontopack.watch)
  --interval-ms N       pack watch polling interval in ms (default: 1000)
  --output PATH         write plist to PATH instead of stdout/default install path
  --install             write plist to ~/Library/LaunchAgents (or PLIST_DIR) and print launchctl guidance
  --plist-dir DIR       LaunchAgents directory for --install (default: ~/Library/LaunchAgents)
  -h, --help            show help

This script only writes the plist. It does not run launchctl automatically.
USAGE
}

xml_escape() {
  python3 - "$1" <<'PY'
import html, sys
print(html.escape(sys.argv[1], quote=False))
PY
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --pack-root)
      PACK_ROOT="${2:?--pack-root requires a directory}"
      shift 2
      ;;
    --pack-bin)
      PACK_BIN="${2:?--pack-bin requires a path}"
      shift 2
      ;;
    --label)
      LABEL="${2:?--label requires a value}"
      shift 2
      ;;
    --interval-ms)
      INTERVAL_MS="${2:?--interval-ms requires a value}"
      shift 2
      ;;
    --output)
      OUTPUT="${2:?--output requires a path}"
      shift 2
      ;;
    --install)
      INSTALL=1
      shift
      ;;
    --plist-dir)
      PLIST_DIR="${2:?--plist-dir requires a directory}"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "unknown argument: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

if [[ -z "$PACK_ROOT" ]]; then
  echo "--pack-root is required" >&2
  usage >&2
  exit 2
fi
if [[ ! -d "$PACK_ROOT" ]]; then
  echo "pack root does not exist: $PACK_ROOT" >&2
  exit 1
fi
if [[ ! "$INTERVAL_MS" =~ ^[0-9]+$ ]] || [[ "$INTERVAL_MS" -lt 100 ]]; then
  echo "--interval-ms must be an integer >= 100" >&2
  exit 2
fi

PACK_ROOT_ABS="$(cd "$PACK_ROOT" && pwd)"
PACK_BIN_ESCAPED="$(xml_escape "$PACK_BIN")"
PACK_ROOT_ESCAPED="$(xml_escape "$PACK_ROOT_ABS")"
LABEL_ESCAPED="$(xml_escape "$LABEL")"
INTERVAL_ESCAPED="$(xml_escape "$INTERVAL_MS")"

PLIST_BODY="$(cat <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key>
  <string>$LABEL_ESCAPED</string>
  <key>ProgramArguments</key>
  <array>
    <string>$PACK_BIN_ESCAPED</string>
    <string>watch</string>
    <string>--interval-ms</string>
    <string>$INTERVAL_ESCAPED</string>
  </array>
  <key>WorkingDirectory</key>
  <string>$PACK_ROOT_ESCAPED</string>
  <key>RunAtLoad</key>
  <true/>
  <key>KeepAlive</key>
  <true/>
  <key>StandardOutPath</key>
  <string>$PACK_ROOT_ESCAPED/.pack/watch.out.log</string>
  <key>StandardErrorPath</key>
  <string>$PACK_ROOT_ESCAPED/.pack/watch.err.log</string>
</dict>
</plist>
PLIST
)"

if [[ "$INSTALL" == "1" ]]; then
  mkdir -p "$PLIST_DIR"
  OUTPUT="$PLIST_DIR/$LABEL.plist"
elif [[ -z "$OUTPUT" ]]; then
  printf '%s\n' "$PLIST_BODY"
  exit 0
fi

mkdir -p "$(dirname "$OUTPUT")"
printf '%s\n' "$PLIST_BODY" >"$OUTPUT"
echo "wrote launch agent plist: $OUTPUT"
if [[ "$INSTALL" == "1" ]]; then
  echo "load with: launchctl bootstrap gui/$(id -u) '$OUTPUT'"
  echo "unload with: launchctl bootout gui/$(id -u) '$OUTPUT'"
fi
