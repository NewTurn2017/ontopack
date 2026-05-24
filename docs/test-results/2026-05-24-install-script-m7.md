# M7 install script validation

## Claim

OntoPack now has a local installer script that can build or install a prebuilt `pack` binary into a configurable prefix and optionally wire shell completions.

## Scope

- `scripts/install.sh --prefix DIR` builds `pack` with Cargo release mode and installs it to `DIR/bin/pack`.
- `--bin PATH --no-build` installs an already-built binary, which keeps CI/smoke tests fast and non-destructive.
- `--completion-shell bash|zsh|fish` writes completion output under the matching `share/` completion directory.
- `scripts/real-test.sh` installs the debug smoke binary into `/tmp/ontopack-real-install` and verifies the binary plus zsh completion file.

## Evidence

- `scripts/real-test.sh` — passed with install script assertions in a temporary prefix.
- `cargo test` — passed.
- `cargo clippy --all-targets -- -D warnings` — passed.
- `cargo check -p pack-cli --features real-embed` — passed.
- `git diff --check` — passed.

## Notes

This does not yet create a background service/launch agent. It is a safe local binary/completion installer foundation for future platform-specific packaging.
