use anyhow::{bail, Result};
use pack_core::pack::Pack;
use pack_core::search::{RankSource, SearchFilters as CoreSearchFilters, SearchHit};
use serde::Serialize;
use std::time::Instant;

const MAX_SEARCH_K: usize = 100;

#[derive(Debug, Serialize, PartialEq)]
pub struct SearchResponse {
    pub query: String,
    pub mode: String,
    pub source: String,
    pub hits: Vec<SearchCard>,
    pub elapsed_ms: u64,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct SearchFilters<'a> {
    pub note_type: Option<&'a str>,
    pub tag: Option<&'a str>,
    pub from: Option<&'a str>,
    pub to: Option<&'a str>,
    pub mode: Option<&'a str>,
    pub k: usize,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct SearchCard {
    pub note_id: String,
    pub chunk_id: String,
    pub title: String,
    pub note_type: String,
    pub snippet: String,
    pub path: String,
    pub score: f64,
    pub rank_source: String,
    pub asset: Option<String>,
    pub asset_url: Option<String>,
    pub media_kind: Option<String>,
    pub mime: Option<String>,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct AskResponse {
    pub question: String,
    pub answer_mode: String,
    pub instruction: String,
    pub context_blocks: Vec<SearchCard>,
    pub elapsed_ms: u64,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct NoteDetail {
    pub id: String,
    pub title: String,
    pub note_type: String,
    pub tags: Vec<String>,
    pub created: Option<String>,
    pub asset: Option<String>,
    pub asset_url: Option<String>,
    pub media_kind: Option<String>,
    pub mime: Option<String>,
    pub related: Vec<String>,
    pub body: String,
    pub path: String,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct RelatedResponse {
    pub note_id: String,
    pub related: Vec<RelatedCard>,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct RelatedCard {
    pub id: String,
    pub title: String,
    pub note_type: String,
    pub path: String,
    pub depth: usize,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct TimelineResponse {
    pub notes: Vec<TimelineCard>,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct TimelineCard {
    pub id: String,
    pub title: String,
    pub note_type: String,
    pub path: String,
    pub created: Option<String>,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct GraphResponse {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct GraphNode {
    pub id: String,
    pub title: String,
    pub note_type: String,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct GraphEdge {
    pub from: String,
    pub to: String,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct FacetsResponse {
    pub types: Vec<String>,
    pub tags: Vec<String>,
    pub created_min: Option<String>,
    pub created_max: Option<String>,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct GalleryResponse {
    pub items: Vec<GalleryItem>,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct DashboardResponse {
    pub facets: FacetsResponse,
    pub gallery: GalleryResponse,
    pub timeline: TimelineResponse,
    pub graph: GraphResponse,
    pub elapsed_ms: u64,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct DashboardFilters<'a> {
    pub note_type: Option<&'a str>,
    pub from: Option<&'a str>,
    pub to: Option<&'a str>,
    pub gallery_k: usize,
    pub timeline_k: usize,
    pub graph_limit: usize,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct GalleryItem {
    pub id: String,
    pub title: String,
    pub note_type: String,
    pub tags: Vec<String>,
    pub asset: Option<String>,
    pub asset_url: Option<String>,
    pub media_kind: Option<String>,
    pub mime: Option<String>,
    pub path: String,
    pub caption: String,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct CapabilitiesResponse {
    pub default_search_mode: String,
    pub semantic_search: bool,
    pub embedding_model: String,
    pub embedding_dim: usize,
    pub search_modes: Vec<SearchModeCapability>,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct SearchModeCapability {
    pub mode: String,
    pub available: bool,
    pub source: String,
    pub reason: Option<String>,
}

pub fn search(
    pack: &Pack,
    query: &str,
    note_type: Option<&str>,
    k: usize,
) -> Result<SearchResponse> {
    search_with_filters(
        pack,
        query,
        SearchFilters {
            note_type,
            k,
            ..SearchFilters::default()
        },
    )
}

pub fn search_with_filters(
    pack: &Pack,
    query: &str,
    filters: SearchFilters<'_>,
) -> Result<SearchResponse> {
    let started = Instant::now();
    validate_search_mode(filters.mode)?;
    let k = filters.k.clamp(1, MAX_SEARCH_K);
    let hits = pack.search_keyword_chunks_filtered(
        query,
        k,
        CoreSearchFilters {
            note_type: filters.note_type,
            tag: filters.tag,
            from: filters.from,
            to: filters.to,
        },
    )?;
    Ok(SearchResponse {
        query: query.to_string(),
        mode: "keyword".to_string(),
        source: "sqlite_fts".to_string(),
        hits: hits.into_iter().map(search_card).collect(),
        elapsed_ms: elapsed_ms_since(started),
    })
}

pub fn ask(pack: &Pack, question: &str, k: usize) -> Result<AskResponse> {
    let started = Instant::now();
    let k = k.clamp(1, MAX_SEARCH_K);
    let context_blocks = pack
        .search_keyword_chunks(question, k)?
        .into_iter()
        .map(search_card)
        .collect();
    Ok(AskResponse {
        question: question.to_string(),
        answer_mode: "external_llm_required".to_string(),
        instruction:
            "Use context_blocks to synthesize an answer with citations outside deterministic pack-core."
                .to_string(),
        context_blocks,
        elapsed_ms: elapsed_ms_since(started),
    })
}

pub fn note(pack: &Pack, id: &str) -> Result<NoteDetail> {
    let Some(note) = pack.note_by_id_or_scan(id)? else {
        bail!("note not found: {id}");
    };
    let media = media_metadata(note.asset.as_deref());
    Ok(NoteDetail {
        id: note.id,
        title: note.title,
        note_type: note.note_type,
        tags: note.tags,
        created: note.created,
        asset: note.asset,
        asset_url: media.asset_url,
        media_kind: media.media_kind,
        mime: media.mime,
        related: note.related,
        body: note.body,
        path: note.path.to_string_lossy().to_string(),
    })
}

pub fn related(pack: &Pack, note_id: &str, depth: usize) -> Result<RelatedResponse> {
    Ok(RelatedResponse {
        note_id: note_id.to_string(),
        related: pack
            .related_notes_or_scan(note_id, depth)?
            .into_iter()
            .map(|note| RelatedCard {
                id: note.id,
                title: note.title,
                note_type: note.note_type,
                path: note.path.to_string_lossy().to_string(),
                depth: note.depth,
            })
            .collect(),
    })
}

pub fn timeline(
    pack: &Pack,
    from: Option<&str>,
    to: Option<&str>,
    note_type: Option<&str>,
    k: usize,
) -> Result<TimelineResponse> {
    Ok(TimelineResponse {
        notes: pack
            .timeline_notes_or_scan(from, to, note_type, k)?
            .into_iter()
            .map(|note| TimelineCard {
                id: note.id,
                title: note.title,
                note_type: note.note_type,
                path: note.path.to_string_lossy().to_string(),
                created: note.created,
            })
            .collect(),
    })
}

pub fn graph(pack: &Pack, note_type: Option<&str>, limit: usize) -> Result<GraphResponse> {
    let graph = pack.graph_or_scan(note_type, limit)?;
    let nodes = graph
        .nodes
        .into_iter()
        .map(|node| GraphNode {
            id: node.id,
            title: node.title,
            note_type: node.note_type,
        })
        .collect();
    let edges = graph
        .edges
        .into_iter()
        .map(|(from, to)| GraphEdge { from, to })
        .collect();
    Ok(GraphResponse { nodes, edges })
}

pub fn facets(pack: &Pack) -> Result<FacetsResponse> {
    let facets = pack.facets_or_scan()?;
    Ok(FacetsResponse {
        types: facets.types,
        tags: facets.tags,
        created_min: facets.created_min,
        created_max: facets.created_max,
    })
}

pub fn gallery(pack: &Pack, note_type: Option<&str>, k: usize) -> Result<GalleryResponse> {
    let mut items = Vec::new();
    for note in pack.gallery_notes_or_scan(note_type, k)? {
        let media = media_metadata(note.asset.as_deref());
        items.push(GalleryItem {
            id: note.id,
            title: note.title,
            note_type: note.note_type,
            tags: note.tags,
            asset: note.asset,
            asset_url: media.asset_url,
            media_kind: media.media_kind,
            mime: media.mime,
            path: note.path.to_string_lossy().to_string(),
            caption: note.body.trim().to_string(),
        });
    }
    Ok(GalleryResponse { items })
}

pub fn dashboard(pack: &Pack, filters: DashboardFilters<'_>) -> Result<DashboardResponse> {
    let started = Instant::now();
    Ok(DashboardResponse {
        facets: facets(pack)?,
        gallery: gallery(pack, filters.note_type, filters.gallery_k)?,
        timeline: timeline(
            pack,
            filters.from,
            filters.to,
            filters.note_type,
            filters.timeline_k,
        )?,
        graph: graph(pack, filters.note_type, filters.graph_limit)?,
        elapsed_ms: elapsed_ms_since(started),
    })
}

pub fn capabilities(pack: &Pack) -> CapabilitiesResponse {
    let semantic_reason =
        "pack-server is keyword-only in this build; use the CLI real-embed path for vector/hybrid search until server capabilities are expanded"
            .to_string();
    CapabilitiesResponse {
        default_search_mode: "keyword".to_string(),
        semantic_search: false,
        embedding_model: pack.config.embed_model.clone(),
        embedding_dim: pack.config.embed_dim,
        search_modes: vec![
            SearchModeCapability {
                mode: "keyword".to_string(),
                available: true,
                source: "sqlite_fts".to_string(),
                reason: None,
            },
            SearchModeCapability {
                mode: "vector".to_string(),
                available: false,
                source: "sqlite_vec".to_string(),
                reason: Some(semantic_reason.clone()),
            },
            SearchModeCapability {
                mode: "hybrid".to_string(),
                available: false,
                source: "hybrid".to_string(),
                reason: Some(semantic_reason),
            },
        ],
    }
}

fn search_card(hit: SearchHit) -> SearchCard {
    let media = media_metadata(hit.asset.as_deref());
    SearchCard {
        note_id: hit.note_id,
        chunk_id: hit.chunk_id,
        title: hit.title,
        note_type: hit.note_type,
        snippet: hit.snippet,
        path: hit.path,
        score: hit.score,
        rank_source: rank_source_label(hit.rank_source).to_string(),
        asset: hit.asset,
        asset_url: media.asset_url,
        media_kind: media.media_kind,
        mime: media.mime,
    }
}

struct MediaMetadata {
    asset_url: Option<String>,
    media_kind: Option<String>,
    mime: Option<String>,
}

fn media_metadata(asset: Option<&str>) -> MediaMetadata {
    let Some(asset) = asset else {
        return MediaMetadata {
            asset_url: None,
            media_kind: None,
            mime: None,
        };
    };
    let mime = mime_for_asset(asset).to_string();
    MediaMetadata {
        asset_url: asset_url(asset),
        media_kind: Some(media_kind_for_mime(&mime).to_string()),
        mime: Some(mime),
    }
}

pub fn mime_for_asset(asset: &str) -> &'static str {
    match asset
        .rsplit('.')
        .next()
        .unwrap_or("")
        .to_ascii_lowercase()
        .as_str()
    {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        "mp4" | "m4v" => "video/mp4",
        "webm" => "video/webm",
        "mov" | "qt" => "video/quicktime",
        "mp3" => "audio/mpeg",
        "wav" => "audio/wav",
        "m4a" => "audio/mp4",
        "pdf" => "application/pdf",
        _ => "application/octet-stream",
    }
}

fn media_kind_for_mime(mime: &str) -> &'static str {
    if mime.starts_with("image/") {
        "image"
    } else if mime.starts_with("video/") {
        "video"
    } else if mime.starts_with("audio/") {
        "audio"
    } else if mime == "application/octet-stream" {
        "unknown"
    } else {
        "file"
    }
}

fn asset_url(asset: &str) -> Option<String> {
    let relative = asset.strip_prefix("assets/").unwrap_or(asset);
    if relative.is_empty() || relative.starts_with('/') || relative.contains("..") {
        return None;
    }
    Some(format!("/assets/{}", percent_encode_path(relative)))
}

fn validate_search_mode(mode: Option<&str>) -> Result<()> {
    match mode.unwrap_or("keyword") {
        "keyword" => Ok(()),
        "vector" | "hybrid" => bail!(
            "search mode unavailable in pack-server: vector/hybrid require a future real-embed server capability; use pack search --mode vector|hybrid with a real-embed CLI build for now"
        ),
        other => bail!("unknown search mode: {other}"),
    }
}

fn elapsed_ms_since(started: Instant) -> u64 {
    started.elapsed().as_millis().try_into().unwrap_or(u64::MAX)
}

fn percent_encode_path(path: &str) -> String {
    path.split('/')
        .map(percent_encode_segment)
        .collect::<Vec<_>>()
        .join("/")
}

fn percent_encode_segment(segment: &str) -> String {
    let mut out = String::new();
    for byte in segment.as_bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(*byte as char)
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

fn rank_source_label(source: RankSource) -> &'static str {
    match source {
        RankSource::Keyword => "keyword",
        RankSource::Vector => "vector",
        RankSource::Hybrid => "hybrid",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pack_core::pack::Pack;
    use tempfile::tempdir;

    #[test]
    fn search_api_returns_source_cards() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("p");
        Pack::init(&root, "p").unwrap();
        std::fs::write(
            root.join("notes/hook.md"),
            "---\ntype: prompt\ntitle: 썸네일 훅\ntags: [youtube]\ncreated: 2026-02-01\n---\n클릭을 부르는 훅 카피.",
        )
        .unwrap();
        let pack = Pack::open(&root).unwrap();
        pack.build_index().unwrap();

        let response = search(&pack, "훅", None, 10).unwrap();
        assert_eq!(response.query, "훅");
        assert_eq!(response.hits[0].note_id, "hook");
        assert_eq!(response.hits[0].chunk_id, "hook#0000");
        assert_eq!(response.hits[0].rank_source, "keyword");
        assert_eq!(response.hits[0].asset_url, None);
    }

    #[test]
    fn search_filters_apply_before_final_limit() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("p");
        Pack::init(&root, "p").unwrap();
        for i in 0..101 {
            std::fs::write(
                root.join("notes").join(format!("distractor-{i:03}.md")),
                format!(
                    "---
type: note
title: Distractor {i:03}
---
common term {i}",
                ),
            )
            .unwrap();
        }
        std::fs::write(
            root.join("notes/z.md"),
            "---
type: prompt
title: Z
tags: [needle]
created: 2026-05-23
---
common term",
        )
        .unwrap();
        let pack = Pack::open(&root).unwrap();
        pack.build_index().unwrap();

        let unfiltered = search(&pack, "common", None, 1).unwrap();
        assert_eq!(unfiltered.hits.len(), 1);

        let filtered = search_with_filters(
            &pack,
            "common",
            SearchFilters {
                note_type: Some("prompt"),
                tag: Some("needle"),
                from: Some("2026-01-01"),
                to: Some("2026-12-31"),
                k: 1,
                ..SearchFilters::default()
            },
        )
        .unwrap();
        assert_eq!(filtered.hits.len(), 1);
        assert_eq!(filtered.hits[0].note_id, "z");
    }

    #[test]
    fn note_api_returns_note_detail() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("p");
        Pack::init(&root, "p").unwrap();
        std::fs::write(
            root.join("notes/a.md"),
            "---\ntype: note\ntitle: A\ntags: [x]\ncreated: 2026-01-01\n---\n본문 [[b]]",
        )
        .unwrap();
        let pack = Pack::open(&root).unwrap();

        let note = note(&pack, "a").unwrap();
        assert_eq!(note.id, "a");
        assert_eq!(note.title, "A");
        assert_eq!(note.tags, vec!["x"]);
        assert_eq!(note.related, vec!["b"]);
        assert!(note.body.contains("본문"));
    }

    #[test]
    fn graph_api_returns_nodes_and_edges() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("p");
        Pack::init(&root, "p").unwrap();
        std::fs::write(root.join("notes/a.md"), "A [[b]]").unwrap();
        std::fs::write(root.join("notes/b.md"), "B").unwrap();
        let pack = Pack::open(&root).unwrap();

        let graph = graph(&pack, None, 50).unwrap();
        assert_eq!(graph.nodes.len(), 2);
        assert_eq!(graph.edges[0].from, "a");
        assert_eq!(graph.edges[0].to, "b");
    }
    #[test]
    fn ask_api_returns_context_blocks() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("p");
        Pack::init(&root, "p").unwrap();
        std::fs::write(
            root.join("notes/hook.md"),
            "---\ntype: prompt\ntitle: 썸네일 훅\n---\n클릭을 부르는 훅 카피.",
        )
        .unwrap();
        let pack = Pack::open(&root).unwrap();
        pack.build_index().unwrap();

        let response = ask(&pack, "훅 자료?", 3).unwrap();
        assert_eq!(response.question, "훅 자료?");
        assert_eq!(response.answer_mode, "external_llm_required");
        assert_eq!(response.context_blocks[0].note_id, "hook");
    }

    #[test]
    fn facets_api_returns_types_tags_and_date_range() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("p");
        Pack::init(&root, "p").unwrap();
        std::fs::write(
            root.join("notes/a.md"),
            "---\ntype: prompt\ntitle: A\ntags: [youtube, hook]\ncreated: 2026-01-01\n---\nA",
        )
        .unwrap();
        std::fs::write(
            root.join("notes/b.md"),
            "---\ntype: image\ntitle: B\ntags: [gallery]\ncreated: 2026-03-01\n---\nB",
        )
        .unwrap();
        let pack = Pack::open(&root).unwrap();

        let response = facets(&pack).unwrap();
        assert_eq!(response.types, vec!["image", "prompt"]);
        assert_eq!(response.tags, vec!["gallery", "hook", "youtube"]);
        assert_eq!(response.created_min.as_deref(), Some("2026-01-01"));
        assert_eq!(response.created_max.as_deref(), Some("2026-03-01"));
    }

    #[test]
    fn gallery_api_returns_asset_notes() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("p");
        Pack::init(&root, "p").unwrap();
        std::fs::write(
            root.join("notes/pic.md"),
            "---\ntype: image\ntitle: Pic\nasset: assets/pic.png\ntags: [gallery]\n---\n캡션",
        )
        .unwrap();
        std::fs::write(root.join("notes/plain.md"), "plain").unwrap();
        let pack = Pack::open(&root).unwrap();

        let response = gallery(&pack, None, 10).unwrap();
        assert_eq!(response.items.len(), 1);
        assert_eq!(response.items[0].id, "pic");
        assert_eq!(response.items[0].asset.as_deref(), Some("assets/pic.png"));
        assert_eq!(
            response.items[0].asset_url.as_deref(),
            Some("/assets/pic.png")
        );
        assert_eq!(response.items[0].media_kind.as_deref(), Some("image"));
        assert_eq!(response.items[0].mime.as_deref(), Some("image/png"));
    }

    #[test]
    fn note_api_returns_media_metadata() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("p");
        Pack::init(&root, "p").unwrap();
        std::fs::write(
            root.join("notes/clip.md"),
            "---
type: video
title: Demo Clip
asset: assets/demo clip.mp4
---
영상 캡션",
        )
        .unwrap();
        let pack = Pack::open(&root).unwrap();

        let response = note(&pack, "clip").unwrap();
        assert_eq!(response.asset.as_deref(), Some("assets/demo clip.mp4"));
        assert_eq!(
            response.asset_url.as_deref(),
            Some("/assets/demo%20clip.mp4")
        );
        assert_eq!(response.media_kind.as_deref(), Some("video"));
        assert_eq!(response.mime.as_deref(), Some("video/mp4"));
    }

    #[test]
    fn search_api_returns_media_metadata_for_asset_hits() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("p");
        Pack::init(&root, "p").unwrap();
        std::fs::write(
            root.join("notes/pic.md"),
            "---
type: image
title: Pic
asset: assets/pic.webp
---
검색 가능한 이미지 캡션",
        )
        .unwrap();
        let pack = Pack::open(&root).unwrap();
        pack.build_index().unwrap();

        let response = search(&pack, "이미지", None, 10).unwrap();
        assert_eq!(response.hits[0].asset.as_deref(), Some("assets/pic.webp"));
        assert_eq!(
            response.hits[0].asset_url.as_deref(),
            Some("/assets/pic.webp")
        );
        assert_eq!(response.hits[0].media_kind.as_deref(), Some("image"));
        assert_eq!(response.hits[0].mime.as_deref(), Some("image/webp"));
    }
    #[test]
    fn note_api_reads_from_index_after_source_file_is_removed() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("p");
        Pack::init(&root, "p").unwrap();
        let note_path = root.join("notes/pic.md");
        std::fs::write(
            &note_path,
            "---
type: image
title: Indexed Pic
asset: assets/pic.png
tags: [gallery]
---
인덱스에 남은 캡션",
        )
        .unwrap();
        let pack = Pack::open(&root).unwrap();
        pack.build_index().unwrap();
        std::fs::remove_file(note_path).unwrap();

        let response = note(&pack, "pic").unwrap();
        assert_eq!(response.title, "Indexed Pic");
        assert_eq!(response.asset_url.as_deref(), Some("/assets/pic.png"));
    }

    #[test]
    fn gallery_api_reads_from_index_after_source_file_is_removed() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("p");
        Pack::init(&root, "p").unwrap();
        let note_path = root.join("notes/pic.md");
        std::fs::write(
            &note_path,
            "---
type: image
title: Indexed Pic
asset: assets/pic.png
tags: [gallery]
---
인덱스에 남은 캡션",
        )
        .unwrap();
        let pack = Pack::open(&root).unwrap();
        pack.build_index().unwrap();
        std::fs::remove_file(note_path).unwrap();

        let response = gallery(&pack, None, 10).unwrap();
        assert_eq!(response.items.len(), 1);
        assert_eq!(response.items[0].title, "Indexed Pic");
        assert_eq!(
            response.items[0].asset_url.as_deref(),
            Some("/assets/pic.png")
        );
    }
}
