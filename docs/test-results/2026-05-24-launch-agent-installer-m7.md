# M7 macOS LaunchAgent installer validation

## Claim

OntoPack now has a safe macOS LaunchAgent plist generator for running `pack watch` as a login background job.

## Scope

- `scripts/install-launch-agent.sh --pack-root DIR` emits a LaunchAgent plist to stdout by default.
- `--output PATH` writes the plist to an explicit path for review/testing.
- `--install` writes to `~/Library/LaunchAgents` or `PLIST_DIR`, but the script does not call `launchctl` automatically.
- The plist uses `ProgramArguments = [pack, watch, --interval-ms, N]`, `WorkingDirectory = pack root`, `RunAtLoad = true`, `KeepAlive = true`, and pack-local `.pack/watch.*.log` paths.
- `scripts/real-test.sh` generates a plist in `/tmp`, parses it with Python `plistlib`, and verifies label, arguments, working directory, restart flags, and log paths.

## Evidence

- `bash -n scripts/install-launch-agent.sh` — passed during implementation.
- `scripts/real-test.sh` — passed with LaunchAgent plist parsing assertions.
- `cargo test` — passed.
- `cargo clippy --all-targets -- -D warnings` — passed.
- `cargo check -p pack-cli --features real-embed` — passed.
- `git diff --check` — passed.

## Notes

This slice intentionally avoids running `launchctl` in tests or automatically mutating the user's live login session. Users can review the generated plist and then run the printed `launchctl bootstrap` / `bootout` commands when using `--install`.
