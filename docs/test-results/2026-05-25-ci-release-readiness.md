# CI and release readiness foundation — 2026-05-25

Goal: move OntoPack from source-build-only pre-release toward an operationally releasable CLI/MCP toolchain.

## Added release-readiness surface

- Added an MIT `LICENSE` file so third-party source use has an explicit permissive license before wider release.
- Added Cargo package metadata for each workspace crate: description, license, repository, homepage, and root README reference.
- Added `.github/workflows/ci.yml` with macOS, Linux, and Windows jobs:
  - `cargo fmt --all -- --check`
  - `cargo test --workspace --locked`
  - `cargo clippy --workspace --all-targets --locked -- -D warnings`
  - release binary build for `pack` and `pack-mcp`
  - Unix MVP smoke on macOS/Linux
  - live `scripts/windows-smoke.ps1` on Windows
- Added `.github/workflows/release.yml` for tag/workflow-dispatch builds that package prebuilt `pack` and `pack-mcp` artifacts plus README/LICENSE, then publish tag releases with checksums.
- Ignored `.understand-anything/` because knowledge graph/dashboard outputs are local generated analysis artifacts, not source.

## Local validation

Executed on the current macOS checkout:

```bash
cargo metadata --locked --no-deps
cargo fmt --check
cargo test -q
cargo clippy --all-targets -- -D warnings
scripts/mvp-smoke.sh
scripts/real-test.sh
```

Result: passed locally before pushing CI workflows. `scripts/real-test.sh` used the default no-download path and skipped the optional real embedding runtime, as expected unless `RUN_REAL_EMBED=1` is set.

## CI-only validation expected after push

The new workflow must prove the remaining live-platform gap:

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\windows-smoke.ps1 -PackBin .\target\release\pack.exe
```

## Known follow-up

- The GitHub-hosted Windows job is the first real Windows proof; if it exposes PowerShell/path differences, fix the script or CLI path handling before publishing a release.
- Release artifacts are unsigned. Add signing/notarization only after the binary release path is stable.
- CI currently runs the MVP smoke on Unix and the Windows portability smoke on Windows; the heavier `scripts/real-test.sh`, real embedding, and large performance benchmark remain manual or future scheduled gates.
