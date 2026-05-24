# M7 real embedding runtime validation

## Claim

The optional `real-embed` path is still functional after M5-M7 productization work.

## Scope

- Ran the full realistic smoke with `RUN_REAL_EMBED=1`.
- This exercises the normal realistic pack flow plus the optional release build with `--features real-embed`, `pack embed --skip-build`, and hybrid search.
- The default non-embed path remains the normal gate; real embeddings are explicit because model/cache/runtime state is heavier than ordinary CLI tests.

## Evidence

- `RUN_REAL_EMBED=1 scripts/real-test.sh` — passed.
- `cargo check -p pack-cli --features real-embed` — passed before the run.

## Notes

The test completed successfully in the current macOS environment. It did not preserve the temporary pack because `KEEP_REAL_TEST_PACK` was not set.
