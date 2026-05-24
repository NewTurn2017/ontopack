# ontopack

로컬 멀티모달 지식 팩 엔진. 평문 파일이 진실, SQLite+FTS5가 빠른 인덱스.

## 빠른 MVP 검증

- 시스템 동작 원리, 상세 실행 방법, 미디어/성능 다음 계획은 `docs/system-deep-dive.md`를 참고하세요.
- 새 사용자는 `docs/mvp.md`의 5분 runbook으로 init → process/build → search → MCP → viewer를 확인할 수 있습니다.
- 전체 자동 smoke: `scripts/mvp-smoke.sh` (임시 팩으로 CLI + MCP + viewer API + `pack open --no-browser` 검증)
- 실제형 테스트: `scripts/real-test.sh` (강의/자료 pack + 필터 스트레스 + MCP/viewer API + optional real-embed 준비)

## M1 (현재)
- `pack init [경로]` — 새 팩
- `pack add <파일> [--type T]` — md/markdown/txt→notes/, 그 외→assets/+사이드카
- `pack build` — 인덱스 (재)빌드
- `pack search "<질의>"` — 키워드(BM25) 검색

## M2A
- `pack process` — `_inbox/` 파일을 `notes/` 또는 `assets/` + 사이드카로 정리
- `pack build --incremental` — 변경된 노트만 파생 인덱스 갱신
- 인덱스는 `notes`, `notes_fts`, `edges`, `chunks`를 재생성/갱신

## M2B
- `pack build --no-embed` — 기본 MVP와 같은 오프라인 키워드/청크 인덱스만 빌드(모델 다운로드 없음)
- `pack-core::embed::Embedder` — 실제 모델과 테스트용 fake embedder를 분리하는 임베딩 인터페이스
- `pack-core`는 `sqlite-vec` 기반 `vec_chunks` 파생 테이블에 청크 벡터를 저장하고, fake embedder fixture로 벡터 검색을 검증
- 실제 BGE-M3 provider는 optional feature로 분리:
  - 기본 빌드: 모델 다운로드 없음, `pack embed`는 feature 안내 오류를 출력
  - 실제 임베딩 빌드: `cargo build --release --features real-embed`
  - 사용: `pack embed` 또는 기존 인덱스를 유지하려면 `pack embed --skip-build`
  - 첫 실행은 fastembed/Hugging Face 캐시에 `BAAI/bge-m3` 모델을 내려받을 수 있음

## M2C
- `pack search "<질의>" --mode keyword` — 기본 키워드 검색을 citation-ready source card로 출력
- `pack search "<질의>" --mode vector|hybrid` — `real-embed` 빌드에서 BGE-M3 임베더로 vector/hybrid 검색
- `pack export --format markdown-bundle|jsonl|mcp-context [--output 파일] [--copy-assets 디렉터리]` — UI 없이 Claude/Codex/강의 번들/다른 앱으로 넘길 수 있는 citation-ready portable context 출력. `--copy-assets`는 참조된 원본/derived media를 경로 보존 방식으로 복사
- `pack import context.jsonl --format jsonl [--asset-root 디렉터리]` — export JSONL과 복사된 asset tree를 새 팩으로 복원
- `pack bundle <디렉터리> [--archive bundle.tar.gz]` / `pack import <bundle-디렉터리|bundle.tar.gz>` — context JSONL, Markdown bundle, MCP context, assets, manifest를 한 portable 디렉터리 artifact로 묶고, 필요하면 같은 레이아웃을 `.tar.gz`로 포장해 복원
- 검색 결과는 `[keyword|vector|hybrid] 제목  (note_id / chunk_id) snippet` 형태라 MCP/뷰어 citation에 재사용 가능
- core에서는 `SearchHit`, `RankSource`, RRF fusion, `Pack::search_hybrid_with`를 제공하며 테스트는 fake embedder로 모델 다운로드 없이 검증

## M3
- `pack-mcp --pack-root <팩>` — Claude/Codex용 stdio MCP 서버
- MCP 도구: `search`, `ask`, `related`, `add`, `timeline`, `media/list_pending`, `media/read_note`, `media/write_enrichment`, `index/rebuild`
- `ask`는 LLM 답변을 코어에서 생성하지 않고 citation-ready `context_blocks`를 반환
- `pack enrich-pending --provider-command scripts/providers/auto_media_worker.py` — API 키가 있으면 API provider를 우선 사용하고, 없으면 macOS 로컬 Ollama/Tesseract/FFmpeg worker로 pending media enrichment 후 검색 인덱스 재빌드
- 포터블 bundle/import 저장 형식은 OS-neutral path 계약(`assets/...`)을 사용하지만, 현재 real smoke와 provider toolchain 검증은 macOS 기준입니다. Windows는 `docs/providers.md`, `docs/real-test.md`의 미검증 경로를 참고하세요.
- media 도구는 Claude/Codex 같은 외부 AI worker가 로컬 asset sidecar를 읽고 caption/OCR/transcript/summary를 안전한 managed block에 쓰도록 연결
- `related`/`timeline`/`add`/media 동작은 `pack-core`가 소유하고 MCP는 얇은 JSON-RPC 어댑터로 유지
- 설정 예시는 `docs/mcp.md`, provider worker 예시는 `docs/providers.md` 참고

## M4
- `pack serve --port 8787` — 현재 팩을 localhost JSON API + 정적 위키 뷰어로 제공
- `pack serve --semantic` / `pack open --semantic` — `real-embed` 빌드에서 서버 프로세스에 BGE-M3 임베더를 한 번 로드해 `/api/search?mode=vector|hybrid`와 뷰어 semantic mode를 활성화
- `pack open` — 로컬 뷰어 URL을 브라우저로 열고 서버를 유지
- API: `/api/search`, `/api/ask`, `/api/facets`, `/api/gallery`, `/api/notes/:id`, `/api/related/:id`, `/api/timeline`, `/api/graph`
- 뷰어: 검색 카드, Ask 컨텍스트, type/tag/date 필터, 노트 상세, 관련 노트, 타임라인, 갤러리, lightweight graph 요약
- 자동화 smoke: `pack serve --port 0 --once --request $'GET /api/search?q=hello HTTP/1.1\r\nHost: localhost\r\n\r\n'`
- 자세한 사용법은 `docs/viewer.md` 참고

## M6
- `pack duplicates [--json]` — source-of-truth note body를 정규화해 같은 본문을 가진 중복 후보 그룹을 리포트
- `pack orphans [--json]` — incoming/outgoing wiki link가 모두 없는 외톨이 노트를 read-only로 리포트
- `pack gaps [--json]` — 존재하지 않는 노트 id로 향하는 깨진 wiki link를 read-only로 리포트
- `pack topics [--min-count N] [--json]` — 태그 기반 토픽 노드와 co-occurrence edge를 결정적으로 리포트
- `pack recommend [note-id] [-k N] [--json]` — 명시 태그가 겹치지만 아직 연결되지 않은 관련 노트 후보를 추천

## 다음
CLIP/실제 STT 같은 provider-heavy 멀티모달 확장, orphan/gap/topic-map 지식 유지보수, watcher/installer/Windows 검증.
