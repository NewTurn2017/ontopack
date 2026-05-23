# OntoPack M5A media preview QA — 2026-05-23

## Scope

Verify the first M5A implementation: safe local asset serving plus visible image/video previews in the embedded mechanical-vault viewer.

## Automated checks

- `cargo test -q`: passed.
- `scripts/real-test.sh`: passed.
- `KEEP_REAL_TEST_PACK=1 scripts/real-test.sh`: passed and kept pack at `/var/folders/6w/ryvjgm214g361w38k2x2dcch0000gn/T//ontopack-real-test.Kx1Ejl`.

## Browser QA

- URL: `http://127.0.0.1:59413`
- Scenario:
  1. Load viewer.
  2. Confirm gallery renders 3 media cards: 2 images and 1 video.
  3. Search `다이어그램` and confirm source card includes an image preview.
  4. Select `demo-video` from gallery and confirm selected-note panel renders a `<video controls preload="metadata">` preview.
  5. Inspect browser console.
- Observed:
  - Gallery: 2 images, 1 video, 3 media cards.
  - Search `다이어그램`: 1 result card, 1 image preview, title `온톨로지 다이어그램`.
  - Selected note: video preview rendered, title `데모 비디오`.
  - Browser console: 0 errors, 0 warnings.
- Screenshot: `output/playwright/ontopack-media-preview-m5a-20260523.png`

## Visual verdict

- Score: 90
- Verdict: M5A pass.
- Notes: The viewer now shows actual local media inside the vault UI. The video test asset is intentionally tiny/fake, so the browser control appears as a metadata preview rather than playable content, but the routing and UI rendering are verified.
