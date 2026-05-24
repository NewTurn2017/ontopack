# M7 doctor command validation

## Claim

OntoPack now has a productization-oriented `pack doctor` command for local installation and pack health checks.

## Scope

- `pack doctor` reports executable path, cwd, discovered pack root, required pack paths, and index availability.
- `pack doctor --json` emits the same report as stable JSON for scripts/installers.
- The command is read-only; it does not refresh manifests or rewrite source markdown.
- `scripts/real-test.sh` runs doctor after building a realistic pack and asserts a healthy index is reported.

## Evidence

- `cargo test -p pack-cli doctor_reports_pack_health_without_mutating` — passed.
- `scripts/real-test.sh` — passed with doctor text and JSON assertions on a realistic pack.
- `cargo test` — passed.
- `cargo clippy --all-targets -- -D warnings` — passed.
- `cargo check -p pack-cli --features real-embed` — passed.
- `git diff --check` — passed.

## Notes

This is the environment-check foundation for future installer/shell-completion work. It intentionally does not install files or mutate the pack.
