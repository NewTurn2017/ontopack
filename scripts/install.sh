#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PREFIX="${PREFIX:-$HOME/.local}"
BIN_PATH=""
BUILD=1
COMPLETION_SHELL=""

usage() {
  cat <<'USAGE'
Usage: scripts/install.sh [--prefix DIR] [--bin PATH] [--no-build] [--completion-shell bash|zsh|fish]

Installs the OntoPack `pack` binary into PREFIX/bin.
By default it builds `pack` with `cargo build --release -p pack-cli` first.
Use --bin with --no-build for CI/tests or prebuilt binaries.
USAGE
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --prefix)
      PREFIX="${2:?--prefix requires a directory}"
      shift 2
      ;;
    --bin)
      BIN_PATH="${2:?--bin requires a path}"
      shift 2
      ;;
    --no-build)
      BUILD=0
      shift
      ;;
    --completion-shell)
      COMPLETION_SHELL="${2:?--completion-shell requires bash, zsh, or fish}"
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

if [[ "$BUILD" == "1" ]]; then
  cargo build --release -p pack-cli --manifest-path "$ROOT/Cargo.toml"
  BIN_PATH="$ROOT/target/release/pack"
elif [[ -z "$BIN_PATH" ]]; then
  echo "--no-build requires --bin PATH" >&2
  exit 2
fi

if [[ ! -x "$BIN_PATH" ]]; then
  echo "pack binary is not executable: $BIN_PATH" >&2
  exit 1
fi

install -d "$PREFIX/bin"
install -m 0755 "$BIN_PATH" "$PREFIX/bin/pack"
INSTALLED="$PREFIX/bin/pack"

echo "installed pack: $INSTALLED"
"$INSTALLED" --help >/dev/null

if [[ -n "$COMPLETION_SHELL" ]]; then
  case "$COMPLETION_SHELL" in
    bash)
      COMPLETION_PATH="$PREFIX/share/bash-completion/completions/pack"
      ;;
    zsh)
      COMPLETION_PATH="$PREFIX/share/zsh/site-functions/_pack"
      ;;
    fish)
      COMPLETION_PATH="$PREFIX/share/fish/vendor_completions.d/pack.fish"
      ;;
    *)
      echo "unsupported completion shell: $COMPLETION_SHELL" >&2
      exit 2
      ;;
  esac
  install -d "$(dirname "$COMPLETION_PATH")"
  "$INSTALLED" completions "$COMPLETION_SHELL" >"$COMPLETION_PATH"
  echo "installed completion: $COMPLETION_PATH"
fi
