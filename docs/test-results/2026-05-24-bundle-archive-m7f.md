# M7F portable bundle archive validation — 2026-05-24

Goal: make a portable bundle easier to move as one compressed file while preserving the directory bundle as the canonical internal layout.

## Added CLI surface

- `pack bundle <dir> --archive <file.tar.gz>` still writes the bundle directory first, then wraps that exact layout as a gzip-compressed tar archive.
- `pack import <file.tar.gz>` extracts to a temporary directory and then uses the same `bundle.json` manifest validation and import preflight path as directory import.

## Dependency decision

Rust stdlib does not provide tar or gzip archive support. The implementation uses:

- `flate2` for gzip streams. This crate was already present in `Cargo.lock` transitively.
- `tar` for tar archive read/write. This is a small MIT/Apache-2.0 crate from the Rust ecosystem. It is added with `default-features = false` to avoid xattr support and keep the surface minimal.

Rejected alternatives:

- Shelling out to system `zip`/`tar`: less portable and harder to test consistently.
- Replacing the directory bundle with archive-only output: would weaken the debuggable canonical layout.

## Validation

- `cargo test -p pack-cli bundle_archive_imports_with_same_manifest_contract -- --exact`

The test creates a directory bundle and `.tar.gz`, imports from the archive into a fresh pack, checks restored asset bytes, rebuilds indexes, and search-validates the restored note.

## Known gaps

- Only `.tar.gz` / `.tgz` bundle archives are recognized.
- Archive extraction is temporary and then routed through normal bundle manifest validation; malformed gzip/tar failures are surfaced as import errors.
