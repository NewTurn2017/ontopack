# ontopack — 개인 멀티모달 지식 엔진 설계

> 작성일 2026-05-22 · 상태: 합의 완료(Phase 1 구현 대기)
> 코드네임: **ontopack** · CLI: `pack`

---

## 1. 한 줄 정의

내 영상·프롬프트·이미지·노트를 **로컬 평문 팩**에 모으고, **Rust 엔진 + 로컬 임베딩 + 하이브리드 검색**으로 1초에 꺼내며, **MCP를 통해 Claude Code·Codex 어느 쪽에서든** 자연어로 부려 쓰는, "카파시 위키" 느낌의 빠른 로컬 지식 도구.

## 2. 사용자와 쓰임새

- **사용자**: 비개발 직장인/창작자. 코드는 AI(Claude/Codex)가 쓰고 본인은 디자인·운영을 지시.
- **쓰임새 (전부 해당, 같은 팩 위 네 가지 뷰)**:
  1. 창작 자산 재사용 — "그 썸네일 프롬프트 어디 갔더라"
  2. 지식·레퍼런스 검색 — "이 주제로 모아둔 게 뭐 있지"
  3. AI 컨텍스트 공급 — 팩이 곧 RAG 소스
  4. 작업 기록·회고 — 타임라인·프로젝트별 되돌아보기

## 3. 핵심 설계 원칙

> **평문 파일 = 진실의 원본. SQLite + 벡터 인덱스 = 거기서 파생된, 언제든 재생성 가능한 빠른 캐시.**

- 팩을 통째로 복사·git·압축·전달 가능(소유·이식). LLM이 직접 읽을 수 있음.
- 인덱스가 깨지거나 삭제돼도 `pack build`로 100% 복원. 인덱스는 절대 진실의 출처가 아님.
- 멀티모달은 **사이드카 패턴**: 원본 미디어는 안 건드리고, 옆 노트(.md)에 캡션·태그·전사를 적어 "글자화"한다. ("파일은 검색 안 된다, 글자만 검색된다")

## 4. 확정된 기술 결정 (Decision Log)

| 결정 | 선택 | 근거 |
|---|---|---|
| 폼 팩터 | 폴더 팩 + 로컬 뷰어, 에이전트 비종속 | 소유·이식 + Claude/Codex 양쪽 |
| 코어 런타임 | **Rust** 엔진 + 로컬 서빙 SPA | 단일 네이티브 바이너리·즉시 기동·in-process 인덱스·로컬 임베딩 라이브러리 시너지(fastembed-rs/candle) |
| LLM 인터페이스 | **MCP 우선** | Claude·Codex가 도구로 직접 호출. GraphQL 서버 오버헤드 회피 |
| 저장/벡터 | **SQLite + FTS5 + sqlite-vec** (단일 파일) | 검증된 최速 로컬, 개인 규모(수천 건)에 충분. 대용량 시 LanceDB로 이전 여지 |
| 임베딩 | **로컬, BGE-M3** | 한국어 강함·100+ 언어·dense+sparse(하이브리드 궁합)·8192토큰·오프라인·무비용 |
| 검색(Phase 1) | **하이브리드(벡터+BM25/FTS5) + RRF 융합** | 의미+키워드 동시, 2026 정설, 코드 없이 빠름 |
| 캡처 | 인박스에 막 던지고 → `pack process` 일괄 정리 | 마찰 최소(5초) + 통제 가능. 강의 "1분 캡처" 패턴과 일치 |

> 비종속 원칙: 결정적 작업(파일 정리·인덱싱·검색·뷰어 빌드)은 `pack` 바이너리가, AI 판단(캡션·태깅·Q&A)은 그때 띄운 에이전트가 맡는다.

## 5. 아키텍처

### 5.1 3층 + 엔진/데이터 분리

```
① 팩 (DATA)   ~/my-pack/ · 평문 .md + 자산 + 사이드카   ← 진실의 원본, 이식 가능
② 엔진 (RUST) ontopack 바이너리 · 인덱서 · 하이브리드 검색 · 임베딩 · MCP
              두 입구 →  MCP(에이전트용)  +  로컬 HTTP(뷰어용)
③ 뷰어 (VIEW) "카파시 위키" SPA · 자연어 ask · 갤러리 · 온톨로지 그래프 · 필터
```

- **엔진(프로그램)** 과 **팩(내 데이터)** 은 분리. 엔진은 config로 팩 경로를 가리킨다. 팩 여러 개 전환 가능.

### 5.2 레포 구조 (엔진)

```
ontopack/
├─ crates/
│  ├─ pack-core/   # 라이브러리: 인덱서, 하이브리드 검색(벡터+FTS5+RRF), 임베딩(BGE-M3 via fastembed), sqlite-vec 바인딩, 팩 파서
│  ├─ pack-cli/    # 바이너리: add · process · build · search · open · serve
│  └─ pack-mcp/    # 바이너리: MCP 서버 (stdio) — ask · search · related · add · timeline
├─ viewer/         # SPA(정적): 위키 UI. pack serve가 호스팅
├─ AGENTS.md       # 에이전트 공용 규약 (↔ CLAUDE.md 동일 내용 심링크/복제)
├─ CLAUDE.md
└─ docs/superpowers/specs/2026-05-22-ontopack-design.md
```

### 5.3 팩 구조 (사용자 데이터)

```
~/my-pack/
├─ notes/                 # 노트 한 장 = 한 개체 (.md + frontmatter)
│  ├─ prompt_썸네일-훅.md
│  ├─ image_데모-화이트보드.md   # 사이드카 노트 (원본은 assets/)
│  └─ project_오로라.md
├─ assets/                # 원본 미디어 (엔진이 안 건드림)
├─ _inbox/                # 막 던지는 곳 (5초 캡처)
├─ .pack/                 # 파생물: index.db (SQLite+FTS5+vec), 임베딩 캐시
└─ pack.toml              # 팩 설정 (임베딩 모델, 타입/관계 스키마 등)
```

## 6. 데이터 모델

### 6.1 노트 = 개체 (atomic)

노트 한 장 = 한 개체. 맨 위 frontmatter + 본문(자유 텍스트/캡션/전사).

```yaml
---
type: prompt            # 개체 타입 (사용자 정의: prompt/image/video/project/person/ref ...)
title: 썸네일 훅 카피 v3
tags: [thumbnail, hook, marketing]
created: 2026-05-20
asset: assets/thumb_0521.png   # (미디어 노트면) 원본 경로 — 사이드카
related:                       # 관계 (위키링크, 따옴표 필수)
  - "[[project_오로라]]"
  - "[[prompt_썸네일-훅-v2]]"
---
본문 = 검색 대상. 이미지면 캡션, 영상이면 전사 요약이 여기에.
```

- **관계**는 frontmatter의 위키링크 필드 + 본문 `[[...]]`. 이것이 온톨로지 그래프의 엣지가 된다.
- 타입·관계 어휘는 표준이 아니라 **사용자 약속**(`pack.toml`에 선언, 작게 시작 3~4개).

### 6.2 사이드카 (멀티모달)

- 미디어 원본은 `assets/`에 그대로. 노트(.md)가 캡션·태그·(영상)전사를 보유 → 그게 검색·임베딩 대상.
- Phase 1: 캡션/태그는 에이전트가 작성(AGENTS.md 규약). Phase 2: 영상 자동 전사 + CLIP 이미지 임베딩.

## 7. 인덱스 설계 (`.pack/index.db`)

파생 캐시. `pack build`가 노트를 스캔해 채운다.

- `notes(id, path, type, title, tags_json, created, mtime, body, asset)` — 메타 + 본문
- `notes_fts` — FTS5 가상 테이블(title, body, tags) → BM25 키워드 검색
- `vec_notes(note_id, embedding)` — sqlite-vec, BGE-M3 dense 벡터(본문 청크 단위)
- `edges(src_id, dst_id, kind)` — 관계 그래프(위키링크에서 추출). Phase 2 GraphRAG 확장의 기반
- `chunks(id, note_id, ord, text)` — 긴 노트 청킹(임베딩·인용 단위)

증분 인덱싱: `mtime`/해시 비교로 바뀐 노트만 재임베딩.

## 8. 검색 파이프라인 (Phase 1)

```
질의 → ① 벡터 검색(sqlite-vec, BGE-M3 dense)  ─┐
       ② 키워드 검색(FTS5 BM25)              ─┤→ RRF 융합 → 상위 K 카드/청크 → (ask면) LLM이 인용해 답
```

- **RRF(Reciprocal Rank Fusion)**: 두 랭킹을 순위 기반으로 융합. 가중치 튜닝 부담 적고 견고.
- `ask`(MCP/뷰어): 융합 결과 상위 청크를 컨텍스트로 LLM이 답 + **출처 노트 인용**.
- 성능 목표: 수천 건 규모에서 검색 p95 < 50ms(임베딩 제외), 콜드 스타트 < 1s.

## 9. CLI 사양 (`pack`)

| 명령 | 동작 | AI 필요? |
|---|---|---|
| `pack init [경로]` | 새 팩 골격 + `pack.toml` 생성 | 아니오 |
| `pack add <파일\|URL>` | `_inbox`에 넣고 노트 스텁 생성 | 아니오 |
| `pack process` | `_inbox` 비우며 정리(타입 추정·자산 이동·스텁). 캡션/태깅이 필요한 항목 표시 | 일부(에이전트가 채움) |
| `pack build` | 변경 노트 스캔 → 임베딩 → SQLite+FTS5+vec 인덱스 갱신 | 아니오 |
| `pack search "<질의>"` | 터미널 하이브리드 검색 결과 | 아니오 |
| `pack serve` | 로컬 HTTP로 뷰어 + JSON API 호스팅 | 아니오 |
| `pack open` | `serve` 후 브라우저로 위키 열기 | 아니오 |

## 10. MCP 서버 사양 (`pack-mcp`, stdio)

Claude Code·Codex가 동일하게 붙는 도구:

- `search(query, type?, k?)` → 하이브리드 검색 결과(노트 메타 + 스니펫)
- `ask(question, k?)` → RRF 컨텍스트 + 답변 + 인용 노트 id
- `related(note_id, depth?)` → 관계/유사도 기반 관련 노트 (위키 "이걸 보면 저것도")
- `add(content, type?, tags?)` → 노트 생성(인박스 거치거나 직접) 후 인덱스 갱신
- `timeline(from?, to?, type?)` → 기간/타입별 작업 기록

## 11. 뷰어 사양 (위키 SPA)

`pack serve`가 호스팅하는 정적 SPA + JSON API:

- **상단 자연어 ask 바**: 질문 → RAG 답 + 출처 카드(클릭하면 노트로). 위키의 1순위 인터페이스.
- **카드 갤러리**: 썸네일·제목·타입·태그. 미디어는 썸네일.
- **온톨로지 그래프**: 점=개체(색=타입), 선=관계. 클릭하면 점프. 헤어볼 방지 — 타입 색상 + 필터.
- **필터**: 타입/태그/기간.
- "관련 노트 자동 추천" 패널(Phase 1 1순위 wow 기능).

## 12. 에이전트 규약 (AGENTS.md ↔ CLAUDE.md)

두 파일 동일 내용. 포함:
- 팩 폴더 규약 + frontmatter 스키마 + 따옴표 위키링크 규칙.
- 캡션/태깅 지침(이미지 보면 캡션·태그 이렇게), 영상 전사 요약 형식.
- "정리 끝나면 `pack build` 호출", "팩 근거로 답할 땐 노트만 인용".
- MCP 도구 사용 가이드.

## 13. 캡처 → 정리 → 꺼내기 흐름

```
[_inbox에 막 던짐 5초]
   → pack process (자산 이동·노트 스텁·캡션 필요 표시)
   → 에이전트가 캡션/태그/관계 채움 (AGENTS.md 규약)
   → pack build (임베딩·인덱스 갱신)
   → pack open / pack search / MCP ask 로 꺼내 씀
```

## 14. 범위

### Phase 1 (이번 스펙)
Rust 엔진(pack-core) + CLI(init/add/process/build/search/serve/open) + 인덱서(SQLite+FTS5+sqlite-vec) + BGE-M3 로컬 임베딩 + 하이브리드+RRF 검색 + MCP(search/ask/related/add/timeline) + 위키 뷰어(ask/갤러리/그래프/필터) + AGENTS.md.

### Phase 2 (별도 스펙, 백로그)
- 로컬 cross-encoder 리랭커(fastembed-rs)
- GraphRAG: 온톨로지 관계 따라 컨텍스트 확장
- CLIP 이미지 의미검색
- 영상 자동 전사(기존 Scribe/whisper 재사용) + 타임스탬프
- 드롭폴더 watcher(완전 자동 ingest)
- 자동 클러스터·토픽맵, 중복·고립 노트 탐지
- (대용량 시) LanceDB 이전, Tauri 네이티브 앱 셸

## 15. 컴포넌트 경계 (격리·테스트)

| 컴포넌트 | 책임 | 의존 | 인터페이스 |
|---|---|---|---|
| pack-core | 팩 파싱·인덱싱·검색·임베딩 | sqlite-vec, fastembed | Rust API(라이브러리) |
| pack-cli | 사용자 명령 → core 호출 | pack-core | argv/stdout |
| pack-mcp | MCP 도구 → core 호출 | pack-core | MCP(stdio) |
| viewer | 표시 | pack serve JSON API | HTTP/JSON |

각 크레이트는 독립 테스트 가능. core가 진짜 로직, cli/mcp/viewer는 얇은 어댑터.

## 16. 비기능 요구

- **오프라인 우선**: 로컬 임베딩(BGE-M3)·로컬 DB. 네트워크 없이 동작.
- **성능**: 콜드 스타트 < 1s, 검색 p95 < 50ms(수천 건). 증분 인덱싱.
- **이식성**: 팩은 평문 폴더. 인덱스는 재생성 가능.
- **에이전트 비종속**: Claude Code·Codex·사람 직접 호출 모두 동일 결과.

## 17. 미해결 / 추후 결정

- 임베딩 청크 크기·오버랩 기본값(구현 시 실측 튜닝).
- 그래프 시각화 라이브러리(뷰어): 경량 선택지 비교는 구현 계획에서.
- `pack.toml` 기본 타입/관계 스키마 시드(작게 시작 3~4개로).
- "wow 분석 기능" 우선순위 중 클러스터/중복탐지의 Phase 1.5 편입 여부.

## 18. 성공 기준

- 자료를 인박스에 던지고 한 번의 정리·빌드로 검색에 잡힌다.
- 자연어로 물으면 출처 인용과 함께 답이 온다(MCP·뷰어 양쪽).
- Claude Code와 Codex 둘 다에서 동일하게 부려 쓸 수 있다.
- "와, 진짜 빠르다" — 검색이 체감 즉각적이고, 위키가 네이티브 도구처럼 느껴진다.
