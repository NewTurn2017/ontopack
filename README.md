# ontopack

로컬 멀티모달 지식 팩 엔진. 평문 파일이 진실, SQLite+FTS5가 빠른 인덱스.

## M1 (현재)
- `pack init [경로]` — 새 팩
- `pack add <파일> [--type T]` — md→notes/, 그 외→assets/+사이드카
- `pack build` — 인덱스 (재)빌드
- `pack search "<질의>"` — 키워드(BM25) 검색

## 다음 (M2~)
임베딩(BGE-M3)+sqlite-vec+하이브리드/RRF, MCP 서버, 위키 뷰어.
