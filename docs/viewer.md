# ontopack 로컬 뷰어

M4 뷰어는 `pack-core`가 만든 인덱스와 노트 파서를 그대로 사용하는 localhost HTTP UI입니다. 별도 프론트엔드 빌드 없이 `pack-server`에 HTML/CSS/JS가 내장되어 있습니다.

## 실행

```bash
pack serve --port 8787
```

출력된 `http://127.0.0.1:8787` 주소를 브라우저에서 열면 검색, 노트 상세, 관련 노트, 타임라인, 그래프를 볼 수 있습니다.

브라우저까지 바로 열려면:

```bash
pack open
```

임의 포트를 쓰려면:

```bash
pack open --port 0 --print-url
```

자동화/스모크 테스트용으로 요청 하나만 처리할 수 있습니다.

```bash
pack serve --port 0 --once --request $'GET /api/search?q=hello HTTP/1.1\r\nHost: localhost\r\n\r\n'
```

## API

### `GET /api/search?q=<query>&k=<n>&type=<type>`

키워드 chunk source card를 반환합니다. 기본 M4 서버는 모델 다운로드 없는 keyword 검색을 사용합니다.

```json
{
  "query": "훅",
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
  ]
}
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

- 서버 검색은 기본 keyword mode입니다. vector/hybrid는 `pack search --mode vector|hybrid`와 `real-embed` 경로에서 먼저 검증된 뒤 서버 API에 연결할 예정입니다.
- 뷰어는 framework-free MVP입니다. 갤러리/그래프 시각화 라이브러리는 API와 사용 패턴이 안정된 뒤 추가합니다.
- `pack open`의 실제 브라우저 실행은 OS 명령(`open`, `xdg-open`, `cmd /C start`)에 의존합니다. 자동화에서는 `--no-browser --print-url`을 사용하세요.
