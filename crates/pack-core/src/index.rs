use crate::note::Note;
use anyhow::{anyhow, Result};
use rusqlite::{
    params, params_from_iter, types::Value, Connection, OptionalExtension, Row, Transaction,
};
use std::path::Path;
use std::sync::Once;

pub struct Index {
    conn: Connection,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VectorChunkHit {
    pub chunk_id: String,
    pub note_id: String,
    pub title: String,
    pub note_type: String,
    pub text: String,
    pub path: String,
    pub asset: Option<String>,
    pub remote_url: Option<String>,
    pub thumbnail_url: Option<String>,
    pub media_kind: Option<String>,
    pub mime: Option<String>,
    pub distance: f32,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct BuildReport {
    pub indexed: usize,
    pub skipped: usize,
    pub removed: usize,
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
  remote_url    TEXT,
  thumbnail_url TEXT,
  media_kind    TEXT,
  mime          TEXT,
  body    TEXT NOT NULL,
  mtime   INTEGER NOT NULL,
  hash    TEXT NOT NULL
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
CREATE TABLE IF NOT EXISTS chunks (
  id      TEXT PRIMARY KEY,
  note_id TEXT NOT NULL,
  ord     INTEGER NOT NULL,
  text    TEXT NOT NULL,
  FOREIGN KEY(note_id) REFERENCES notes(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_chunks_note_ord ON chunks(note_id, ord);
CREATE INDEX IF NOT EXISTS idx_notes_type ON notes(type);
CREATE INDEX IF NOT EXISTS idx_notes_created ON notes(created);
CREATE INDEX IF NOT EXISTS idx_notes_asset ON notes(asset);
CREATE INDEX IF NOT EXISTS idx_edges_src ON edges(src);
CREATE INDEX IF NOT EXISTS idx_edges_dst ON edges(dst);
";

impl Index {
    fn init(conn: Connection) -> Result<Index> {
        reset_legacy_derived_schema_if_needed(&conn)?;
        conn.execute_batch(SCHEMA)?;
        Ok(Index { conn })
    }

    pub fn open(db_path: &Path) -> Result<Index> {
        register_sqlite_vec_extension();
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        Index::init(Connection::open(db_path)?)
    }

    pub fn open_in_memory() -> Result<Index> {
        register_sqlite_vec_extension();
        Index::init(Connection::open_in_memory()?)
    }

    /// 모든 테이블을 비우고 notes를 다시 채운다(전체 재빌드).
    /// 삭제+삽입은 한 트랜잭션으로 묶어 실패 시 기존 인덱스를 보존한다.
    pub fn rebuild(&mut self, notes: &[Note]) -> Result<()> {
        reject_duplicate_note_ids(notes)?;
        let tx = self.conn.transaction()?;
        tx.execute_batch(
            "DELETE FROM chunks; DELETE FROM notes; DELETE FROM notes_fts; DELETE FROM edges;",
        )?;
        for note in notes {
            insert_note_rows(&tx, note, &note.content_hash())?;
        }
        tx.commit()?;
        Ok(())
    }

    /// 변경된 노트만 파생 인덱스 행을 갱신하고, 사라진 노트 행은 제거한다.
    pub fn rebuild_incremental(&mut self, notes: &[Note]) -> Result<BuildReport> {
        reject_duplicate_note_ids(notes)?;
        let tx = self.conn.transaction()?;
        let mut report = BuildReport::default();

        let incoming: std::collections::HashSet<&str> =
            notes.iter().map(|n| n.id.as_str()).collect();
        for id in collect_existing_note_ids(&tx)? {
            if !incoming.contains(id.as_str()) {
                delete_note_rows(&tx, &id)?;
                report.removed += 1;
            }
        }

        for note in notes {
            let hash = note.content_hash();
            let existing: Option<(i64, String)> = tx
                .query_row(
                    "SELECT mtime, hash FROM notes WHERE id = ?1",
                    [note.id.as_str()],
                    |r| Ok((r.get(0)?, r.get(1)?)),
                )
                .optional()?;
            if existing
                .as_ref()
                .is_some_and(|(mtime, old_hash)| *mtime == note.mtime && old_hash == &hash)
            {
                report.skipped += 1;
                continue;
            }
            delete_note_rows(&tx, &note.id)?;
            insert_note_rows(&tx, note, &hash)?;
            report.indexed += 1;
        }

        tx.commit()?;
        Ok(report)
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

    pub fn search_keyword_chunks(
        &self,
        query: &str,
        k: usize,
    ) -> Result<Vec<crate::search::SearchHit>> {
        self.search_keyword_chunks_filtered(query, k, crate::search::SearchFilters::default())
    }

    pub fn search_keyword_chunks_filtered(
        &self,
        query: &str,
        k: usize,
        filters: crate::search::SearchFilters<'_>,
    ) -> Result<Vec<crate::search::SearchHit>> {
        let safe = sanitize_fts_query(query);
        if safe.is_empty() || k == 0 {
            return Ok(Vec::new());
        }
        let (where_clause, mut params) = keyword_filter_where_clause(filters)?;
        let sql = format!(
            "WITH ranked AS (
               SELECT n.id, n.title, n.type, n.path, n.asset, n.remote_url,
                      n.thumbnail_url, n.media_kind, n.mime, bm25(notes_fts) AS score
               FROM notes_fts JOIN notes n ON n.id = notes_fts.id
               WHERE notes_fts MATCH ? {where_clause}
               ORDER BY score
               LIMIT ?
             )
             SELECT id, chunk_id, title, type, text, path, asset, remote_url,
                    thumbnail_url, media_kind, mime, score
             FROM (
               SELECT
                 ranked.id,
                 c.id AS chunk_id,
                 ranked.title,
                 ranked.type,
                 c.text,
                 ranked.path,
                 ranked.asset,
                 ranked.remote_url,
                 ranked.thumbnail_url,
                 ranked.media_kind,
                 ranked.mime,
                 ranked.score,
                 row_number() OVER (
                   PARTITION BY ranked.id
                   ORDER BY CASE WHEN c.text LIKE ? ESCAPE '\\' THEN 0 ELSE 1 END, c.ord
                 ) AS rn
               FROM ranked
               JOIN chunks c ON c.note_id = ranked.id
             )
             WHERE rn = 1
             ORDER BY score"
        );
        params.insert(0, Value::Text(safe));
        params.push(Value::Integer(k as i64));
        params.push(Value::Text(snippet_match_pattern(query)));
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(params_from_iter(params.iter()), |r| {
            let score: f64 = r.get(11)?;
            Ok(crate::search::SearchHit {
                note_id: r.get(0)?,
                chunk_id: r.get(1)?,
                title: r.get(2)?,
                note_type: r.get(3)?,
                snippet: r.get(4)?,
                path: r.get(5)?,
                asset: r.get(6)?,
                remote_url: r.get(7)?,
                thumbnail_url: r.get(8)?,
                media_kind: r.get(9)?,
                mime: r.get(10)?,
                score: -score,
                rank_source: crate::search::RankSource::Keyword,
            })
        })?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    pub fn rebuild_chunk_embeddings<E: crate::embed::Embedder + ?Sized>(
        &mut self,
        embedder: &E,
    ) -> Result<usize> {
        reset_vec_schema(&self.conn, embedder.dimension())?;
        let chunks = collect_chunks_for_embedding(&self.conn)?;
        let texts: Vec<String> = chunks.iter().map(|(_, text)| text.clone()).collect();
        let vectors = embedder.embed_passages(&texts)?;
        if vectors.len() != chunks.len() {
            return Err(anyhow!(
                "embedder returned {} vectors for {} chunks",
                vectors.len(),
                chunks.len()
            ));
        }

        let tx = self.conn.transaction()?;
        for (idx, ((chunk_id, _), vector)) in chunks.iter().zip(vectors.iter()).enumerate() {
            if vector.len() != embedder.dimension() {
                return Err(anyhow!(
                    "embedding dimension mismatch for {chunk_id}: expected {}, got {}",
                    embedder.dimension(),
                    vector.len()
                ));
            }
            let rowid = (idx + 1) as i64;
            tx.execute(
                "INSERT INTO vec_chunks(rowid, embedding) VALUES (?1, ?2)",
                params![rowid, crate::embed::f32s_to_vec_blob(vector)],
            )?;
            tx.execute(
                "INSERT INTO chunk_embedding_map(rowid, chunk_id) VALUES (?1, ?2)",
                params![rowid, chunk_id],
            )?;
        }
        tx.commit()?;
        Ok(chunks.len())
    }

    pub fn search_vector_chunks<E: crate::embed::Embedder + ?Sized>(
        &self,
        query: &str,
        k: usize,
        embedder: &E,
    ) -> Result<Vec<VectorChunkHit>> {
        if k == 0 || !table_exists(&self.conn, "vec_chunks")? {
            return Ok(Vec::new());
        }
        let query_vector = embedder.embed_query(query)?;
        if query_vector.len() != embedder.dimension() {
            return Err(anyhow!(
                "query embedding dimension mismatch: expected {}, got {}",
                embedder.dimension(),
                query_vector.len()
            ));
        }
        let query_blob = crate::embed::f32s_to_vec_blob(&query_vector);
        let mut stmt = self.conn.prepare(
            "WITH matches AS (
               SELECT rowid, distance
               FROM vec_chunks
               WHERE embedding MATCH ?1 AND k = ?2
             )
             SELECT c.id, c.note_id, n.title, n.type, c.text, n.path, n.asset,
                    n.remote_url, n.thumbnail_url, n.media_kind, n.mime, matches.distance
             FROM matches
             JOIN chunk_embedding_map m ON m.rowid = matches.rowid
             JOIN chunks c ON c.id = m.chunk_id
             JOIN notes n ON n.id = c.note_id
             ORDER BY matches.distance",
        )?;
        let rows = stmt.query_map(params![query_blob, k as i64], |r| {
            Ok(VectorChunkHit {
                chunk_id: r.get(0)?,
                note_id: r.get(1)?,
                title: r.get(2)?,
                note_type: r.get(3)?,
                text: r.get(4)?,
                path: r.get(5)?,
                asset: r.get(6)?,
                remote_url: r.get(7)?,
                thumbnail_url: r.get(8)?,
                media_kind: r.get(9)?,
                mime: r.get(10)?,
                distance: r.get(11)?,
            })
        })?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    pub fn search_vector_chunk_hits<E: crate::embed::Embedder + ?Sized>(
        &self,
        query: &str,
        k: usize,
        embedder: &E,
    ) -> Result<Vec<crate::search::SearchHit>> {
        Ok(self
            .search_vector_chunks(query, k, embedder)?
            .into_iter()
            .map(|hit| crate::search::SearchHit {
                note_id: hit.note_id,
                chunk_id: hit.chunk_id,
                title: hit.title,
                note_type: hit.note_type,
                snippet: hit.text,
                path: hit.path,
                asset: hit.asset,
                remote_url: hit.remote_url,
                thumbnail_url: hit.thumbnail_url,
                media_kind: hit.media_kind,
                mime: hit.mime,
                score: 1.0 - f64::from(hit.distance),
                rank_source: crate::search::RankSource::Vector,
            })
            .collect())
    }

    pub fn all_notes(&self) -> Result<Vec<Note>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, path, type, title, tags, created, asset, remote_url, thumbnail_url,
                    media_kind, mime, body, mtime
             FROM notes
             ORDER BY id",
        )?;
        let rows = stmt.query_map([], note_from_row)?;
        let mut notes = Vec::new();
        for row in rows {
            let mut note = row?;
            note.related = collect_related_for_note(&self.conn, &note.id)?;
            notes.push(note);
        }
        Ok(notes)
    }

    pub fn note_by_id(&self, id: &str) -> Result<Option<Note>> {
        let mut note = self
            .conn
            .query_row(
                "SELECT id, path, type, title, tags, created, asset, remote_url, thumbnail_url,
                        media_kind, mime, body, mtime
                 FROM notes
                 WHERE id = ?1",
                [id],
                note_from_row,
            )
            .optional()?;
        if let Some(note) = &mut note {
            note.related = collect_related_for_note(&self.conn, &note.id)?;
        }
        Ok(note)
    }

    pub fn gallery_notes(&self, note_type: Option<&str>, k: usize) -> Result<Vec<Note>> {
        let mut sql = String::from(
            "SELECT id, path, type, title, tags, created, asset, remote_url, thumbnail_url,
                    media_kind, mime, body, mtime
             FROM notes
             WHERE asset IS NOT NULL OR thumbnail_url IS NOT NULL",
        );
        let mut values = Vec::new();
        if let Some(note_type) = note_type {
            sql.push_str(" AND type = ?");
            values.push(Value::Text(note_type.to_string()));
        }
        sql.push_str(" ORDER BY id LIMIT ?");
        values.push(Value::Integer(k.max(1) as i64));

        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(params_from_iter(values.iter()), note_from_row)?;
        let mut notes = Vec::new();
        for row in rows {
            notes.push(row?);
        }
        Ok(notes)
    }

    pub fn timeline_notes(
        &self,
        from: Option<&str>,
        to: Option<&str>,
        note_type: Option<&str>,
        k: usize,
    ) -> Result<Vec<crate::pack::TimelineNote>> {
        let mut sql = String::from(
            "SELECT id, title, type, path, created
             FROM notes
             WHERE created IS NOT NULL",
        );
        let mut values = Vec::new();
        if let Some(note_type) = note_type {
            sql.push_str(" AND type = ?");
            values.push(Value::Text(note_type.to_string()));
        }
        if let Some(from) = from {
            sql.push_str(" AND created >= ?");
            values.push(Value::Text(from.to_string()));
        }
        if let Some(to) = to {
            sql.push_str(" AND created <= ?");
            values.push(Value::Text(to.to_string()));
        }
        sql.push_str(" ORDER BY created DESC, id ASC LIMIT ?");
        values.push(Value::Integer(k as i64));

        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(params_from_iter(values.iter()), |r| {
            Ok(crate::pack::TimelineNote {
                id: r.get(0)?,
                title: r.get(1)?,
                note_type: r.get(2)?,
                path: std::path::PathBuf::from(r.get::<_, String>(3)?),
                created: r.get(4)?,
            })
        })?;
        let mut notes = Vec::new();
        for row in rows {
            notes.push(row?);
        }
        Ok(notes)
    }

    pub fn related_notes(
        &self,
        note_id: &str,
        depth: usize,
    ) -> Result<Vec<crate::pack::RelatedNote>> {
        let mut out = Vec::new();
        let mut seen = std::collections::HashSet::new();
        let mut queue = std::collections::VecDeque::from([(note_id.to_string(), 0usize)]);

        while let Some((current_id, current_depth)) = queue.pop_front() {
            if current_depth >= depth {
                continue;
            }
            for next_id in collect_related_for_note(&self.conn, &current_id)? {
                if !seen.insert(next_id.clone()) || next_id == note_id {
                    continue;
                }
                if let Some(next) = self.note_by_id(&next_id)? {
                    let next_depth = current_depth + 1;
                    out.push(crate::pack::RelatedNote {
                        id: next.id.clone(),
                        title: next.title.clone(),
                        note_type: next.note_type.clone(),
                        path: next.path.clone(),
                        depth: next_depth,
                    });
                    queue.push_back((next_id, next_depth));
                }
            }
        }
        Ok(out)
    }

    pub fn graph(&self, note_type: Option<&str>, limit: usize) -> Result<crate::pack::GraphData> {
        let mut values = Vec::new();
        let mut sql = String::from("SELECT id, title, type FROM notes");
        if let Some(note_type) = note_type {
            sql.push_str(" WHERE type = ?");
            values.push(Value::Text(note_type.to_string()));
        }
        sql.push_str(" ORDER BY id LIMIT ?");
        values.push(Value::Integer(limit as i64));

        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(params_from_iter(values.iter()), |r| {
            Ok(crate::pack::GraphNode {
                id: r.get(0)?,
                title: r.get(1)?,
                note_type: r.get(2)?,
            })
        })?;
        let mut nodes = Vec::new();
        for row in rows {
            nodes.push(row?);
        }

        let included: std::collections::HashSet<_> =
            nodes.iter().map(|node| node.id.as_str()).collect();
        if included.is_empty() {
            return Ok(crate::pack::GraphData {
                nodes,
                edges: Vec::new(),
            });
        }

        let mut stmt = self
            .conn
            .prepare("SELECT src, dst FROM edges ORDER BY src, dst")?;
        let rows = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?;
        let mut edges = Vec::new();
        for row in rows {
            let (src, dst) = row?;
            if included.contains(src.as_str()) && included.contains(dst.as_str()) {
                edges.push((src, dst));
            }
        }
        Ok(crate::pack::GraphData { nodes, edges })
    }

    pub fn facets(&self) -> Result<crate::pack::FacetValues> {
        let mut types = Vec::new();
        let mut stmt = self
            .conn
            .prepare("SELECT DISTINCT type FROM notes ORDER BY type")?;
        let rows = stmt.query_map([], |r| r.get(0))?;
        for row in rows {
            types.push(row?);
        }

        let mut tags = std::collections::BTreeSet::new();
        let mut stmt = self.conn.prepare("SELECT tags FROM notes")?;
        let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
        for row in rows {
            let tags_json = row?;
            let parsed: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();
            tags.extend(parsed);
        }

        let (created_min, created_max) = self.conn.query_row(
            "SELECT min(created), max(created) FROM notes WHERE created IS NOT NULL",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?;

        Ok(crate::pack::FacetValues {
            types,
            tags: tags.into_iter().collect(),
            created_min,
            created_max,
        })
    }
}

fn note_from_row(r: &Row<'_>) -> rusqlite::Result<Note> {
    let tags_json: String = r.get(4)?;
    let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();
    Ok(Note {
        id: r.get(0)?,
        path: std::path::PathBuf::from(r.get::<_, String>(1)?),
        note_type: r.get(2)?,
        title: r.get(3)?,
        tags,
        created: r.get(5)?,
        asset: r.get(6)?,
        remote_url: r.get(7)?,
        thumbnail_url: r.get(8)?,
        media_kind: r.get(9)?,
        mime: r.get(10)?,
        related: Vec::new(),
        body: r.get(11)?,
        mtime: r.get(12)?,
    })
}

fn collect_related_for_note(conn: &Connection, note_id: &str) -> Result<Vec<String>> {
    let mut stmt = conn.prepare("SELECT dst FROM edges WHERE src = ?1 ORDER BY dst")?;
    let rows = stmt.query_map([note_id], |r| r.get(0))?;
    let mut related = Vec::new();
    for row in rows {
        related.push(row?);
    }
    Ok(related)
}

fn keyword_filter_where_clause(
    filters: crate::search::SearchFilters<'_>,
) -> Result<(String, Vec<Value>)> {
    let mut where_clause = String::new();
    let mut params = Vec::new();

    if let Some(note_type) = filters.note_type {
        where_clause.push_str(" AND n.type = ?");
        params.push(Value::Text(note_type.to_string()));
    }
    if let Some(tag) = filters.tag {
        where_clause.push_str(" AND n.tags LIKE ? ESCAPE '\\'");
        params.push(Value::Text(format!(
            "%{}%",
            escape_like(&serde_json::to_string(tag)?)
        )));
    }
    if let Some(from) = filters.from {
        where_clause.push_str(" AND n.created IS NOT NULL AND n.created >= ?");
        params.push(Value::Text(from.to_string()));
    }
    if let Some(to) = filters.to {
        where_clause.push_str(" AND n.created IS NOT NULL AND n.created <= ?");
        params.push(Value::Text(to.to_string()));
    }

    Ok((where_clause, params))
}

fn escape_like(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

fn snippet_match_pattern(query: &str) -> String {
    let term = query
        .split_whitespace()
        .map(str::trim)
        .filter(|term| !term.is_empty())
        .max_by_key(|term| term.chars().count())
        .unwrap_or(query);
    format!("%{}%", escape_like(term))
}

fn register_sqlite_vec_extension() {
    static REGISTER: Once = Once::new();
    REGISTER.call_once(|| unsafe {
        type ExtensionEntry = unsafe extern "C" fn(
            *mut rusqlite::ffi::sqlite3,
            *mut *mut std::os::raw::c_char,
            *const rusqlite::ffi::sqlite3_api_routines,
        ) -> std::os::raw::c_int;
        let entry = std::mem::transmute::<*const (), ExtensionEntry>(
            sqlite_vec::sqlite3_vec_init as *const (),
        );
        rusqlite::ffi::sqlite3_auto_extension(Some(entry));
    });
}

fn reset_vec_schema(conn: &Connection, dimension: usize) -> Result<()> {
    conn.execute_batch(
        "DROP TABLE IF EXISTS vec_chunks; DROP TABLE IF EXISTS chunk_embedding_map;",
    )?;
    conn.execute(
        &format!(
            "CREATE VIRTUAL TABLE vec_chunks USING vec0(embedding float[{dimension}] distance_metric=cosine)"
        ),
        [],
    )?;
    conn.execute(
        "CREATE TABLE chunk_embedding_map (
           rowid    INTEGER PRIMARY KEY,
           chunk_id TEXT NOT NULL UNIQUE,
           FOREIGN KEY(chunk_id) REFERENCES chunks(id) ON DELETE CASCADE
         )",
        [],
    )?;
    Ok(())
}

fn collect_chunks_for_embedding(conn: &Connection) -> Result<Vec<(String, String)>> {
    let mut stmt = conn.prepare("SELECT id, text FROM chunks ORDER BY note_id, ord")?;
    let rows = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?;
    let mut chunks = Vec::new();
    for row in rows {
        chunks.push(row?);
    }
    Ok(chunks)
}

fn reset_legacy_derived_schema_if_needed(conn: &Connection) -> Result<()> {
    if !table_exists(conn, "notes")? {
        return Ok(());
    }
    let has_hash = table_has_column(conn, "notes", "hash")?;
    let has_chunks = table_exists(conn, "chunks")?;
    let has_thumbnail_url = table_has_column(conn, "notes", "thumbnail_url")?;
    let has_remote_url = table_has_column(conn, "notes", "remote_url")?;
    if has_hash && has_chunks && has_thumbnail_url && has_remote_url {
        return Ok(());
    }
    conn.execute_batch(
        "
        DROP TABLE IF EXISTS chunks;
        DROP TABLE IF EXISTS vec_chunks;
        DROP TABLE IF EXISTS chunk_embedding_map;
        DROP TABLE IF EXISTS notes_fts;
        DROP TABLE IF EXISTS edges;
        DROP TABLE IF EXISTS notes;
        ",
    )?;
    Ok(())
}

fn table_exists(conn: &Connection, table: &str) -> Result<bool> {
    let exists: i64 = conn.query_row(
        "SELECT count(*) FROM sqlite_master WHERE type IN ('table','virtual table') AND name = ?1",
        [table],
        |r| r.get(0),
    )?;
    Ok(exists > 0)
}

fn table_has_column(conn: &Connection, table: &str, column: &str) -> Result<bool> {
    let mut stmt = conn.prepare(&format!("PRAGMA table_info({})", quote_ident(table)))?;
    let rows = stmt.query_map([], |r| r.get::<_, String>(1))?;
    for row in rows {
        if row? == column {
            return Ok(true);
        }
    }
    Ok(false)
}

fn quote_ident(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\""))
}

fn collect_existing_note_ids(tx: &Transaction<'_>) -> Result<Vec<String>> {
    let mut stmt = tx.prepare("SELECT id FROM notes")?;
    let rows = stmt.query_map([], |r| r.get(0))?;
    let mut ids = Vec::new();
    for row in rows {
        ids.push(row?);
    }
    Ok(ids)
}

fn delete_note_rows(tx: &Transaction<'_>, note_id: &str) -> Result<()> {
    tx.execute("DELETE FROM chunks WHERE note_id = ?1", [note_id])?;
    tx.execute("DELETE FROM notes_fts WHERE id = ?1", [note_id])?;
    tx.execute("DELETE FROM edges WHERE src = ?1 OR dst = ?1", [note_id])?;
    tx.execute("DELETE FROM notes WHERE id = ?1", [note_id])?;
    Ok(())
}

fn insert_note_rows(tx: &Transaction<'_>, note: &Note, hash: &str) -> Result<()> {
    let tags_json = serde_json::to_string(&note.tags)?;
    let tags_text = note.tags.join(" ");
    tx.execute(
        "INSERT INTO notes (id, path, type, title, tags, created, asset, remote_url, thumbnail_url,
                            media_kind, mime, body, mtime, hash)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
        rusqlite::params![
            note.id,
            note.path.to_string_lossy(),
            note.note_type,
            note.title,
            tags_json,
            note.created,
            note.asset,
            note.remote_url,
            note.thumbnail_url,
            note.media_kind,
            note.mime,
            note.body,
            note.mtime,
            hash,
        ],
    )?;
    tx.execute(
        "INSERT INTO notes_fts (id, title, body, tags) VALUES (?1, ?2, ?3, ?4)",
        rusqlite::params![note.id, note.title, note.body, tags_text],
    )?;
    for chunk in crate::chunk::chunk_text(&note.id, &note.body, 900, 120) {
        tx.execute(
            "INSERT INTO chunks (id, note_id, ord, text) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![chunk.id, chunk.note_id, chunk.ord, chunk.text],
        )?;
    }
    for dst in &note.related {
        tx.execute(
            "INSERT OR IGNORE INTO edges (src, dst, kind) VALUES (?1, ?2, ?3)",
            rusqlite::params![note.id, dst, "related"],
        )?;
    }
    Ok(())
}

fn reject_duplicate_note_ids(notes: &[Note]) -> Result<()> {
    let mut seen = std::collections::HashSet::new();
    for note in notes {
        if !seen.insert(note.id.as_str()) {
            return Err(anyhow!("duplicate note id: {}", note.id));
        }
    }
    Ok(())
}

/// 사용자 질의를 FTS5 MATCH에 넣기 안전하게 변환: 토큰을 "..."로 감싸 OR.
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
    use crate::embed::FakeEmbedder;
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
    fn creates_chunks_table_in_memory() {
        let idx = Index::open_in_memory().unwrap();
        let count: i64 = idx
            .conn
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE name = 'chunks'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn init_resets_legacy_derived_index_schema() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "
            CREATE TABLE notes (
              id TEXT PRIMARY KEY,
              path TEXT NOT NULL,
              type TEXT NOT NULL,
              title TEXT NOT NULL,
              tags TEXT NOT NULL,
              created TEXT,
              asset TEXT,
              body TEXT NOT NULL,
              mtime INTEGER NOT NULL
            );
            CREATE VIRTUAL TABLE notes_fts USING fts5(id UNINDEXED, title, body, tags);
            CREATE TABLE edges (
              src TEXT NOT NULL,
              dst TEXT NOT NULL,
              kind TEXT NOT NULL,
              PRIMARY KEY (src, dst, kind)
            );
            INSERT INTO notes (id, path, type, title, tags, body, mtime)
            VALUES ('old', 'notes/old.md', 'note', 'old', '[]', 'old body', 0);
            ",
        )
        .unwrap();

        let mut idx = Index::init(conn).unwrap();
        let n = parse_str("fresh", "새 본문").unwrap();
        idx.rebuild(&[n]).unwrap();

        let columns: Vec<String> = {
            let mut stmt = idx.conn.prepare("PRAGMA table_info(notes)").unwrap();
            stmt.query_map([], |r| r.get(1))
                .unwrap()
                .collect::<rusqlite::Result<_>>()
                .unwrap()
        };
        assert!(columns.iter().any(|c| c == "hash"));
        let chunks_table: i64 = idx
            .conn
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE name = 'chunks'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(chunks_table, 1);
    }

    #[test]
    fn rebuild_inserts_chunks_for_notes() {
        let mut idx = Index::open_in_memory().unwrap();
        let n = parse_str("a", "0123456789abcdef").unwrap();
        idx.rebuild(&[n]).unwrap();

        let chunks: i64 = idx
            .conn
            .query_row("SELECT count(*) FROM chunks WHERE note_id = 'a'", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert!(chunks >= 1);

        let first: String = idx
            .conn
            .query_row(
                "SELECT id FROM chunks WHERE note_id = 'a' ORDER BY ord LIMIT 1",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(first, "a#0000");
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

    #[test]
    fn incremental_rebuild_skips_unchanged_notes() {
        let mut idx = Index::open_in_memory().unwrap();
        let n = parse_str("a", "본문").unwrap();
        let first = idx.rebuild_incremental(std::slice::from_ref(&n)).unwrap();
        assert_eq!(first.indexed, 1);
        assert_eq!(first.skipped, 0);

        let second = idx.rebuild_incremental(std::slice::from_ref(&n)).unwrap();
        assert_eq!(second.indexed, 0);
        assert_eq!(second.skipped, 1);
    }

    #[test]
    fn incremental_rebuild_removes_missing_notes() {
        let mut idx = Index::open_in_memory().unwrap();
        let a = parse_str("a", "A").unwrap();
        let b = parse_str("b", "B").unwrap();
        idx.rebuild_incremental(&[a]).unwrap();
        let report = idx.rebuild_incremental(&[b]).unwrap();
        assert_eq!(report.removed, 1);

        let a_count: i64 = idx
            .conn
            .query_row("SELECT count(*) FROM notes WHERE id = 'a'", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(a_count, 0);
    }

    #[test]
    fn vector_search_finds_semantic_chunk_without_keyword_overlap() {
        let mut idx = Index::open_in_memory().unwrap();
        let lesson = parse_str("lesson", "수업 설계 절차").unwrap();
        let whale = parse_str("whale", "바다 고래 관찰").unwrap();
        idx.rebuild(&[lesson, whale]).unwrap();

        let embedder = FakeEmbedder::new(3)
            .with_passage("수업 설계 절차", vec![1.0, 0.0, 0.0])
            .with_passage("바다 고래 관찰", vec![0.0, 1.0, 0.0])
            .with_query("강의 준비", vec![0.95, 0.05, 0.0]);

        let indexed = idx.rebuild_chunk_embeddings(&embedder).unwrap();
        assert_eq!(indexed, 2);

        let hits = idx.search_vector_chunks("강의 준비", 2, &embedder).unwrap();
        assert_eq!(hits[0].note_id, "lesson");
        assert_eq!(hits[0].chunk_id, "lesson#0000");
        assert!(hits[0].text.contains("수업 설계"));
        assert!(!hits[0].text.contains("강의"));
    }

    #[test]
    fn keyword_chunk_search_returns_citation_ready_cards() {
        let mut idx = Index::open_in_memory().unwrap();
        let mut note = parse_str(
            "hook",
            "---\ntype: prompt\ntitle: 썸네일 훅\n---\n클릭을 부르는 훅 카피.",
        )
        .unwrap();
        note.path = "notes/hook.md".into();
        idx.rebuild(&[note]).unwrap();

        let hits = idx.search_keyword_chunks("훅", 10).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].note_id, "hook");
        assert_eq!(hits[0].chunk_id, "hook#0000");
        assert_eq!(hits[0].title, "썸네일 훅");
        assert_eq!(hits[0].note_type, "prompt");
        assert_eq!(hits[0].path, "notes/hook.md");
        assert!(hits[0].snippet.contains("클릭을 부르는 훅"));
        assert_eq!(hits[0].rank_source, crate::search::RankSource::Keyword);
    }

    #[test]
    fn keyword_chunk_search_prefers_chunk_containing_query() {
        let mut idx = Index::open_in_memory().unwrap();
        let prefix = "도입 설명 ".repeat(260);
        let mut note = parse_str(
            "deep",
            &format!("---\ntitle: 긴 노트\n---\n{prefix}\n\n정확한 니들 문장이 뒤쪽 청크에 있다."),
        )
        .unwrap();
        note.path = "notes/deep.md".into();
        idx.rebuild(&[note]).unwrap();

        let hits = idx.search_keyword_chunks("니들", 10).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].note_id, "deep");
        assert_ne!(hits[0].chunk_id, "deep#0000");
        assert!(hits[0].snippet.contains("니들"));
    }

    #[test]
    fn keyword_chunk_search_filters_before_limit() {
        let mut idx = Index::open_in_memory().unwrap();
        let mut notes = Vec::new();
        for i in 0..101 {
            notes.push(
                parse_str(
                    &format!("distractor-{i:03}"),
                    &format!("---\ntype: note\ntitle: Distractor {i:03}\n---\ncommon term {i}"),
                )
                .unwrap(),
            );
        }
        notes.push(
            parse_str(
                "target",
                "---\ntype: prompt\ntitle: Target\ntags: [needle]\ncreated: 2026-05-23\n---\ncommon term target",
            )
            .unwrap(),
        );
        idx.rebuild(&notes).unwrap();

        let hits = idx
            .search_keyword_chunks_filtered(
                "common",
                1,
                crate::search::SearchFilters {
                    note_type: Some("prompt"),
                    tag: Some("needle"),
                    from: Some("2026-01-01"),
                    to: Some("2026-12-31"),
                },
            )
            .unwrap();

        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].note_id, "target");
    }

    #[test]
    fn endpoint_specific_index_reads_return_filtered_rows() {
        let mut idx = Index::open_in_memory().unwrap();
        let mut image = parse_str(
            "image",
            "---\ntype: image\ntitle: Image\ntags: [gallery, visual]\ncreated: 2026-05-20\nthumbnail_url: https://example.test/thumb.jpg\nremote_url: https://example.test/image\nmedia_kind: image\nmime: image/jpeg\nrelated: [prompt]\n---\n캡션",
        )
        .unwrap();
        image.path = "notes/image.md".into();
        let mut prompt = parse_str(
            "prompt",
            "---\ntype: prompt\ntitle: Prompt\ntags: [needle]\ncreated: 2026-05-21\nrelated: [image]\n---\n본문",
        )
        .unwrap();
        prompt.path = "notes/prompt.md".into();
        idx.rebuild(&[image, prompt]).unwrap();

        let note = idx.note_by_id("image").unwrap().unwrap();
        assert_eq!(note.title, "Image");
        assert_eq!(
            note.remote_url.as_deref(),
            Some("https://example.test/image")
        );
        assert_eq!(
            note.thumbnail_url.as_deref(),
            Some("https://example.test/thumb.jpg")
        );
        assert_eq!(note.media_kind.as_deref(), Some("image"));
        assert_eq!(note.mime.as_deref(), Some("image/jpeg"));
        assert_eq!(note.related, vec!["prompt"]);

        let gallery = idx.gallery_notes(Some("image"), 10).unwrap();
        assert_eq!(
            gallery
                .iter()
                .map(|note| note.id.as_str())
                .collect::<Vec<_>>(),
            vec!["image"]
        );

        let timeline = idx
            .timeline_notes(Some("2026-05-21"), Some("2026-05-21"), Some("prompt"), 10)
            .unwrap();
        assert_eq!(timeline[0].id, "prompt");

        let related = idx.related_notes("image", 1).unwrap();
        assert_eq!(related[0].id, "prompt");

        let graph = idx.graph(None, 10).unwrap();
        assert_eq!(graph.nodes.len(), 2);
        assert!(graph
            .edges
            .contains(&("image".to_string(), "prompt".to_string())));

        let facets = idx.facets().unwrap();
        assert_eq!(facets.types, vec!["image", "prompt"]);
        assert!(facets.tags.contains(&"gallery".to_string()));
        assert_eq!(facets.created_min.as_deref(), Some("2026-05-20"));
        assert_eq!(facets.created_max.as_deref(), Some("2026-05-21"));
    }
}
