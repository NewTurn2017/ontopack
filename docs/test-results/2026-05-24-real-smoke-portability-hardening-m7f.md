# M7F real smoke portability hardening — 2026-05-24

Goal: move bundle/archive safety checks from focused integration tests into the realistic end-to-end smoke path.

## Added smoke coverage

`scripts/real-test.sh` now verifies:

- `pack bundle <dir> --archive <file.tar.gz>` creates both directory and archive artifacts.
- Directory bundle import restores media bytes exactly (`cmp`).
- Re-import without `--overwrite` fails on existing note/asset state.
- Re-import with `--overwrite` succeeds.
- `.tar.gz` archive import restores media bytes exactly.
- A broken bundle with a missing referenced asset fails and leaves no partially restored sidecar note.

## Validation

- `scripts/real-test.sh`

## Known gaps

- This smoke still runs on the current macOS environment; Windows execution remains documented as unverified.
