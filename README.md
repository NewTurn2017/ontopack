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
- 실제 BGE-M3 provider/model download는 `Embedder` 뒤에 붙일 다음 slice로 남겨두며, 현재 테스트/기본 CLI 경로는 모델 다운로드를 요구하지 않음

## 다음 (M2~)
실제 BGE-M3 provider, 하이브리드/RRF, MCP 서버, 위키 뷰어.
