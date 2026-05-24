# M7 Windows smoke script validation

## Claim

OntoPack now has an executable Windows smoke contract for validating storage-format portability and core CLI behavior on a Windows runner.

## Scope

- `scripts/windows-smoke.ps1` initializes a temporary pack, writes a note with a Windows path, builds the index, searches it, runs `pack doctor`, exports JSONL, creates a bundle archive, imports the archive into a restored pack, rebuilds, and searches again.
- The script uses PowerShell-native file/path APIs and avoids Bash/GNU tool assumptions.
- The current macOS real-test statically verifies the PowerShell smoke contains the required core commands and success marker.
- Actual Windows execution remains pending until a Windows machine/runner is available.

## Evidence

- `scripts/real-test.sh` — passed with Windows smoke contract assertions.
- `cargo test` — passed.
- `cargo clippy --all-targets -- -D warnings` — passed.
- `cargo check -p pack-cli --features real-embed` — passed.
- `git diff --check` — passed.

## How to run on Windows

```powershell
.\scripts\windows-smoke.ps1 -PackBin .\target\release\pack.exe
```

Use `-KeepPack` to preserve the generated temporary pack for inspection.

## Notes

This is not a substitute for live Windows proof; it is the committed runner contract. Provider-heavy Windows checks still need a live environment with ffmpeg/ffprobe, tesseract, ollama, whisper, and Python provider command handling.
