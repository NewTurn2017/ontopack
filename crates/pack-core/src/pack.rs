use crate::config::PackConfig;
use crate::enrichment::{self, EnrichmentPatch, EnrichmentStatus};
use crate::index::{Index, VectorChunkHit};
use crate::note::{self, Note};
use crate::process::{infer_type, ProcessReport};
use crate::search::{rrf_fuse, NoteHit, SearchFilters, SearchHit};
use anyhow::{anyhow, bail, Context, Result};
use serde::Serialize;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use walkdir::WalkDir;

#[derive(Clone)]
pub struct Pack {
    pub root: PathBuf,
    pub config: PackConfig,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AddOutcome {
    Note {
        path: PathBuf,
    },
    AssetWithSidecar {
        asset_path: PathBuf,
        note_path: PathBuf,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelatedNote {
    pub id: String,
    pub title: String,
    pub note_type: String,
    pub path: PathBuf,
    pub depth: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimelineNote {
    pub id: String,
    pub title: String,
    pub note_type: String,
    pub path: PathBuf,
    pub created: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphNode {
    pub id: String,
    pub title: String,
    pub note_type: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphData {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<(String, String)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FacetValues {
    pub types: Vec<String>,
    pub tags: Vec<String>,
    pub created_min: Option<String>,
    pub created_max: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PackObject {
    pub note_id: String,
    pub title: String,
    pub note_type: String,
    pub kind: String,
    pub note_path: String,
    pub asset_path: Option<String>,
    pub content_hash: String,
    pub indexed: bool,
    pub enrichment_status: EnrichmentStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PackStatus {
    pub total: usize,
    pub notes: usize,
    pub assets: usize,
    pub indexed: usize,
    pub pending_enrichment: usize,
    pub done_enrichment: usize,
    pub error_enrichment: usize,
    pub objects: Vec<PackObject>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DuplicateCandidate {
    pub note_id: String,
    pub title: String,
    pub note_type: String,
    pub path: String,
    pub asset: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DuplicateGroup {
    pub fingerprint: String,
    pub normalized_len: usize,
    pub candidates: Vec<DuplicateCandidate>,
}

#[derive(Serialize)]
struct AssetSidecarFrontMatter<'a> {
    #[serde(rename = "type")]
    note_type: &'a str,
    title: &'a str,
    asset: &'a str,
    tags: Vec<String>,
}

#[derive(Serialize)]
struct ContentNoteFrontMatter<'a> {
    #[serde(rename = "type")]
    note_type: &'a str,
    title: &'a str,
    tags: &'a [String],
}

impl Pack {
    /// 새 팩 골격 생성: pack.toml + notes/ assets/ _inbox/ .pack/
    pub fn init(root: &Path, name: &str) -> Result<()> {
        std::fs::create_dir_all(root.join("notes"))?;
        std::fs::create_dir_all(root.join("assets"))?;
        std::fs::create_dir_all(root.join("_inbox"))?;
        std::fs::create_dir_all(root.join(".pack"))?;
        let toml_path = root.join("pack.toml");
        if !toml_path.exists() {
            std::fs::write(&toml_path, PackConfig::default_toml(name))?;
        }
        Ok(())
    }

    /// pack.toml이 있는 디렉터리를 팩 루트로 연다.
    pub fn open(root: &Path) -> Result<Pack> {
        let config = PackConfig::load(root)?;
        Ok(Pack {
            root: root.to_path_buf(),
            config,
        })
    }

    /// notes/ 아래 모든 .md를 Note로 읽는다.
    pub fn scan_notes(&self) -> Result<Vec<Note>> {
        let notes_dir = self.root.join("notes");
        let mut out = Vec::new();
        for entry in WalkDir::new(&notes_dir) {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("md") {
                let id = note_id_from_notes_path(&notes_dir, path)?;
                out.push(note::parse_file_with_id(path, &id)?);
            }
        }
        Ok(out)
    }

    /// Viewer/API fast path: read derived note rows from SQLite when an index exists.
    /// Falls back to source markdown scanning for brand-new packs that have not been built yet.
    pub fn indexed_notes_or_scan(&self) -> Result<Vec<Note>> {
        let index_path = self.index_path();
        if index_path.exists() {
            return Index::open(&index_path)?.all_notes();
        }
        self.scan_notes()
    }

    pub fn note_by_id_or_scan(&self, id: &str) -> Result<Option<Note>> {
        let index_path = self.index_path();
        if index_path.exists() {
            return Index::open(&index_path)?.note_by_id(id);
        }
        Ok(self.scan_notes()?.into_iter().find(|note| note.id == id))
    }

    pub fn gallery_notes_or_scan(&self, note_type: Option<&str>, k: usize) -> Result<Vec<Note>> {
        let index_path = self.index_path();
        if index_path.exists() {
            return Index::open(&index_path)?.gallery_notes(note_type, k);
        }
        let mut items = Vec::new();
        for note in self.scan_notes()? {
            if note.asset.is_none()
                || note_type.is_some_and(|note_type| note.note_type != note_type)
            {
                continue;
            }
            items.push(note);
            if items.len() >= k.max(1) {
                break;
            }
        }
        Ok(items)
    }

    pub fn timeline_notes_or_scan(
        &self,
        from: Option<&str>,
        to: Option<&str>,
        note_type: Option<&str>,
        k: usize,
    ) -> Result<Vec<TimelineNote>> {
        let index_path = self.index_path();
        if index_path.exists() {
            return Index::open(&index_path)?.timeline_notes(from, to, note_type, k);
        }
        self.timeline_notes(from, to, note_type, k)
    }

    pub fn related_notes_or_scan(&self, note_id: &str, depth: usize) -> Result<Vec<RelatedNote>> {
        let index_path = self.index_path();
        if index_path.exists() {
            return Index::open(&index_path)?.related_notes(note_id, depth);
        }
        self.related_notes(note_id, depth)
    }

    pub fn graph_or_scan(&self, note_type: Option<&str>, limit: usize) -> Result<GraphData> {
        let index_path = self.index_path();
        if index_path.exists() {
            return Index::open(&index_path)?.graph(note_type, limit);
        }
        let notes = self.scan_notes()?;
        Ok(graph_from_notes(&notes, note_type, limit))
    }

    pub fn facets_or_scan(&self) -> Result<FacetValues> {
        let index_path = self.index_path();
        if index_path.exists() {
            return Index::open(&index_path)?.facets();
        }
        Ok(facets_from_notes(self.scan_notes()?))
    }

    /// Source-of-truth notes/assets를 읽어 현재 팩 객체와 enrichment 상태를 계산한다.
    pub fn objects(&self) -> Result<Vec<PackObject>> {
        let indexed_ids = self.indexed_note_ids()?;
        let mut objects = self
            .scan_notes()?
            .into_iter()
            .map(|note| {
                let has_asset = note.asset.is_some();
                PackObject {
                    note_id: note.id.clone(),
                    title: note.title.clone(),
                    note_type: note.note_type.clone(),
                    kind: object_kind(&note),
                    note_path: relative_display(&self.root, &note.path),
                    asset_path: note.asset.clone(),
                    content_hash: note.content_hash(),
                    indexed: indexed_ids.contains(&note.id),
                    enrichment_status: enrichment::status_for_body(&note.body, has_asset),
                }
            })
            .collect::<Vec<_>>();
        objects.sort_by(|a, b| a.note_id.cmp(&b.note_id));
        Ok(objects)
    }

    pub fn status(&self) -> Result<PackStatus> {
        let objects = self.objects()?;
        let assets = objects
            .iter()
            .filter(|object| object.asset_path.is_some())
            .count();
        let indexed = objects.iter().filter(|object| object.indexed).count();
        let pending_enrichment = objects
            .iter()
            .filter(|object| object.enrichment_status == EnrichmentStatus::Pending)
            .count();
        let done_enrichment = objects
            .iter()
            .filter(|object| object.enrichment_status == EnrichmentStatus::Done)
            .count();
        let error_enrichment = objects
            .iter()
            .filter(|object| object.enrichment_status == EnrichmentStatus::Error)
            .count();
        Ok(PackStatus {
            total: objects.len(),
            notes: objects.len() - assets,
            assets,
            indexed,
            pending_enrichment,
            done_enrichment,
            error_enrichment,
            objects,
        })
    }

    /// `.pack/objects.jsonl`은 파생 ledger이며 source-of-truth notes/assets에서 재생성 가능하다.
    pub fn refresh_object_manifest(&self) -> Result<PathBuf> {
        std::fs::create_dir_all(self.root.join(".pack"))?;
        let manifest_path = self.root.join(".pack").join("objects.jsonl");
        let objects = self.objects()?;
        let mut body = String::new();
        for object in objects {
            body.push_str(&serde_json::to_string(&object)?);
            body.push('\n');
        }
        let tmp = temp_sibling_path(&manifest_path);
        if let Err(err) = std::fs::write(&tmp, body.as_bytes())
            .and_then(|_| std::fs::rename(&tmp, &manifest_path))
        {
            let _ = std::fs::remove_file(&tmp);
            return Err(err.into());
        }
        Ok(manifest_path)
    }

    pub fn pending_enrichment_objects(&self) -> Result<Vec<PackObject>> {
        Ok(self
            .objects()?
            .into_iter()
            .filter(|object| object.enrichment_status == EnrichmentStatus::Pending)
            .collect())
    }

    pub fn duplicate_notes(&self) -> Result<Vec<DuplicateGroup>> {
        let mut by_body: BTreeMap<String, Vec<Note>> = BTreeMap::new();
        for note in self.scan_notes()? {
            let normalized = normalize_duplicate_body(&note.body);
            if normalized.is_empty() {
                continue;
            }
            by_body.entry(normalized).or_default().push(note);
        }

        let mut groups = by_body
            .into_iter()
            .filter_map(|(normalized, mut notes)| {
                if notes.len() < 2 {
                    return None;
                }
                notes.sort_by(|a, b| a.id.cmp(&b.id));
                Some(DuplicateGroup {
                    fingerprint: stable_text_fingerprint(&normalized),
                    normalized_len: normalized.len(),
                    candidates: notes
                        .into_iter()
                        .map(|note| DuplicateCandidate {
                            note_id: note.id,
                            title: note.title,
                            note_type: note.note_type,
                            path: relative_display(&self.root, &note.path),
                            asset: note.asset,
                        })
                        .collect(),
                })
            })
            .collect::<Vec<_>>();
        groups.sort_by(|a, b| {
            b.candidates
                .len()
                .cmp(&a.candidates.len())
                .then_with(|| a.fingerprint.cmp(&b.fingerprint))
        });
        Ok(groups)
    }

    pub fn update_enrichment(&self, note_id: &str, patch: &EnrichmentPatch) -> Result<PathBuf> {
        let note = self
            .scan_notes()?
            .into_iter()
            .find(|note| note.id == note_id)
            .ok_or_else(|| anyhow!("note not found: {note_id}"))?;
        let raw = std::fs::read_to_string(&note.path)?;
        let updated = enrichment::apply_enrichment_patch(&raw, patch)?;
        atomic_write(&note.path, updated.as_bytes())?;
        Ok(note.path)
    }

    fn indexed_note_ids(&self) -> Result<std::collections::HashSet<String>> {
        if !self.index_path().exists() {
            return Ok(std::collections::HashSet::new());
        }
        Ok(Index::open(&self.index_path())?
            .all_notes()?
            .into_iter()
            .map(|note| note.id)
            .collect())
    }

    /// 인덱스 DB 경로 (.pack/index.db)
    pub fn index_path(&self) -> PathBuf {
        self.root.join(".pack").join("index.db")
    }

    /// 파일을 팩에 추가한다. md/markdown/txt는 notes/로, 그 외는 assets/와 사이드카 note로 복사한다.
    pub fn add_file(&self, file: &Path, note_type: &str) -> Result<AddOutcome> {
        let stem = file
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("untitled");
        let ext = file.extension().and_then(|s| s.to_str()).unwrap_or("");
        if ext == "md" || ext == "markdown" || ext == "txt" {
            let dst = self.root.join("notes").join(format!("{stem}.md"));
            copy_without_overwrite(file, &dst)?;
            Ok(AddOutcome::Note { path: dst })
        } else {
            let file_name = file
                .file_name()
                .context("추가할 파일 이름을 읽을 수 없습니다")?
                .to_string_lossy()
                .to_string();
            let asset_rel = format!("assets/{file_name}");
            let asset_path = self.root.join(&asset_rel);
            let note_path = self.root.join("notes").join(format!("{stem}.md"));
            ensure_missing(&asset_path)?;
            ensure_missing(&note_path)?;
            let frontmatter = serde_yaml::to_string(&AssetSidecarFrontMatter {
                note_type,
                title: stem,
                asset: &asset_rel,
                tags: Vec::new(),
            })?;
            let body = format!("---\n{frontmatter}---\n캡션을 적어주세요(검색 대상).\n");
            write_asset_and_sidecar(file, &asset_path, &note_path, body.as_bytes())?;
            Ok(AddOutcome::AssetWithSidecar {
                asset_path,
                note_path,
            })
        }
    }

    /// 문자열 콘텐츠를 notes/ 아래 새 markdown note로 추가한다.
    pub fn add_content_note(
        &self,
        title: &str,
        content: &str,
        note_type: &str,
        tags: &[String],
    ) -> Result<PathBuf> {
        let file_stem = safe_note_file_stem(title);
        let dst = unique_note_path(&self.root.join("notes"), &file_stem);
        let frontmatter = serde_yaml::to_string(&ContentNoteFrontMatter {
            note_type,
            title,
            tags,
        })?;
        let body = format!("---\n{frontmatter}---\n{content}\n");
        let tmp = temp_sibling_path(&dst);
        ensure_missing(&tmp)?;
        if let Err(err) =
            std::fs::write(&tmp, body.as_bytes()).and_then(|_| std::fs::rename(&tmp, &dst))
        {
            let _ = std::fs::remove_file(&tmp);
            return Err(err.into());
        }
        Ok(dst)
    }

    /// _inbox 바로 아래 파일을 notes/assets로 정리한다.
    pub fn process_inbox(&self) -> Result<ProcessReport> {
        let inbox = self.root.join("_inbox");
        let mut report = ProcessReport::default();
        for entry in WalkDir::new(&inbox).min_depth(1).max_depth(1) {
            let entry = entry?;
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let file_name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
            let inferred = infer_type(file_name);
            let outcome = self.add_file(path, inferred.as_note_type())?;
            match outcome {
                AddOutcome::Note { path } => report.created.push(path),
                AddOutcome::AssetWithSidecar { note_path, .. } => report.created.push(note_path),
            }
            std::fs::remove_file(path)?;
            report.processed += 1;
        }
        Ok(report)
    }

    /// note_id에서 시작해 related 링크를 depth 단계까지 따라간다.
    pub fn related_notes(&self, note_id: &str, depth: usize) -> Result<Vec<RelatedNote>> {
        let notes = self.scan_notes()?;
        let by_id: std::collections::HashMap<String, Note> = notes
            .into_iter()
            .map(|note| (note.id.clone(), note))
            .collect();
        let mut out = Vec::new();
        let mut seen = std::collections::HashSet::new();
        let mut queue = std::collections::VecDeque::from([(note_id.to_string(), 0usize)]);

        while let Some((current_id, current_depth)) = queue.pop_front() {
            if current_depth >= depth {
                continue;
            }
            let Some(current) = by_id.get(&current_id) else {
                continue;
            };
            for next_id in &current.related {
                if !seen.insert(next_id.clone()) || next_id == note_id {
                    continue;
                }
                if let Some(next) = by_id.get(next_id) {
                    let next_depth = current_depth + 1;
                    out.push(RelatedNote {
                        id: next.id.clone(),
                        title: next.title.clone(),
                        note_type: next.note_type.clone(),
                        path: next.path.clone(),
                        depth: next_depth,
                    });
                    queue.push_back((next_id.clone(), next_depth));
                }
            }
        }
        Ok(out)
    }

    /// created metadata 기준으로 노트를 최신순 나열한다.
    pub fn timeline_notes(
        &self,
        from: Option<&str>,
        to: Option<&str>,
        note_type: Option<&str>,
        k: usize,
    ) -> Result<Vec<TimelineNote>> {
        let mut notes: Vec<_> = self
            .scan_notes()?
            .into_iter()
            .filter(|note| note_type.is_none_or(|t| note.note_type == t))
            .filter(|note| {
                note.created.as_deref().is_some_and(|created| {
                    from.is_none_or(|from| created >= from) && to.is_none_or(|to| created <= to)
                })
            })
            .map(|note| TimelineNote {
                id: note.id,
                title: note.title,
                note_type: note.note_type,
                path: note.path,
                created: note.created,
            })
            .collect();
        notes.sort_by(|a, b| b.created.cmp(&a.created).then_with(|| a.id.cmp(&b.id)));
        notes.truncate(k);
        Ok(notes)
    }

    /// 현재 notes/ 상태로 전체 인덱스를 재빌드한다.
    pub fn build_index(&self) -> Result<usize> {
        let notes = self.scan_notes()?;
        let mut idx = Index::open(&self.index_path())?;
        idx.rebuild(&notes)?;
        Ok(notes.len())
    }

    /// 현재 notes/ 상태로 증분 인덱스를 갱신한다.
    pub fn build_index_incremental(&self) -> Result<crate::index::BuildReport> {
        let notes = self.scan_notes()?;
        let mut idx = Index::open(&self.index_path())?;
        idx.rebuild_incremental(&notes)
    }

    /// 현재 chunks 테이블을 기준으로 청크 임베딩을 재생성한다.
    pub fn build_chunk_embeddings_with<E: crate::embed::Embedder + ?Sized>(
        &self,
        embedder: &E,
    ) -> Result<usize> {
        let mut idx = Index::open(&self.index_path())?;
        idx.rebuild_chunk_embeddings(embedder)
    }

    /// 현재 팩 인덱스에서 키워드 검색을 수행한다.
    pub fn search_keyword(&self, query: &str, k: usize) -> Result<Vec<NoteHit>> {
        let idx = Index::open(&self.index_path())?;
        idx.search_keyword(query, k)
    }

    /// 현재 팩 인덱스에서 키워드 청크 카드를 검색한다.
    pub fn search_keyword_chunks(&self, query: &str, k: usize) -> Result<Vec<SearchHit>> {
        let idx = Index::open(&self.index_path())?;
        idx.search_keyword_chunks(query, k)
    }

    /// 현재 팩 인덱스에서 메타데이터 필터를 먼저 적용한 뒤 키워드 청크 카드를 검색한다.
    pub fn search_keyword_chunks_filtered(
        &self,
        query: &str,
        k: usize,
        filters: SearchFilters<'_>,
    ) -> Result<Vec<SearchHit>> {
        let idx = Index::open(&self.index_path())?;
        idx.search_keyword_chunks_filtered(query, k, filters)
    }

    /// 현재 팩 인덱스에서 벡터 청크 검색을 수행한다.
    pub fn search_vector_chunks_with<E: crate::embed::Embedder + ?Sized>(
        &self,
        query: &str,
        k: usize,
        embedder: &E,
    ) -> Result<Vec<VectorChunkHit>> {
        let idx = Index::open(&self.index_path())?;
        idx.search_vector_chunks(query, k, embedder)
    }

    /// 현재 팩 인덱스에서 벡터 청크 카드를 검색한다.
    pub fn search_vector_chunk_hits_with<E: crate::embed::Embedder + ?Sized>(
        &self,
        query: &str,
        k: usize,
        embedder: &E,
    ) -> Result<Vec<SearchHit>> {
        let idx = Index::open(&self.index_path())?;
        idx.search_vector_chunk_hits(query, k, embedder)
    }

    /// 키워드와 벡터 청크 검색 결과를 RRF로 융합한다.
    pub fn search_hybrid_with<E: crate::embed::Embedder + ?Sized>(
        &self,
        query: &str,
        k: usize,
        embedder: &E,
    ) -> Result<Vec<SearchHit>> {
        let idx = Index::open(&self.index_path())?;
        let keyword = idx.search_keyword_chunks(query, k)?;
        let vector = idx.search_vector_chunk_hits(query, k, embedder)?;
        Ok(rrf_fuse(&keyword, &vector, k))
    }
}

fn graph_from_notes(notes: &[Note], note_type: Option<&str>, limit: usize) -> GraphData {
    let mut nodes = Vec::new();
    let mut included = std::collections::HashSet::new();
    for note in notes
        .iter()
        .filter(|note| note_type.is_none_or(|note_type| note.note_type == note_type))
        .take(limit)
    {
        included.insert(note.id.clone());
        nodes.push(GraphNode {
            id: note.id.clone(),
            title: note.title.clone(),
            note_type: note.note_type.clone(),
        });
    }
    let edges = notes
        .iter()
        .filter(|note| included.contains(&note.id))
        .flat_map(|note| {
            note.related
                .iter()
                .filter(|to| included.contains(*to))
                .map(|to| (note.id.clone(), to.clone()))
        })
        .collect();
    GraphData { nodes, edges }
}

fn facets_from_notes(notes: Vec<Note>) -> FacetValues {
    let mut types = std::collections::BTreeSet::new();
    let mut tags = std::collections::BTreeSet::new();
    let mut created_values = Vec::new();
    for note in notes {
        types.insert(note.note_type);
        tags.extend(note.tags);
        if let Some(created) = note.created {
            created_values.push(created);
        }
    }
    created_values.sort();
    FacetValues {
        types: types.into_iter().collect(),
        tags: tags.into_iter().collect(),
        created_min: created_values.first().cloned(),
        created_max: created_values.last().cloned(),
    }
}

fn object_kind(note: &Note) -> String {
    if note.asset.is_none() {
        return "note".to_string();
    }
    match note.note_type.as_str() {
        "image" | "video" | "audio" | "pdf" => note.note_type.clone(),
        _ => note
            .asset
            .as_deref()
            .and_then(|asset| Path::new(asset).extension())
            .and_then(|ext| ext.to_str())
            .map(|ext| match ext.to_ascii_lowercase().as_str() {
                "png" | "jpg" | "jpeg" | "gif" | "webp" | "svg" => "image".to_string(),
                "mp4" | "mov" | "mkv" | "webm" => "video".to_string(),
                "mp3" | "wav" | "m4a" | "flac" => "audio".to_string(),
                "pdf" => "pdf".to_string(),
                _ => "asset".to_string(),
            })
            .unwrap_or_else(|| "asset".to_string()),
    }
}

fn normalize_duplicate_body(body: &str) -> String {
    body.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn stable_text_fingerprint(text: &str) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in text.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

fn relative_display(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .to_string()
}

fn atomic_write(path: &Path, body: &[u8]) -> Result<()> {
    let tmp = temp_sibling_path(path);
    ensure_missing(&tmp)?;
    if let Err(err) = std::fs::write(&tmp, body).and_then(|_| std::fs::rename(&tmp, path)) {
        let _ = std::fs::remove_file(&tmp);
        return Err(err.into());
    }
    Ok(())
}

fn ensure_missing(path: &Path) -> Result<()> {
    if path.exists() {
        bail!("대상 파일이 이미 존재합니다: {}", path.display());
    }
    Ok(())
}

fn copy_without_overwrite(src: &Path, dst: &Path) -> Result<()> {
    ensure_missing(dst)?;
    let tmp = temp_sibling_path(dst);
    ensure_missing(&tmp)?;
    if let Err(err) = std::fs::copy(src, &tmp).and_then(|_| std::fs::rename(&tmp, dst)) {
        let _ = std::fs::remove_file(&tmp);
        return Err(err.into());
    }
    Ok(())
}

fn safe_note_file_stem(title: &str) -> String {
    let stem = title
        .trim()
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '-',
            c if c.is_control() => '-',
            c => c,
        })
        .collect::<String>()
        .trim_matches([' ', '-', '.'])
        .to_string();
    if stem.is_empty() {
        "untitled".to_string()
    } else {
        stem
    }
}

fn unique_note_path(notes_dir: &Path, file_stem: &str) -> PathBuf {
    let mut candidate = notes_dir.join(format!("{file_stem}.md"));
    let mut suffix = 1usize;
    while candidate.exists() {
        candidate = notes_dir.join(format!("{file_stem}-{suffix}.md"));
        suffix += 1;
    }
    candidate
}

fn write_asset_and_sidecar(
    src_asset: &Path,
    asset_path: &Path,
    note_path: &Path,
    note_body: &[u8],
) -> Result<()> {
    ensure_missing(asset_path)?;
    ensure_missing(note_path)?;
    let tmp_asset = temp_sibling_path(asset_path);
    let tmp_note = temp_sibling_path(note_path);
    ensure_missing(&tmp_asset)?;
    ensure_missing(&tmp_note)?;

    if let Err(err) = std::fs::copy(src_asset, &tmp_asset) {
        let _ = std::fs::remove_file(&tmp_asset);
        return Err(err.into());
    }
    if let Err(err) = std::fs::write(&tmp_note, note_body) {
        let _ = std::fs::remove_file(&tmp_asset);
        let _ = std::fs::remove_file(&tmp_note);
        return Err(err.into());
    }
    if let Err(err) = std::fs::rename(&tmp_asset, asset_path) {
        let _ = std::fs::remove_file(&tmp_asset);
        let _ = std::fs::remove_file(&tmp_note);
        return Err(err.into());
    }
    if let Err(err) = std::fs::rename(&tmp_note, note_path) {
        let _ = std::fs::remove_file(asset_path);
        let _ = std::fs::remove_file(&tmp_note);
        return Err(err.into());
    }
    Ok(())
}

fn temp_sibling_path(path: &Path) -> PathBuf {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("tmp");
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    parent.join(format!(".{name}.tmp-{}-{nonce}", std::process::id()))
}

fn note_id_from_notes_path(notes_dir: &Path, path: &Path) -> Result<String> {
    let relative = path.strip_prefix(notes_dir)?;
    let without_ext = relative.with_extension("");
    let parts: Vec<String> = without_ext
        .components()
        .map(|component| component.as_os_str().to_string_lossy().to_string())
        .collect();
    if parts.is_empty() {
        Err(anyhow!("노트 ID를 만들 수 없습니다: {}", path.display()))
    } else {
        Ok(parts.join("/"))
    }
}

/// start에서 위로 올라가며 pack.toml이 있는 디렉터리를 찾는다.
pub fn find_pack_root(start: &Path) -> Result<PathBuf> {
    let mut cur = if start.is_file() {
        start.parent().unwrap_or(start)
    } else {
        start
    };
    loop {
        if cur.join("pack.toml").exists() {
            return Ok(cur.to_path_buf());
        }
        match cur.parent() {
            Some(p) => cur = p,
            None => return Err(anyhow!("pack.toml을 찾지 못함(여기는 팩 안이 아닙니다)")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::embed::FakeEmbedder;
    use tempfile::tempdir;

    #[test]
    fn init_creates_skeleton() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("my-pack");
        Pack::init(&root, "my-pack").unwrap();
        assert!(root.join("pack.toml").exists());
        assert!(root.join("notes").is_dir());
        assert!(root.join("assets").is_dir());
        assert!(root.join("_inbox").is_dir());
    }

    #[test]
    fn find_root_walks_up() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("p");
        Pack::init(&root, "p").unwrap();
        let nested = root.join("notes");
        let found = find_pack_root(&nested).unwrap();
        assert_eq!(found, root);
    }

    #[test]
    fn scan_notes_reads_markdown() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("p");
        Pack::init(&root, "p").unwrap();
        std::fs::write(
            root.join("notes/a.md"),
            "---\ntype: prompt\ntitle: A\n---\n본문 a",
        )
        .unwrap();
        std::fs::write(root.join("notes/b.md"), "본문 b").unwrap();
        let pack = Pack::open(&root).unwrap();
        let mut notes = pack.scan_notes().unwrap();
        notes.sort_by(|x, y| x.id.cmp(&y.id));
        assert_eq!(notes.len(), 2);
        assert_eq!(notes[0].id, "a");
        assert_eq!(notes[0].note_type, "prompt");
        assert_eq!(notes[1].id, "b");
    }

    #[test]
    fn nested_note_ids_use_relative_paths() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("p");
        Pack::init(&root, "p").unwrap();
        std::fs::create_dir_all(root.join("notes/a")).unwrap();
        std::fs::create_dir_all(root.join("notes/b")).unwrap();
        std::fs::write(root.join("notes/a/foo.md"), "A").unwrap();
        std::fs::write(root.join("notes/b/foo.md"), "B").unwrap();
        let pack = Pack::open(&root).unwrap();
        let mut ids: Vec<_> = pack
            .scan_notes()
            .unwrap()
            .into_iter()
            .map(|n| n.id)
            .collect();
        ids.sort();
        assert_eq!(ids, vec!["a/foo", "b/foo"]);
    }

    #[test]
    fn add_file_markdown_and_asset_live_in_core() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("p");
        Pack::init(&root, "p").unwrap();
        let pack = Pack::open(&root).unwrap();

        let src = dir.path().join("hello.md");
        std::fs::write(&src, "본문").unwrap();
        assert!(matches!(
            pack.add_file(&src, "note").unwrap(),
            AddOutcome::Note { .. }
        ));
        assert!(root.join("notes/hello.md").exists());

        let img = dir.path().join("pic.png");
        std::fs::write(&img, [0x89, 0x50, 0x4e, 0x47]).unwrap();
        assert!(matches!(
            pack.add_file(&img, "image").unwrap(),
            AddOutcome::AssetWithSidecar { .. }
        ));
        assert!(root.join("assets/pic.png").exists());
        assert!(root.join("notes/pic.md").exists());
    }

    #[test]
    fn add_file_treats_text_as_note() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("p");
        Pack::init(&root, "p").unwrap();
        let pack = Pack::open(&root).unwrap();

        let src = dir.path().join("memo.txt");
        std::fs::write(&src, "텍스트 메모").unwrap();
        assert!(matches!(
            pack.add_file(&src, "note").unwrap(),
            AddOutcome::Note { .. }
        ));
        assert!(root.join("notes/memo.md").exists());
        assert!(!root.join("assets/memo.txt").exists());
    }

    #[test]
    fn add_file_refuses_to_overwrite_source_of_truth() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("p");
        Pack::init(&root, "p").unwrap();
        let pack = Pack::open(&root).unwrap();

        let src = dir.path().join("hello.md");
        std::fs::write(&src, "새 본문").unwrap();
        std::fs::write(root.join("notes/hello.md"), "기존 본문").unwrap();
        assert!(pack.add_file(&src, "note").is_err());

        let img = dir.path().join("pic.png");
        std::fs::write(&img, [0x89, 0x50, 0x4e, 0x47]).unwrap();
        std::fs::write(root.join("assets/pic.png"), [1, 2, 3]).unwrap();
        assert!(pack.add_file(&img, "image").is_err());
    }

    #[test]
    fn add_file_escapes_sidecar_frontmatter() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("p");
        Pack::init(&root, "p").unwrap();
        let pack = Pack::open(&root).unwrap();

        let img = dir.path().join("a: b.png");
        std::fs::write(&img, [0x89, 0x50, 0x4e, 0x47]).unwrap();
        pack.add_file(&img, "image:still").unwrap();
        let note = note::parse_file(&root.join("notes/a: b.md")).unwrap();
        assert_eq!(note.note_type, "image:still");
        assert_eq!(note.title, "a: b");
        assert_eq!(note.asset.as_deref(), Some("assets/a: b.png"));
    }
    #[test]
    fn build_index_incremental_reports_skips() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("p");
        Pack::init(&root, "p").unwrap();
        std::fs::write(root.join("notes/a.md"), "본문").unwrap();
        let pack = Pack::open(&root).unwrap();

        let first = pack.build_index_incremental().unwrap();
        assert_eq!(first.indexed, 1);
        let second = pack.build_index_incremental().unwrap();
        assert_eq!(second.skipped, 1);
    }

    #[test]
    fn pack_builds_and_searches_chunk_embeddings_with_fake_embedder() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("p");
        Pack::init(&root, "p").unwrap();
        std::fs::write(root.join("notes/lesson.md"), "수업 설계 절차").unwrap();
        std::fs::write(root.join("notes/whale.md"), "바다 고래 관찰").unwrap();
        let pack = Pack::open(&root).unwrap();
        pack.build_index().unwrap();

        let embedder = FakeEmbedder::new(3)
            .with_passage("수업 설계 절차", vec![1.0, 0.0, 0.0])
            .with_passage("바다 고래 관찰", vec![0.0, 1.0, 0.0])
            .with_query("강의 준비", vec![0.95, 0.05, 0.0]);

        let indexed = pack.build_chunk_embeddings_with(&embedder).unwrap();
        assert_eq!(indexed, 2);

        let hits = pack
            .search_vector_chunks_with("강의 준비", 1, &embedder)
            .unwrap();
        assert_eq!(hits[0].note_id, "lesson");
        assert!(!hits[0].text.contains("강의"));
    }

    #[test]
    fn pack_hybrid_search_returns_fused_chunk_cards_with_fake_embedder() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("p");
        Pack::init(&root, "p").unwrap();
        std::fs::write(
            root.join("notes/lesson.md"),
            "---\ntitle: 강의 설계\n---\n수업 설계 절차",
        )
        .unwrap();
        std::fs::write(root.join("notes/whale.md"), "바다 고래 관찰").unwrap();
        let pack = Pack::open(&root).unwrap();
        pack.build_index().unwrap();

        let embedder = FakeEmbedder::new(3)
            .with_passage("수업 설계 절차", vec![1.0, 0.0, 0.0])
            .with_passage("바다 고래 관찰", vec![0.0, 1.0, 0.0])
            .with_query("강의 준비", vec![0.95, 0.05, 0.0]);
        pack.build_chunk_embeddings_with(&embedder).unwrap();

        let hits = pack.search_hybrid_with("강의 준비", 5, &embedder).unwrap();
        assert_eq!(hits[0].note_id, "lesson");
        assert_eq!(hits[0].chunk_id, "lesson#0000");
        assert!(hits[0].snippet.contains("수업 설계"));
        assert!(matches!(
            hits[0].rank_source,
            crate::search::RankSource::Hybrid | crate::search::RankSource::Vector
        ));
    }

    #[test]
    fn related_notes_follow_note_links_by_depth() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("p");
        Pack::init(&root, "p").unwrap();
        std::fs::write(root.join("notes/a.md"), "A [[b]]").unwrap();
        std::fs::write(root.join("notes/b.md"), "---\ntitle: B\n---\nB [[c]]").unwrap();
        std::fs::write(root.join("notes/c.md"), "---\ntitle: C\n---\nC").unwrap();
        let pack = Pack::open(&root).unwrap();

        let related = pack.related_notes("a", 2).unwrap();
        let ids: Vec<_> = related.iter().map(|note| note.id.as_str()).collect();
        assert_eq!(ids, vec!["b", "c"]);
        assert_eq!(related[0].depth, 1);
        assert_eq!(related[1].depth, 2);
    }

    #[test]
    fn timeline_notes_filters_type_and_created_range() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("p");
        Pack::init(&root, "p").unwrap();
        std::fs::write(
            root.join("notes/old.md"),
            "---\ntype: prompt\ntitle: Old\ncreated: 2026-01-01\n---\nold",
        )
        .unwrap();
        std::fs::write(
            root.join("notes/new.md"),
            "---\ntype: prompt\ntitle: New\ncreated: 2026-02-01\n---\nnew",
        )
        .unwrap();
        std::fs::write(
            root.join("notes/img.md"),
            "---\ntype: image\ntitle: Img\ncreated: 2026-03-01\n---\nimg",
        )
        .unwrap();
        let pack = Pack::open(&root).unwrap();

        let timeline = pack
            .timeline_notes(Some("2026-01-15"), Some("2026-12-31"), Some("prompt"), 10)
            .unwrap();
        assert_eq!(timeline.len(), 1);
        assert_eq!(timeline[0].id, "new");
        assert_eq!(timeline[0].created.as_deref(), Some("2026-02-01"));
    }

    #[test]
    fn add_content_note_writes_frontmatter_note_without_overwrite() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("p");
        Pack::init(&root, "p").unwrap();
        let pack = Pack::open(&root).unwrap();

        let first = pack
            .add_content_note("강의 훅", "본문", "prompt", &["lecture".to_string()])
            .unwrap();
        let second = pack
            .add_content_note("강의 훅", "두 번째", "prompt", &[])
            .unwrap();

        assert!(first.ends_with("강의 훅.md"));
        assert!(second.ends_with("강의 훅-1.md"));
        let note = note::parse_file(&first).unwrap();
        assert_eq!(note.title, "강의 훅");
        assert_eq!(note.note_type, "prompt");
        assert_eq!(note.tags, vec!["lecture"]);
    }

    #[test]
    fn process_inbox_imports_files_and_clears_inbox() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("p");
        Pack::init(&root, "p").unwrap();
        std::fs::write(root.join("_inbox/memo.md"), "메모").unwrap();
        std::fs::write(root.join("_inbox/pic.png"), [0x89, 0x50, 0x4e, 0x47]).unwrap();

        let pack = Pack::open(&root).unwrap();
        let report = pack.process_inbox().unwrap();

        assert_eq!(report.processed, 2);
        assert!(root.join("notes/memo.md").exists());
        assert!(root.join("assets/pic.png").exists());
        assert!(root.join("notes/pic.md").exists());
        assert!(!root.join("_inbox/memo.md").exists());
        assert!(!root.join("_inbox/pic.png").exists());
    }

    #[test]
    fn duplicate_notes_groups_notes_with_same_normalized_body() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("p");
        Pack::init(&root, "p").unwrap();
        std::fs::write(
            root.join("notes/a.md"),
            "---\ntitle: A\n---\n같은 본문   여러   공백",
        )
        .unwrap();
        std::fs::write(
            root.join("notes/b.md"),
            "---\ntitle: B\ntags: [copy]\n---\n같은 본문 여러 공백",
        )
        .unwrap();
        std::fs::write(root.join("notes/c.md"), "다른 본문").unwrap();
        let pack = Pack::open(&root).unwrap();

        let groups = pack.duplicate_notes().unwrap();
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].normalized_len, "같은 본문 여러 공백".len());
        let ids: Vec<_> = groups[0]
            .candidates
            .iter()
            .map(|candidate| candidate.note_id.as_str())
            .collect();
        assert_eq!(ids, vec!["a", "b"]);
        assert_eq!(groups[0].candidates[0].path, "notes/a.md");
    }

    #[test]
    fn duplicate_notes_ignores_blank_bodies() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("p");
        Pack::init(&root, "p").unwrap();
        std::fs::write(root.join("notes/a.md"), "---\ntitle: A\n---\n   ").unwrap();
        std::fs::write(root.join("notes/b.md"), "---\ntitle: B\n---\n\n").unwrap();
        let pack = Pack::open(&root).unwrap();

        assert!(pack.duplicate_notes().unwrap().is_empty());
    }

    #[test]
    fn objects_report_pending_asset_enrichment_and_manifest() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("p");
        Pack::init(&root, "p").unwrap();
        let pack = Pack::open(&root).unwrap();
        let img = dir.path().join("pic.png");
        std::fs::write(&img, [0x89, 0x50, 0x4e, 0x47]).unwrap();
        pack.add_file(&img, "image").unwrap();
        std::fs::write(root.join("notes/plain.md"), "그냥 메모").unwrap();
        pack.build_index().unwrap();

        let status = pack.status().unwrap();
        assert_eq!(status.total, 2);
        assert_eq!(status.assets, 1);
        assert_eq!(status.notes, 1);
        assert_eq!(status.indexed, 2);
        assert_eq!(status.pending_enrichment, 1);
        let pic = status
            .objects
            .iter()
            .find(|object| object.note_id == "pic")
            .unwrap();
        assert_eq!(pic.kind, "image");
        assert_eq!(pic.asset_path.as_deref(), Some("assets/pic.png"));
        assert_eq!(pic.enrichment_status, EnrichmentStatus::Pending);

        let manifest = pack.refresh_object_manifest().unwrap();
        let manifest_body = std::fs::read_to_string(manifest).unwrap();
        assert!(manifest_body.contains(r#""note_id":"pic""#));
        assert!(manifest_body.contains(r#""enrichment_status":"pending""#));
    }

    #[test]
    fn update_enrichment_preserves_human_content_and_becomes_searchable() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("p");
        Pack::init(&root, "p").unwrap();
        let pack = Pack::open(&root).unwrap();
        let img = dir.path().join("board.png");
        std::fs::write(&img, [0x89, 0x50, 0x4e, 0x47]).unwrap();
        pack.add_file(&img, "image").unwrap();
        std::fs::write(
            root.join("notes/board.md"),
            "---
type: image
title: Board
asset: assets/board.png
tags: []
---
사람이 적은 원본 메모
",
        )
        .unwrap();

        let patch = EnrichmentPatch {
            caption: Some("화이트보드에 로컬 온톨로지 그래프가 있다".to_string()),
            tags: vec!["ontology".to_string(), "whiteboard".to_string()],
            transcript: Some("[00:00] 로컬 지식팩 설명".to_string()),
            provider: Some("codex".to_string()),
            model: Some("test-double".to_string()),
            ..EnrichmentPatch::default()
        };
        let note_path = pack.update_enrichment("board", &patch).unwrap();
        let once = std::fs::read_to_string(&note_path).unwrap();
        assert!(once.contains("사람이 적은 원본 메모"));
        assert!(once.contains("## AI Caption"));
        assert!(once.contains("화이트보드에 로컬 온톨로지 그래프"));
        assert_eq!(once.matches(enrichment::ENRICHMENT_START).count(), 1);
        pack.update_enrichment("board", &patch).unwrap();
        let twice = std::fs::read_to_string(&note_path).unwrap();
        assert_eq!(twice.matches(enrichment::ENRICHMENT_START).count(), 1);

        let status = pack.status().unwrap();
        let board = status
            .objects
            .iter()
            .find(|object| object.note_id == "board")
            .unwrap();
        assert_eq!(board.enrichment_status, EnrichmentStatus::Done);

        pack.build_index().unwrap();
        let hits = pack.search_keyword_chunks("온톨로지", 5).unwrap();
        assert_eq!(hits[0].note_id, "board");
    }
}
