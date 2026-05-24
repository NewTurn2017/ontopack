# M7 shell completions validation

## Claim

OntoPack now emits shell completion scripts for bash, zsh, and fish without adding runtime or build dependencies.

## Scope

- `pack completions zsh` prints a zsh completion script with `#compdef pack`.
- `pack completions bash` prints a bash `complete -F _pack_completions pack` script.
- `pack completions fish` prints a fish `complete -c pack` script.
- The completion command includes current top-level commands such as `watch` and `doctor`.
- `scripts/real-test.sh` asserts zsh and bash scripts are emitted in the realistic smoke path.

## Evidence

- `cargo test -p pack-cli completions_print_supported_shell_scripts` — passed.
- `scripts/real-test.sh` — passed with completion script assertions.
- `cargo test` — passed.
- `cargo clippy --all-targets -- -D warnings` — passed.
- `cargo check -p pack-cli --features real-embed` — passed.
- `git diff --check` — passed.

## Notes

This is intentionally a lightweight top-level command completion. Rich option/value completion can be added later, preferably after deciding whether adding `clap_complete` is worth the dependency cost.
