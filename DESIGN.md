# Design

## Source of truth
- Status: Active draft for the OntoPack mechanical vault UI upgrade
- Last refreshed: 2026-05-23
- Primary product surfaces:
  - `pack open` / `pack serve` localhost viewer
  - Search, Ask context, note detail, related notes, gallery, timeline, graph panels
  - Future MCP/CLI docs that explain viewer mental model
- Evidence reviewed:
  - User-provided visual reference: mechanical lock UI, industrial cyberpunk, secure knowledge vault, neo terminal, dark metal + green phosphor, AI archive/private intelligence console
  - Current viewer implementation: `crates/pack-server/src/viewer.rs`
  - Current viewer docs: `docs/viewer.md`
  - System/run/performance plan: `docs/system-deep-dive.md`
  - Browser QA evidence: `docs/test-results/2026-05-23-browser-qa.md`, `output/playwright/ontopack-browser-qa-20260523.png`

## Brand
- Personality:
  - Private intelligence console, secure knowledge vault, mechanical archive machine
  - Serious, tactile, precise, self-hosted, hacker-workbench, not friendly SaaS dashboard
- Trust signals:
  - Localhost/private wording, source-card/citation language, visible system status, deterministic context blocks
  - Hardware-like controls that imply locked local ownership rather than cloud SaaS
- Avoid:
  - Generic pastel SaaS cards
  - Cartoon AI assistant visual language
  - Overly decorative sci-fi that harms reading, keyboard use, or small-screen operation
  - Fake cloud/security claims beyond actual localhost/private-pack behavior

## Product goals
- Goals:
  - Make OntoPack feel like a personal knowledge vault and archive terminal.
  - Keep the MVP viewer useful: search, filters, note detail, related, gallery, timeline, graph, Ask context must remain fast and readable.
  - Turn search into the visual center of the product: a vault query console rather than a plain form.
  - Reinforce that answers are context blocks/source cards, not hallucinated server-generated AI responses.
- Non-goals:
  - No heavy frontend framework for this upgrade.
  - No external asset pipeline or large image dependencies for the embedded MVP viewer.
  - No fake vector/hybrid UI controls until server API supports them.
  - No pixel-perfect clone of the reference image unless a later visual-ralph implementation target is explicitly approved.
- Success signals:
  - A new user can identify the app as a secure local archive within 5 seconds.
  - Search/filter/Ask flows still pass browser QA with zero console errors.
  - UI communicates source cards, local pack stats, selected-note context, and visible media previews clearly.
  - Images/videos added as asset sidecars are visible in the viewer, not only represented as metadata cards.
  - Viewer startup and filter/search interactions feel instant on realistic packs by using indexed reads and avoiding repeated markdown scans.
  - Visual richness comes from CSS tokens/panels/components, not brittle static artwork.

## Personas and jobs
- Primary personas:
  - Power user building a local second brain / lecture pack / research archive.
  - Developer-agent user connecting MCP/Codex/Claude to local source material.
  - Creator managing prompts, images, transcripts, and citations in one local pack.
- User jobs:
  - Search personal knowledge fast and see citation-ready source cards.
  - Inspect a note and related notes without losing search context.
  - Use Ask context as grounded material for an external LLM answer.
  - Browse media/asset sidecars, timeline, and graph summary as an archive overview.
- Key contexts of use:
  - Local desktop browser on `127.0.0.1`.
  - Developer workflow while terminal/test server is running.
  - Korean and English content mixed in the same pack.

## Information architecture
- Primary navigation:
  - MVP: single dashboard surface; tabs can be visual-only labels unless backed by routes later.
  - Future: Dashboard, Vault, Search, Workflow, Settings.
- Core routes/screens:
  - Current: `/` embedded viewer shell.
  - Current API: `/api/search`, `/api/ask`, `/api/facets`, `/api/gallery`, `/api/notes/<id>`, `/api/related/<id>`, `/api/timeline`, `/api/graph`.
- Content hierarchy:
  1. Header/top rail: OntoPack identity, local/private status, pack status.
  2. System overview: pack stats/facets summary.
  3. Central search console: query, type/tag/date controls, result cards.
  4. Context panels: Ask context, selected note, related notes.
  5. Archive panels: gallery, timeline, graph/system log.

## Design principles
- Principle 1: Mechanical, not ornamental.
  - Every visual detail should imply structure: lock, panel, rail, status, module, compartment.
- Principle 2: Local trust before AI magic.
  - Use terms like source cards, context blocks, vault, pack, local, indexed; avoid implying autonomous answer generation.
- Principle 3: Readability beats texture.
  - Dark metal and green phosphor are the theme, but body text/snippets need clear contrast and generous line-height.
- Principle 4: One visual system, no asset bloat.
  - Prefer CSS variables, gradients, borders, pseudo-elements, inline SVG/favicon, and Unicode/simple icons.
- Tradeoffs:
  - Dense dashboard aesthetics must be balanced with MVP accessibility and small-screen behavior.
  - Reference-image richness should be staged; do not implement all decorative panels before core flows are stable.

## Visual language
- Color:
  - Background: near-black blue/green metal, e.g. `#050b0d`, `#080f11`, `#0d1517`.
  - Panel metal: `#101719`, `#151b1d`, `#1f2a2d`.
  - Primary phosphor green: `#00ff9a`, `#19d27c`, `#35f2a1`.
  - Secondary cyan: `#4cc9f0` for info/data accents.
  - Warning/danger: restrained amber/red only for error states.
  - Text: cool off-white `#d8e3df`, muted `#7f918c`.
- Typography:
  - Current system fonts are acceptable for Korean readability.
  - Add mono/terminal styling for labels, metadata, IDs, and status lines.
  - Brand wordmark can use uppercase tracking and segmented/terminal treatment without requiring font assets.
- Spacing/layout rhythm:
  - Dashboard grid, dense but not cramped: 12-18px gaps, clear modules.
  - Top rail + left identity/status rail + main grid on desktop.
  - Single-column stacked modules on mobile.
- Shape/radius/elevation:
  - Use clipped corners, inset borders, bevel shadows, and hairline separators.
  - Keep radius modest; mechanical panels should feel machined, not pillowy SaaS.
- Motion:
  - Subtle glow/focus transitions, scanning line only if `prefers-reduced-motion` allows.
  - No constant distracting animation in reading areas.
- Imagery/iconography:
  - CSS/inline-symbol lock, database, search, graph, image/video/document icons.
  - No external icon set unless explicitly approved.

## Components
- Existing components to reuse:
  - Search form and results card renderer in `viewer.rs`.
  - Ask context cards.
  - Note detail, related cards, gallery, timeline, graph panels.
- New/changed components:
  - `media-preview`: local image/video preview for asset sidecars, with large selected-record preview and compact card thumbnails.
  - `media-bay`: gallery module that shows actual images/videos instead of metadata-only cards.
  - `app-shell`: dark vault background and dashboard layout.
  - `top-rail`: mechanical nav/status rail.
  - `brand-lockup`: OntoPack mark + local/private tagline.
  - `system-stat`: facet/count/status modules.
  - `vault-panel`: beveled panel container with header, close/status glyphs.
  - `control-button`: primary/secondary/danger/disabled visual states.
  - `terminal-log`: Ask/system status panel copy.
  - `source-card`: stronger metadata hierarchy with type, note id, chunk id.
- Variants and states:
  - Loading: terminal-line placeholder, not spinner-only.
  - Empty: muted vault-empty copy.
  - Error: red/amber panel strip with actionable text.
  - Active/selected: green glow + left rail marker.
  - Focus: visible phosphor outline, keyboard accessible.
- Token/component ownership:
  - Tokens live inside `viewer::style_css()` until/unless static asset serving is introduced.
  - Component classes should be named semantically and tested by visible text/roles, not brittle generated class names.

## Accessibility
- Target standard:
  - Practical WCAG AA for contrast, keyboard focus, semantic headings/buttons/forms.
- Keyboard/focus behavior:
  - Search input autofocus can remain.
  - All cards/buttons must keep visible focus states.
  - Decorative panels must not trap focus.
- Contrast/readability:
  - Green text on black is acceptable only for labels/accent; long snippets should use off-white.
- Screen-reader semantics:
  - Preserve form labels/aria labels.
  - Use real buttons for clickable cards as currently implemented.
- Reduced motion and sensory considerations:
  - Disable scanline/pulse effects under `prefers-reduced-motion: reduce`.

## Responsive behavior
- Supported breakpoints/devices:
  - Desktop-first for power-user archive workflow.
  - Tablet/laptop narrow widths must remain usable.
  - Mobile stacks all modules; top rail becomes compact.
- Layout adaptations:
  - Desktop: left identity rail + main dashboard grid + optional right status column.
  - Medium: two-column grid.
  - Small: single column, sticky search optional later.
- Touch/hover differences:
  - Hover glows are enhancements only; selected/focus states must work without hover.

## Interaction states
- Loading:
  - `INDEXING...`, `QUERYING...`, or skeleton terminal rows per panel.
- Empty:
  - Search: `NO MATCHING SOURCE CARDS` with current filters summarized.
  - Gallery: `NO ASSET SIDECARS`.
  - Related: `NO LINKED NOTES`.
- Error:
  - Show panel-local error copy; log to console only for unexpected developer errors.
- Success:
  - Results count/status line, selected-note status.
- Disabled:
  - Use muted metal buttons; do not hide unavailable capabilities.
- Offline/slow network:
  - Localhost only, but browser requests can still be slow; keep panel-level loading indicators.

## Content voice
- Tone:
  - Korean-first for explanatory UI, English/terminal labels for system style.
  - Concise, operational, archive/security vocabulary.
- Terminology:
  - `Vault`, `Source Card`, `Context Block`, `Local Pack`, `Index`, `Related`, `Timeline`, `Graph`.
  - Korean equivalents can appear in body copy: `지식 금고`, `출처 카드`, `컨텍스트 블록`.
- Microcopy rules:
  - Do not say the server “answers” questions; it returns context blocks.
  - Make local/private scope explicit.

## Implementation constraints
- Framework/styling system:
  - Current MVP is embedded HTML/CSS/JS in Rust (`crates/pack-server/src/viewer.rs`).
  - Keep no-build, no-framework approach unless a later milestone explicitly changes packaging.
- Design-token constraints:
  - CSS custom properties in `style_css()`.
  - Avoid generated assets; inline SVG favicon is acceptable.
- Performance constraints:
  - Keep CSS lightweight.
  - Avoid large animated backgrounds.
  - Keep API calls bounded and browser-concurrency-safe.
  - Prefer SQLite-backed API reads over reparsing `notes/` on every viewer request.
  - Batch dashboard startup data where possible and cancel stale browser requests.
  - Media previews must be lazy-loaded (`loading=lazy`, video `preload=metadata`) and served only from the local pack `assets/` directory.
- Compatibility constraints:
  - Local Chromium/Safari/Firefox basics.
  - No remote CDN dependency.
- Test/screenshot expectations:
  - `cargo fmt --check`, `cargo test`, `cargo clippy --all-targets -- -D warnings`, `scripts/real-test.sh`.
  - Browser QA via Playwright CLI with screenshot evidence under `output/playwright/`.
  - Console must be 0 errors/warnings for the tested flow.

## Open questions
- [ ] Should the next UI implementation be visual-reference matching (`$visual-ralph`) or a staged “inspired by reference” upgrade? Impact: determines strictness of screenshot scoring.
- [ ] Should top nav labels like Vault/Search/Workflow/Settings be functional routes now or disabled/visual future affordances? Impact: scope and API needs.
- [ ] Should Korean or English dominate visible chrome? Current recommendation: Korean body + English terminal labels.
- [ ] Are static assets acceptable later for a custom lock/logo, or must MVP stay CSS/inline-only? Impact: brand mark fidelity.
