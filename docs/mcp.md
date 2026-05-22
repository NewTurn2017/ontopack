# ontopack MCP 서버

`pack-mcp`는 현재 팩을 Claude/Codex 같은 에이전트가 도구로 호출할 수 있게 하는 stdio MCP 어댑터입니다. `pack-core`가 파일/검색 동작을 소유하고, MCP 서버는 JSON-RPC 요청을 얇게 변환합니다.

## 실행

```bash
cargo run -p pack-mcp -- --pack-root /path/to/my-pack
```

`--pack-root`를 생략하면 현재 디렉터리에서 위로 올라가며 `pack.toml`을 찾습니다.

## 제공 도구

- `search` — keyword source card 검색. `mode=keyword`만 기본 지원하며 vector/hybrid는 real embedding 경로에서 먼저 인덱스를 준비해야 합니다.
- `ask` — LLM 답변을 직접 생성하지 않고 citation-ready `context_blocks`를 반환합니다.
- `related` — `note_id`에서 위키링크/related 관계를 `depth` 단계까지 탐색합니다.
- `add` — 문자열 `content`를 새 노트로 쓰거나 로컬 `path` 파일을 팩에 추가합니다.
- `timeline` — `created` frontmatter 기준으로 최신순 노트를 반환하고 `from`/`to`/`type`/`k`로 필터링합니다.

모든 도구 응답은 MCP text content 안에 JSON 문자열로 들어갑니다. 인덱스 DB는 파생 캐시이므로 `search`/`ask` 전에는 `pack build` 또는 필요한 경우 `pack embed`를 먼저 실행하세요.

## Claude Desktop 설정 예시

```json
{
  "mcpServers": {
    "ontopack": {
      "command": "/Users/genie/dev/ontopack/target/release/pack-mcp",
      "args": ["--pack-root", "/Users/genie/my-pack"]
    }
  }
}
```

개발 중에는 `command`를 `cargo`로 두고 다음처럼 실행할 수도 있습니다.

```json
{
  "mcpServers": {
    "ontopack-dev": {
      "command": "cargo",
      "args": ["run", "-p", "pack-mcp", "--", "--pack-root", "/Users/genie/my-pack"],
      "cwd": "/Users/genie/dev/ontopack"
    }
  }
}
```

## Codex 설정 예시

Codex MCP 설정에도 같은 stdio 명령을 등록합니다.

```toml
[mcp_servers.ontopack]
command = "/Users/genie/dev/ontopack/target/release/pack-mcp"
args = ["--pack-root", "/Users/genie/my-pack"]
```

## 수동 smoke

```bash
pack init /tmp/ontopack-demo
printf '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}\n' \
  | cargo run -p pack-mcp -- --pack-root /tmp/ontopack-demo
```

응답의 `serverInfo.name`이 `ontopack`이면 stdio 초기화 경로가 정상입니다. 서버는 MCP `2025-11-25`, `2025-06-18`, `2025-03-26`, `2024-11-05` 초기화 버전을 지원하며, 클라이언트가 지원 버전을 요청하면 같은 버전으로 응답하고 알 수 없는 버전은 최신 지원 버전으로 협상합니다.
