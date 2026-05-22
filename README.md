# ontopack

로컬 멀티모달 지식 팩 엔진. 평문 파일이 진실, SQLite+FTS5가 빠른 인덱스.

## M1 (현재)
- `pack init [경로]` — 새 팩
- `pack add <파일> [--type T]` — md→notes/, 그 외→assets/+사이드카
- `pack build` — 인덱스 (재)빌드
- `pack search "<질의>"` — 키워드(BM25) 검색

## M2A
- `pack process` — `_inbox/` 파일을 `notes/` 또는 `assets/` + 사이드카로 정리
- `pack build --incremental` — 변경된 노트만 파생 인덱스 갱신
- 인덱스는 `notes`, `notes_fts`, `edges`, `chunks`를 재생성/갱신

## M2B
- `pack build --no-embed` — 임베딩 없이 기존 키워드/청크 인덱스만 빌드
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
- 검색 결과는 `[keyword|vector|hybrid] 제목  (note_id / chunk_id) snippet` 형태라 MCP/뷰어 citation에 재사용 가능
- core에서는 `SearchHit`, `RankSource`, RRF fusion, `Pack::search_hybrid_with`를 제공하며 테스트는 fake embedder로 모델 다운로드 없이 검증

## 다음 (M2~)
MCP 서버, 위키 뷰어.
