# M7 watch loop validation

## Claim

OntoPack now has a productization-oriented watch command that can continuously process `_inbox`, incrementally rebuild the keyword index, and refresh the derived object manifest.

## Scope

- `pack watch --once` runs exactly one process/index/manifest cycle for deterministic tests and shell automation.
- `pack watch` repeats the same safe cycle with `--interval-ms` polling.
- The watch cycle stays offline and uses the existing incremental keyword/chunk index path; real embedding remains an explicit `pack embed` / `real-embed` action.
- `scripts/real-test.sh` seeds `_inbox/watch-real.md`, runs `pack watch --once`, then verifies the imported note is searchable.

## Evidence

- `cargo test -p pack-cli watch_once_processes_inbox_and_incrementally_indexes` — passed.
- `scripts/real-test.sh` — passed with watch-once indexing assertions on a realistic pack.
- `cargo test` — passed.
- `cargo clippy --all-targets -- -D warnings` — passed.
- `cargo check -p pack-cli --features real-embed` — passed.
- `git diff --check` — passed.

## Notes

This is a foreground polling loop, not an OS service installer. Launch agents, shell completion, and platform-specific packaging remain future M7 work.
