# OntoPack mechanical vault UI upgrade plan

## Direction

The target direction is not a generic AI dashboard. It is a private knowledge vault: mechanical lock UI, industrial cyberpunk panels, dark metal surfaces, phosphor-green terminal accents, and source-card archive workflows.

Reference keywords:

- Mechanical Lock UI
- Industrial Cyberpunk
- Secure Knowledge Vault
- Neo Terminal
- Dark Metal + Green Phosphor
- AI Archive System
- Private Intelligence Console

`DESIGN.md` is the design contract for this plan.

## Current state review

Current viewer strengths:

- Functional no-build embedded viewer in `crates/pack-server/src/viewer.rs`.
- Search, filters, Ask context, note detail, related, gallery, timeline, graph already work.
- Real browser QA is available and recently hardened.
- No frontend framework or asset pipeline keeps the CLI tool portable.

Current visual gap:

- The UI still reads as a simple light MVP dashboard.
- There is no vault/lock/secure archive metaphor in layout, panel shape, controls, or status copy.
- Search is useful but not visually central enough.
- Panels do not yet communicate “local private archive / source-card console.”

## Upgrade strategy

Keep the current embedded HTML/CSS/JS architecture. Upgrade the visual system and interaction hierarchy first; add new backend API only if a visual module truly needs it.

### Phase 0 — baseline and guardrails

Status: ready.

Tasks:

- Keep `DESIGN.md` as the decision source.
- Use current `scripts/real-test.sh` and browser QA as the regression gate.
- Capture before/after screenshots.

Acceptance:

- Existing tests pass before UI work starts.
- Reference image and `DESIGN.md` are linked in implementation notes.

### Phase 1 — dark vault shell

Goal: change first impression from “light MVP tool” to “private archive console.”

Tasks:

- Add CSS tokens for dark metal, phosphor green, muted text, bevel borders, panel shadows.
- Replace light page background with layered dark metal/terminal grid using CSS gradients.
- Add top mechanical rail with brand, local/private status, and non-route nav labels.
- Add left/hero brand lockup: `OntoPack`, `UNLOCK YOUR KNOWLEDGE`, `LOCALHOST VAULT`.
- Convert `.panel`, `.card`, form controls, buttons to beveled mechanical components.

Acceptance:

- Viewer loads with no external assets/CDN.
- Search, Ask, note, related, gallery, timeline, graph remain visible and usable.
- Browser console remains clean.
- Text contrast remains readable.

### Phase 2 — dashboard information architecture

Goal: make the existing data feel like vault modules.

Tasks:

- Create dashboard grid areas:
  - system overview / facet stats
  - central smart search console
  - note detail module
  - Ask context terminal module
  - archive modules: gallery, timeline, graph
- Add module headers with status glyphs and metadata labels.
- Add result counts and current filter summary to the search module.
- Make selected source card visually active.

Acceptance:

- Searching `온톨로지` shows source cards in the central console.
- Filter stress `공통질문 + prompt + needle + dates` still returns exactly `filter-target`.
- Clicking source cards still opens note detail and related notes.

### Phase 3 — mechanical controls and interaction states

Goal: make controls feel hardware-like without hurting accessibility.

Tasks:

- Style inputs/selects as terminal/mechanical controls.
- Add button variants: primary query, secondary filters, disabled/future nav.
- Add focus-visible phosphor outline.
- Add loading/empty/error copy per panel:
  - `QUERYING VAULT...`
  - `NO MATCHING SOURCE CARDS`
  - `CONTEXT BLOCKS READY`
- Respect `prefers-reduced-motion` for glow/scanline effects.

Acceptance:

- Keyboard focus is visible on inputs, selects, buttons, cards.
- Empty and error states are readable, not just console errors.
- `cargo test` covers any changed static HTML/JS expectations where practical.

### Phase 4 — browser visual QA loop

Goal: prove the upgrade in a real browser, not just API smoke.

Tasks:

- Run `scripts/real-test.sh`.
- Launch a kept test pack with `pack serve`.
- Use Playwright CLI to verify:
  - initial shell
  - search results
  - filters
  - note detail
  - Ask context
  - gallery/timeline/graph
  - console 0 errors/warnings
- Save screenshot under `output/playwright/`.
- Record result under `docs/test-results/`.

Acceptance:

- `cargo fmt --check`
- `cargo test -q`
- `cargo clippy --all-targets -- -D warnings`
- `scripts/real-test.sh`
- Browser QA screenshot and written evidence

## Recommended implementation order

1. CSS token + dark shell only.
2. Panel/card/control restyle.
3. Header/top rail and brand lockup.
4. Dashboard grid rearrangement.
5. Result count/filter summary/selected state.
6. Loading/empty/error copy.
7. Browser QA and screenshot evidence.

This order keeps each diff reviewable and avoids breaking working API flows while changing visual density.

## Risks and mitigations

- Risk: sci-fi styling reduces readability.
  - Mitigation: off-white body text, AA contrast, no long green body text.
- Risk: dense desktop layout breaks mobile.
  - Mitigation: CSS grid with single-column fallback.
- Risk: visual nav implies missing routes.
  - Mitigation: mark future nav as disabled/ornamental until routes exist.
- Risk: adding assets complicates packaging.
  - Mitigation: CSS/inline SVG only for first upgrade.
- Risk: browser concurrency regressions.
  - Mitigation: keep Playwright browser QA in final gate.

## Post-upgrade candidates

- Dedicated `/assets/*` static serving if custom logo/lock assets are approved.
- Real graph visualization once graph API and expected scale stabilize.
- Server vector/hybrid controls once API supports real-embed search in viewer.
- Korean tokenizer/stemming improvements for search quality.
