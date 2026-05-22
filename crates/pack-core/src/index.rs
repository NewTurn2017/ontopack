use crate::note::Note;
use anyhow::{anyhow, Result};
use rusqlite::Connection;
use std::path::Path;

pub struct Index {
    conn: Connection,
}

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS notes (
  id      TEXT PRIMARY KEY,
  path    TEXT NOT NULL,
  type    TEXT NOT NULL,
  title   TEXT NOT NULL,
  tags    TEXT NOT NULL,
  created TEXT,
  asset   TEXT,
  body    TEXT NOT NULL,
  mtime   INTEGER NOT NULL
);
CREATE VIRTUAL TABLE IF NOT EXISTS notes_fts USING fts5(
  id UNINDEXED, title, body, tags
);
CREATE TABLE IF NOT EXISTS edges (
  src  TEXT NOT NULL,
  dst  TEXT NOT NULL,
  kind TEXT NOT NULL,
  PRIMARY KEY (src, dst, kind)
);
";

impl Index {
    fn init(conn: Connection) -> Result<Index> {
        conn.execute_batch(SCHEMA)?;
        Ok(Index { conn })
    }

    pub fn open(db_path: &Path) -> Result<Index> {
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        Index::init(Connection::open(db_path)?)
    }

    pub fn open_in_memory() -> Result<Index> {
        Index::init(Connection::open_in_memory()?)
    }

    /// 모든 테이블을 비우고 notes를 다시 채운다(전체 재빌드).
    /// 삭제+삽입은 한 트랜잭션으로 묶어 실패 시 기존 인덱스를 보존한다.
    pub fn rebuild(&mut self, notes: &[Note]) -> Result<()> {
        reject_duplicate_note_ids(notes)?;
        let tx = self.conn.transaction()?;
        tx.execute_batch("DELETE FROM notes; DELETE FROM notes_fts; DELETE FROM edges;")?;
        {
            let mut ins = tx.prepare(
                "INSERT INTO notes (id, path, type, title, tags, created, asset, body, mtime)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            )?;
            let mut ins_fts = tx
                .prepare("INSERT INTO notes_fts (id, title, body, tags) VALUES (?1, ?2, ?3, ?4)")?;
            let mut ins_edge =
                tx.prepare("INSERT OR IGNORE INTO edges (src, dst, kind) VALUES (?1, ?2, ?3)")?;
            for n in notes {
                let tags_json = serde_json::to_string(&n.tags)?;
                let tags_text = n.tags.join(" ");
                ins.execute(rusqlite::params![
                    n.id,
                    n.path.to_string_lossy(),
                    n.note_type,
                    n.title,
                    tags_json,
                    n.created,
                    n.asset,
                    n.body,
                    n.mtime,
                ])?;
                ins_fts.execute(rusqlite::params![n.id, n.title, n.body, tags_text])?;
                for dst in &n.related {
                    ins_edge.execute(rusqlite::params![n.id, dst, "related"])?;
                }
            }
        }
        tx.commit()?;
        Ok(())
    }

    /// FTS5 키워드 검색. 결과는 bm25 점수 오름차순(관련도 높은 순).
    pub fn search_keyword(&self, query: &str, k: usize) -> Result<Vec<crate::search::NoteHit>> {
        let safe = sanitize_fts_query(query);
        if safe.is_empty() {
            return Ok(Vec::new());
        }
        let mut stmt = self.conn.prepare(
            "SELECT n.id, n.title, n.type, bm25(notes_fts) AS score
             FROM notes_fts JOIN notes n ON n.id = notes_fts.id
             WHERE notes_fts MATCH ?1
             ORDER BY score
             LIMIT ?2",
        )?;
        let rows = stmt.query_map(rusqlite::params![safe, k as i64], |r| {
            Ok(crate::search::NoteHit {
                id: r.get(0)?,
                title: r.get(1)?,
                note_type: r.get(2)?,
                score: r.get(3)?,
            })
        })?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }
}

/// 사용자 질의를 FTS5 MATCH에 넣기 안전하게 변환: 토큰을 "..."로 감싸 OR.
fn reject_duplicate_note_ids(notes: &[Note]) -> Result<()> {
    let mut seen = std::collections::HashSet::new();
    for note in notes {
        if !seen.insert(note.id.as_str()) {
            return Err(anyhow!("duplicate note id: {}", note.id));
        }
    }
    Ok(())
}

fn sanitize_fts_query(query: &str) -> String {
    query
        .split_whitespace()
        .map(|t| format!("\"{}\"", t.replace('"', "")))
        .filter(|t| t != "\"\"")
        .collect::<Vec<_>>()
        .join(" OR ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::note::parse_str;

    #[test]
    fn creates_schema_in_memory() {
        let idx = Index::open_in_memory().unwrap();
        let count: i64 = idx
            .conn
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE name IN ('notes','notes_fts','edges')",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 3);
    }

    #[test]
    fn rebuild_inserts_notes_and_edges() {
        let mut idx = Index::open_in_memory().unwrap();
        let mut n1 = parse_str(
            "a",
            "---\ntype: prompt\ntitle: 알파\ntags: [x]\nrelated:\n  - \"[[b]]\"\n---\n알파 본문",
        )
        .unwrap();
        n1.path = "notes/a.md".into();
        let mut n2 = parse_str("b", "베타 본문").unwrap();
        n2.path = "notes/b.md".into();
        idx.rebuild(&[n1, n2]).unwrap();

        let notes: i64 = idx
            .conn
            .query_row("SELECT count(*) FROM notes", [], |r| r.get(0))
            .unwrap();
        assert_eq!(notes, 2);
        let fts: i64 = idx
            .conn
            .query_row("SELECT count(*) FROM notes_fts", [], |r| r.get(0))
            .unwrap();
        assert_eq!(fts, 2);
        let edges: i64 = idx
            .conn
            .query_row(
                "SELECT count(*) FROM edges WHERE src='a' AND dst='b'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(edges, 1);
    }

    #[test]
    fn rebuild_is_idempotent() {
        let mut idx = Index::open_in_memory().unwrap();
        let n = parse_str("a", "본문").unwrap();
        idx.rebuild(std::slice::from_ref(&n)).unwrap();
        idx.rebuild(&[n]).unwrap();
        let notes: i64 = idx
            .conn
            .query_row("SELECT count(*) FROM notes", [], |r| r.get(0))
            .unwrap();
        assert_eq!(notes, 1);
    }

    #[test]
    fn rebuild_rejects_duplicate_ids_before_mutating() {
        let mut idx = Index::open_in_memory().unwrap();
        let existing = parse_str("a", "기존 본문").unwrap();
        idx.rebuild(&[existing]).unwrap();

        let first = parse_str("dup", "첫 번째").unwrap();
        let second = parse_str("dup", "두 번째").unwrap();
        let err = idx.rebuild(&[first, second]).unwrap_err();
        assert!(err.to_string().contains("duplicate note id"));

        let notes: i64 = idx
            .conn
            .query_row("SELECT count(*) FROM notes", [], |r| r.get(0))
            .unwrap();
        assert_eq!(notes, 1);
        let id: String = idx
            .conn
            .query_row("SELECT id FROM notes", [], |r| r.get(0))
            .unwrap();
        assert_eq!(id, "a");
    }
}
