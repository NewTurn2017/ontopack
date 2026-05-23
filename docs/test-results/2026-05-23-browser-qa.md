# Browser QA result — 2026-05-23

Repository: `ontopack`
Branch: `main`
Pack under test: `/var/folders/6w/ryvjgm214g361w38k2x2dcch0000gn/T//ontopack-real-test.efszZU`
Browser automation: Playwright CLI, headed Chromium session
Screenshot: `output/playwright/ontopack-browser-qa-20260523.png`

## Summary

Browser QA passed after fixing two browser-only server issues found during the run:

1. Chrome idle/preconnect connections could terminate `pack serve` with `Resource temporarily unavailable (os error 35)`.
2. The viewer server handled browser requests serially, so one idle connection could block concurrent static/API requests.

The server now keeps running after per-connection errors and handles each accepted connection in a lightweight thread. A favicon route was also added to remove browser console 404 noise.

## Evidence

Observed page:

- URL: `http://127.0.0.1:53696/`
- Title: `ontopack viewer`
- Console after final QA: `0 errors, 0 warnings`

Verified interactions:

- Initial viewer shell loads with search, filters, ask, note detail, related, gallery, timeline, and graph panels.
- Search `온톨로지` returns 4 source cards including:
  - `보드 사진 캡션`
  - `썸네일 훅 프롬프트`
  - `transcript`
  - `로컬 온톨로지 강의 설계`
- Filter stress path with `q=공통질문`, `type=prompt`, `tag=needle`, `from=2026-05-01`, `to=2026-05-31` returns exactly 1 search card:
  - `필터 대상 프롬프트`, `prompt · filter-target#0000`
- Clicking the filtered result opens note detail:
  - `prompt · 2026-05-22 · needle, ontology`
- Ask context with query `로컬` renders deterministic context blocks instead of generating an answer:
  - `external_llm_required`
  - 4 context cards returned
- Gallery/timeline/graph panels render without blank-page failure.
- Final browser console: no errors/warnings.

## Notes

- Query `화이트보드` returns no context because the indexed body contains `화이트보드에`; current FTS tokenization does not do Korean morphological stemming. Querying a separate indexed token such as `로컬` returns the expected context cards. This is not treated as an MVP blocker, but Korean tokenizer/stemming is a post-MVP search-quality candidate.
- Timeline currently honors `type/from/to`, but not `tag`. Search itself honors `type/tag/from/to`; full panel-filter parity can be considered post-MVP if desired.
