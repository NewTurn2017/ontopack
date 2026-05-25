# CLI install + setup/doctor review

Date: 2026-05-26

## Claim

OntoPack's installed CLI path now has a clearer first-run contract:

- `pack --version` works for release/support verification.
- `pack setup [--shell bash|zsh|fish]` prints a first-run checklist.
- `pack doctor` still diagnoses install/pack health and now prints actionable `next:` guidance when setup is incomplete.
- shell completions include the new `setup` command.

## Local install check

Installed from source into the user's local prefix:

```bash
scripts/install.sh --prefix "$HOME/.local" --completion-shell zsh
```

Observed:

- binary installed at `$HOME/.local/bin/pack`
- zsh completion installed at `$HOME/.local/share/zsh/site-functions/_pack`

## Validation commands

```bash
cargo fmt --check
cargo test -p pack-cli
scripts/install.sh --prefix "$HOME/.local" --completion-shell zsh
pack --version
pack setup --shell zsh
pack doctor
```

## Known gaps

- `pack doctor` intentionally remains non-mutating and exits successfully even when `ok=false`; scripts should read `--json` when they need a machine gate.
- `pack setup` is a checklist, not an installer. The mutating install path remains `scripts/install.sh` until packaged releases are published.

Additional installed-binary smoke:

```bash
tmp=$(mktemp -d)
cd "$tmp"
pack init .
printf 'hello setup doctor\n' > notes/hello.md
pack build --incremental
pack doctor
pack doctor --json
```

Observed `doctor: ok=true` and JSON `ok=true` after pack initialization and index build.
