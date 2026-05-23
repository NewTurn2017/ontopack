# OntoPack Vault UI Phase 1 QA — 2026-05-23

## Scope

Upgrade the embedded viewer from a plain local search page into the first pass of the OntoPack mechanical knowledge-vault direction defined in `DESIGN.md`.

## Browser smoke

- URL: `http://127.0.0.1:50966`
- Realistic pack: `/var/folders/6w/ryvjgm214g361w38k2x2dcch0000gn/T//ontopack-real-test.ogSCC4`
- Scenario:
  1. Load viewer shell.
  2. Search `온톨로지`.
  3. Verify 4 result cards render.
  4. Ask context for `로컬`.
  5. Verify 4 context cards render.
  6. Open the first result and verify selected-note detail renders.
- Browser console: 0 errors, 0 warnings.
- Screenshot: `output/playwright/ontopack-vault-ui-phase1-20260523.png`

## Visual verdict

- Score: 88
- Verdict: Phase 1 pass with Phase 2 polish.
- Category match: true.
- Remaining polish: add more reusable mechanical ornaments, panel corner plates, lock/core accents, and richer status modules in a later pass without breaking the embedded/no-build viewer constraint.

## Functional expectations preserved

- Search/filter controls still use existing viewer APIs.
- Ask-context still returns deterministic context blocks for external LLM synthesis.
- Note detail, related, gallery, timeline, and graph panels remain accessible.
- The viewer still ships as embedded Rust server HTML/CSS/JS with no new frontend build step.
