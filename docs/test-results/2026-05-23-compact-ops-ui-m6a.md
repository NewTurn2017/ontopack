# M6A compact ops UI validation

Date: 2026-05-23

## Goal

Make the OntoPack viewer denser and easier to scan at a glance without adding a frontend build step or changing the deterministic local APIs.

## Acceptance criteria

- The viewer root carries a `compact-ops` mode hook.
- Desktop layout uses a 3-column ops-console grid.
- Search, note, gallery, timeline, graph, related, and Ask panels remain visible as separate modules.
- Long card lists and note bodies scroll inside their own panels.
- Media previews remain visible but smaller so more knowledge context fits above the fold.
- Existing CLI/MCP/viewer real test stays green.

## Validation log

- `cargo fmt --check`: pass
- `cargo test -p pack-server viewer_uses_compact_ops_console_layout`: pass
- `cargo test`: pass (CLI 16, core 45, MCP 9, server 28, doctests 0)
- `cargo clippy --all-targets -- -D warnings`: pass
- `node --check /tmp/ontopack-app.js`: pass
- `scripts/real-test.sh`: pass (`realistic pack + CLI + MCP + viewer APIs + filter stress + open URL`)
- Browser QA at 1512x982: pass, zero console errors/warnings, search result selected, note opened, media/gallery/timeline/graph all visible.
- Screenshot: `output/playwright/ontopack-compact-ops-m6a-20260523.png`

## Browser QA evidence

```json
{
  "visible": {
    "shell": true,
    "columns": "530.922px 369.75px 341.328px",
    "search": true,
    "note": true,
    "gallery": true,
    "timeline": true,
    "graph": true,
    "mediaAboveTimeline": true,
    "resultCount": "1 SOURCE CARD · 0ms",
    "bodyHeight": 1091,
    "viewportHeight": 982,
    "rects": {
      "search": {
        "x": 234,
        "y": 256,
        "w": 531,
        "h": 615
      },
      "note": {
        "x": 777,
        "y": 256,
        "w": 370,
        "h": 301
      },
      "gallery": {
        "x": 1159,
        "y": 256,
        "w": 341,
        "h": 301
      },
      "timeline": {
        "x": 777,
        "y": 569,
        "w": 370,
        "h": 301
      },
      "graph": {
        "x": 1159,
        "y": 569,
        "w": 341,
        "h": 301
      },
      "ask": {
        "x": 234,
        "y": 883,
        "w": 913,
        "h": 197
      },
      "related": {
        "x": 1159,
        "y": 883,
        "w": 341,
        "h": 197
      }
    }
  },
  "badMessages": []
}
```
