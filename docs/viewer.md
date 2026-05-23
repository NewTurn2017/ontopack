# ontopack 로컬 뷰어

OntoPack 뷰어는 `pack-core`가 만든 인덱스와 노트 파서를 그대로 사용하는 localhost HTTP UI입니다. 별도 프론트엔드 빌드 없이 `pack-server`에 HTML/CSS/JS가 내장되어 있습니다.

## 실행

```bash
pack serve --port 8787
```

출력된 `http://127.0.0.1:8787` 주소를 브라우저에서 열면 검색, Ask 컨텍스트, 검색 모드 capability, type/tag/date 필터, 노트 상세, 관련 노트, 타임라인, 갤러리, 그래프를 볼 수 있습니다.

브라우저까지 바로 열려면:

```bash
pack open
```

임의 포트를 쓰고 URL만 출력하려면:

```bash
pack open --port 0 --no-browser --print-url
```

자동화/스모크 테스트용으로 요청 하나만 처리할 수 있습니다.

```bash
pack serve --port 0 --once --request $'GET /api/search?q=hello HTTP/1.1\r\nHost: localhost\r\n\r\n'
```

## API

### `GET /api/search?q=<query>&k=<n>&type=<type>&tag=<tag>&from=<date>&to=<date>`

키워드 chunk source card를 반환합니다. type/tag/date 필터를 적용할 수 있습니다. 기본 서버는 모델 다운로드 없는 keyword 검색을 사용하며, 응답에는 개발/튜닝용 `elapsed_ms`, 실제 실행 `mode`, `source`가 포함됩니다. `mode=vector|hybrid`는 capability가 열리기 전까지 400 JSON 오류로 거절합니다.

```json
{
  "query": "훅",
  "mode": "keyword",
  "source": "sqlite_fts",
  "hits": [
    {
      "note_id": "hook",
      "chunk_id": "hook#0000",
      "title": "썸네일 훅",
      "note_type": "prompt",
      "snippet": "클릭을 부르는 훅 카피.",
      "path": "/path/to/notes/hook.md",
      "score": -1.23,
      "rank_source": "keyword"
    }
  ],
  "elapsed_ms": 2
}
```


### `GET /api/ask?q=<question>&k=<n>`

LLM 답변을 서버에서 직접 생성하지 않고 citation-ready context blocks를 반환합니다. 브라우저나 호출 에이전트가 이 컨텍스트를 바탕으로 답을 합성합니다.

```json
{
  "question": "훅 자료?",
  "answer_mode": "external_llm_required",
  "instruction": "Use context_blocks to synthesize an answer with citations outside deterministic pack-core.",
  "context_blocks": [],
  "elapsed_ms": 2
}
```

### `GET /api/capabilities`

서버가 실제로 지원하는 검색 모드를 반환합니다. 현재 내장 서버는 `keyword`만 활성화하고 `vector`/`hybrid`는 잠금 상태로 노출합니다. UI는 이 응답을 기준으로 semantic 모드를 비활성화합니다.

### `GET /api/facets`

필터 UI에 필요한 type/tag/date 범위를 반환합니다.

### `GET /api/dashboard?type=<type>&from=<date>&to=<date>&gallery_k=<n>&timeline_k=<n>&graph_limit=<n>`

뷰어 시작/필터 변경 시 필요한 overview 데이터를 한 번에 반환합니다. 응답은 `facets`, `gallery`, `timeline`, `graph`, `elapsed_ms`를 포함하며, viewer는 이 API로 패널 초기 로딩 fan-out을 줄입니다.

### `GET /api/gallery?type=<type>&k=<n>`

`asset` frontmatter가 있는 사이드카 노트를 갤러리 카드로 반환합니다. 각 항목은 실제 로컬 미디어를 표시할 수 있도록 `asset_url`, `media_kind`, `mime`을 포함합니다.

```json
{
  "items": [
    {
      "id": "pic",
      "title": "보드 사진",
      "note_type": "image",
      "tags": ["gallery"],
      "asset": "assets/pic.png",
      "asset_url": "/assets/pic.png",
      "media_kind": "image",
      "mime": "image/png",
      "path": "/path/to/notes/pic.md",
      "caption": "캡션"
    }
  ]
}
```

### `GET /assets/<asset-path>`

팩의 `assets/` 안에 있는 파일만 localhost로 제공합니다. `../` traversal은 거부됩니다. 이미지와 비디오는 viewer에서 lazy thumbnail 또는 metadata preload preview로 표시됩니다.

예:

```text
/assets/pic.png
/assets/demo%20clip.mp4
```

### `GET /api/notes/<id>`

노트 상세를 반환합니다.

```json
{
  "id": "hook",
  "title": "썸네일 훅",
  "note_type": "prompt",
  "tags": ["youtube"],
  "created": "2026-02-01",
  "asset": null,
  "asset_url": null,
  "media_kind": null,
  "mime": null,
  "related": ["project-a"],
  "body": "본문",
  "path": "/path/to/notes/hook.md"
}
```

### `GET /api/related/<id>?depth=<n>`

위키링크/frontmatter related 관계를 따라간 관련 노트를 반환합니다.

### `GET /api/timeline?from=<date>&to=<date>&type=<type>&k=<n>`

`created` frontmatter가 있는 노트를 최신순으로 반환합니다.

### `GET /api/graph?type=<type>&limit=<n>`

그래프용 노드/엣지 목록을 반환합니다. `limit` 기본값은 100이라 큰 팩에서 hairball을 피합니다.

## 현재 한계

- 서버 검색은 capability 기반 keyword mode입니다. vector/hybrid는 `/api/capabilities`에서 locked로 표시되며, `pack search --mode vector|hybrid`와 `real-embed` 경로에서 먼저 검증된 뒤 서버 API에 연결할 예정입니다.
- 뷰어는 framework-free MVP입니다. 갤러리와 선택 노트는 asset sidecar의 이미지/비디오를 실제로 표시하지만, 아직 썸네일 생성/트랜스코딩/비디오 타임라인 인덱싱은 하지 않습니다.
- 그래프는 아직 lightweight 링크 요약입니다. 시각화 라이브러리는 API와 사용 패턴이 안정된 뒤 추가합니다.
- `pack open`의 실제 브라우저 실행은 OS 명령(`open`, `xdg-open`, `cmd /C start`)에 의존합니다. 자동화에서는 `--no-browser --print-url`을 사용하세요.
