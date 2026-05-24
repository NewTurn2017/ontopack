# ontopack real test plan

이 문서는 MVP smoke 이후 “실제 사용 준비”를 확인하는 테스트 계획입니다. 목표는 단순 기능 단위가 아니라, 사용자가 강의/자료 폴더를 지식팩으로 만들고 CLI, MCP, viewer를 함께 쓰는 흐름을 검증하는 것입니다.

## 빠른 실행

repo 루트에서 실행합니다.

```bash
scripts/real-test.sh
```

성공 메시지:

```text
Ontopack real test passed: realistic pack + CLI + exports + MCP + viewer APIs + filter stress + open URL
```

기본 실행은 모델 다운로드 없이 keyword/chunk-only 경로를 검증합니다.

## 테스트 범위

`scripts/real-test.sh`는 임시 pack을 만들고 다음 데이터를 넣습니다.

- 강의 설계 노트: `lecture-outline`
- 녹취 텍스트: `transcript.txt` → `notes/transcript.md`
- 프롬프트 노트: `thumbnail-hook`, `filter-target`
- 연구 노트: `research/agent-memory`
- 이미지 자산 + sidecar: `assets/evidence.png`, `notes/evidence-image.md`
- 필터 스트레스 데이터: `type=note` distractor 125개 + `type=prompt/tag=needle` target 1개

검증 항목:

1. `pack init`, `pack add`, `pack process`, `pack build --no-embed`, `pack build --incremental --no-embed`
2. CLI keyword search source card 출력
3. Portable context export
   - `pack export --format jsonl`
   - `pack export --format markdown-bundle --output ... --copy-assets ...`
   - `pack export --format mcp-context`
   - 모든 출력은 `note:<id>` citation과 asset path를 포함해야 함
   - `--copy-assets` 출력은 참조된 `assets/...` 파일을 보존 경로 그대로 복사해야 함
   - `pack import <context.jsonl> --format jsonl --asset-root ...`로 새 팩에 복원 후 build/search가 가능해야 함
4. HTTP viewer API
   - `/api/search` with `type/tag/from/to/k`
   - `/api/ask`
   - `/api/facets`
   - `/api/gallery`
   - `/api/timeline`
   - `/api/graph`
   - `/api/notes/<id>`
   - `/api/related/<id>`
   - 400 JSON error for missing `q`
5. MCP stdio tools
   - `initialize`
   - `tools/list`
   - `search`
   - `timeline`
   - `ask`
6. `pack open --port 0 --no-browser --print-url`
7. 필터가 `LIMIT` 전에 적용되는지 확인하는 실제형 회귀 테스트


## 미디어 인텔리전스 실제 테스트

영상 keyframe/전사 provider 경로는 별도 실제 테스트로 검증합니다. 이 테스트는 실제 mp4를 생성하고 local provider로 enrichment를 수행한 뒤, derived keyframe JPEG, CLI 검색, note/gallery API 노출까지 확인합니다.

```bash
scripts/media-intelligence-test.sh
```

성공 메시지:

```text
Ontopack media intelligence test passed: real mp4 + local provider + derived keyframes + search + API
```

Whisper 실제 전사까지 검증하려면 ggml 모델 경로를 지정해서 opt-in 실행합니다.

```bash
RUN_REAL_WHISPER=1 \
WHISPER_MODEL="$HOME/.cache/ontopack/whisper/ggml-tiny.en.bin" \
scripts/media-intelligence-test.sh
```

검증된 Mac 기본 조합:

- runtime: Homebrew `whisper-cpp`의 `whisper-cli`
- model: `$HOME/.cache/ontopack/whisper/ggml-tiny.en.bin` (repo에는 커밋하지 않음)
- speech sample: `/opt/homebrew/opt/whisper-cpp/share/whisper-cpp/jfk.wav`

`WHISPER_TEST_AUDIO=/path/to/speech.wav`로 전사 테스트용 음성 샘플을 바꿀 수 있습니다. 기본 경로는 ffmpeg 기반 mp4 생성/keyframe 추출만 수행하고, Whisper 모델 다운로드/경로에는 의존하지 않습니다.

## 실제 임베딩 선택 테스트

BGE-M3/FastEmbed 실제 경로는 모델 다운로드와 네트워크/캐시 상태에 영향을 받으므로 기본 gate에서는 제외합니다. 실제 임베딩까지 확인하려면 명시적으로 실행합니다.

```bash
RUN_REAL_EMBED=1 scripts/real-test.sh
```

이 경로는 다음을 추가로 수행합니다.

- `cargo build --release -p pack-cli --features real-embed`
- `pack embed --skip-build`
- `pack search "강의 자료 연결" --mode hybrid`

주의:

- 첫 실행은 모델/런타임 다운로드 때문에 오래 걸릴 수 있습니다.
- 네트워크가 막혀 있거나 모델 캐시가 없으면 실패할 수 있습니다.
- 기본 MVP 완료 판정은 이 optional 경로에 의존하지 않습니다.

## 사람이 직접 확인할 브라우저 체크리스트

자동 스크립트는 API와 URL만 검증합니다. 실제 브라우저 UI는 다음 순서로 수동 확인합니다.

```bash
KEEP_REAL_TEST_PACK=1 scripts/real-test.sh
cd <printed real test pack path>
/path/to/ontopack/target/debug/pack open
```

브라우저에서 확인할 항목:

- 검색창에 `온톨로지` 입력 시 source card가 표시된다.
- `type=prompt`, `tag=needle`, 날짜 범위를 적용하면 `필터 대상 프롬프트`만 남는다.
- 필터를 바꿀 때 검색 결과가 새로고침된다.
- Ask 패널이 직접 답변을 꾸미지 않고 context block을 보여준다.
- Facets에 `prompt`, `lecture`, `image`, `ontology`, `needle` 등이 보인다.
- Gallery에 `보드 사진 캡션` 카드가 보인다.
- Note detail과 related/timeline/graph 패널이 빈 화면 없이 동작한다.

## 통과 기준

MVP 이후 실제 테스트 준비 완료로 보려면 다음이 모두 만족되어야 합니다.

- `scripts/real-test.sh` 기본 경로 통과
- 기존 base gate 통과: `cargo fmt --check`, `cargo test`, `cargo clippy --all-targets -- -D warnings`
- optional real embed는 실행 여부와 결과를 별도 기록
- 브라우저 수동 체크는 실행자, 날짜, pack path, 실패 스크린샷/메모를 남김
