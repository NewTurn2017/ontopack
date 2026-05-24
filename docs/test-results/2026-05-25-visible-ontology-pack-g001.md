# Visible ontology pack G001 evidence (2026-05-25)

Goal: create a real, visually inspectable OntoPack from free/public web material, with prompt/image/video records visible through search, gallery, and graph APIs.

## Source APIs used

- Civitai Images API: prompt metadata, `nsfw=false`, local blocklist second pass. Reference: <https://github.com/civitai/civitai/wiki/REST-API-Reference/de63434512878133a5788a25f4b94af0c06de4bc>
- Wikimedia Commons / MediaWiki `imageinfo`: free/public image metadata and thumbnail URLs. Reference: <https://www.mediawiki.org/wiki/API:Imageinfo>
- Internet Archive advanced search: public video metadata and thumbnail URLs. Reference: <https://doc-tools.readthedocs.io/en/ia-test-gsod/item-search-apis.html>

## Implementation slice

- Added `scripts/visible-ontology-pack.py` as a no-new-dependency runner using Python stdlib.
- Default asset policy is metadata + remote preview thumbnails; original image/video downloads remain off unless a future explicit ingest command opts in.
- Notes preserve provenance in `.pack/provenance/*.jsonl` and frontmatter fields used by the viewer/API:
  - `remote_url`
  - `thumbnail_url`
  - `media_kind`
  - `mime`
- Core index and server APIs now keep remote preview metadata in search cards, note details, and gallery items.

## Verification run

```bash
cargo test
```

Result: all workspace tests passed.

```text
pack-cli tests: 42 passed
pack-core tests: 57 passed
pack-mcp tests: 10 passed
pack-server tests: 34 passed
```

### Offline deterministic fixture, 100/100/100

```bash
python3 scripts/visible-ontology-pack.py \
  --fixture --limit-each 100 --no-download-assets \
  --output /tmp/ontopack-visible-fixture-remote.RyN2hW \
  --pack-bin /Users/genie/dev/ontopack/target/debug/pack \
  --build
```

Evidence:

```text
counts: prompt=100 image=100 video=100
notes: 304
assets: 0
provenance_records: 300
fixture_remote_dashboard: gallery=20 graph_nodes=120 graph_edges=116
```

### Live public-web run, 100/100/100

```bash
python3 scripts/visible-ontology-pack.py \
  --limit-each 100 --no-download-assets \
  --output /tmp/ontopack-visible-live-remote.N6u1pu \
  --pack-bin /Users/genie/dev/ontopack/target/debug/pack \
  --build
```

Evidence:

```text
counts: prompt=100 image=100 video=100
notes: 304
assets: 0
provenance_records: 300
live_remote_dashboard: gallery=30 graph_nodes=180 graph_edges=352
live_remote_video_gallery: 5 items, asset_url=https://archive.org/services/img/10000000-3086101131533958-2836509322436032543-n
```

The live run proves:

- prompt records are searchable text chunks;
- image records render remote Wikimedia thumbnail URLs in gallery/search cards;
- video records render Internet Archive preview thumbnails through `/api/gallery?type=video`;
- graph API returns a browsable ontology with platform/concept links;
- no original images/videos were downloaded in the bulk run.

## Known follow-up

This is a script-backed runner, not yet the final first-class `pack ingest civitai ...` CLI. The next goals should promote this into a typed ingest command with pagination/rate-limit configuration and richer first-class source metadata.
