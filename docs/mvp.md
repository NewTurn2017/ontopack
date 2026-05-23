# ontopack MVP runbook

이 문서는 새 사용자가 5분 안에 ontopack MVP를 확인하는 경로입니다. 기본 경로는 모델 다운로드 없이 동작하는 keyword/search + MCP context + localhost viewer입니다.

## 1. 빌드

필요 조건: Rust/Cargo가 설치되어 있어야 합니다. checkout 루트에서 release binary를 만듭니다.

```bash
cargo build --release
export PATH="$PWD/target/release:$PATH"
```

이 문서의 `pack`/`pack-mcp` 명령은 위 release binary가 `PATH`에 들어있다고 가정합니다. `PATH`를 바꾸지 않으려면 `target/release/pack`처럼 직접 호출하세요.


실제 BGE-M3 임베딩까지 쓰려면 별도 feature 빌드가 필요합니다.

```bash
cargo build --release --features real-embed
```

기본 MVP 검증은 네트워크/모델 다운로드 없이 진행합니다. `pack build --no-embed`는 이 오프라인 keyword/chunk-only 경로를 명시적으로 고정할 때 사용합니다.

## 2. 새 팩 만들기

```bash
pack init ~/ontopack-demo
cat > ~/ontopack-demo/_inbox/hook.md <<'NOTE'
---
type: prompt
title: 썸네일 훅
tags: [youtube, hook]
created: 2026-03-02
---
클릭을 부르는 훅 카피와 강의 오프닝 구조.
NOTE
cd ~/ontopack-demo
pack process
pack build --incremental
pack search "훅" --mode keyword
```

성공 기준:

- `pack search`가 `[keyword]` source card를 출력한다.
- 결과에 note id와 chunk id가 함께 나온다. 예: `hook / hook#0000`.

## 3. MCP로 에이전트 연결

```bash
pack-mcp --pack-root ~/ontopack-demo
```

Codex/Claude MCP 설정에는 release binary와 pack root를 등록합니다.

```toml
[mcp_servers.ontopack]
command = "/path/to/ontopack/target/release/pack-mcp"
args = ["--pack-root", "/Users/me/ontopack-demo"]
```

MCP 도구:

- `search`: citation-ready source cards
- `ask`: LLM 답변 대신 `context_blocks` 반환
- `related`: note 관계 탐색
- `add`: content 또는 파일 추가
- `timeline`: created frontmatter 기반 목록

`ask`는 deterministic core에서 답변을 생성하지 않습니다. 에이전트가 `context_blocks`를 근거로 답변과 citation을 합성합니다.

## 4. 로컬 뷰어 열기

```bash
cd ~/ontopack-demo
pack open
```

자동화 환경에서는 브라우저 실행 없이 URL만 확인합니다.

```bash
pack open --port 0 --no-browser --print-url
```

뷰어/API는 localhost(`127.0.0.1`)에만 바인딩합니다.

주요 API:

- `/api/search?q=...&type=...&tag=...&from=...&to=...`
- `/api/ask?q=...`
- `/api/facets`
- `/api/gallery`
- `/api/notes/<id>`
- `/api/related/<id>`
- `/api/timeline`
- `/api/graph`

## 5. 전체 MVP smoke

repo checkout에서 다음 스크립트를 실행하면 임시 팩을 만들고 CLI, MCP, viewer API, `pack open --no-browser`를 한 번에 검증합니다.

```bash
scripts/mvp-smoke.sh
```

성공 메시지:

```text
MVP smoke passed: CLI + MCP + viewer API + open URL
```

## MVP 범위와 다음 단계

MVP 포함:

- 파일 기반 pack 구조와 SQLite/FTS5 파생 인덱스
- inbox 처리, 증분 build, keyword source-card search
- optional real embedding provider와 fake-embed 기반 테스트
- MCP stdio tools: `search`, `ask`, `related`, `add`, `timeline`
- framework-free localhost viewer/API: ask context, filters, gallery, graph summary

Post-MVP:

- 서버 API의 vector/hybrid 검색 연결
- 더 풍부한 멀티모달 intake/preview
- 그래프 시각화 라이브러리와 UI polish
- 배포/패키징/설치 UX
