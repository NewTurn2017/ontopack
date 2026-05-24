use anyhow::{bail, Result};
use pack_core::embed::Embedder;
use pack_core::enrichment::{ENRICHMENT_END, ENRICHMENT_START};
use pack_core::pack::Pack;
use pack_core::search::{RankSource, SearchFilters as CoreSearchFilters, SearchHit, SearchMode};
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
    pub remote_url: Option<String>,
    pub thumbnail_url: Option<String>,
    pub asset_url: Option<String>,
    pub media_kind: Option<String>,
    pub mime: Option<String>,
    pub media_citation: Option<MediaCitation>,
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
    pub remote_url: Option<String>,
    pub thumbnail_url: Option<String>,
    pub asset_url: Option<String>,
    pub media_kind: Option<String>,
    pub mime: Option<String>,
    pub related: Vec<String>,
    pub keyframes: Vec<MediaKeyframe>,
    pub body: String,
    pub path: String,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct MediaKeyframe {
    pub time: String,
    pub text: String,
    pub asset: Option<String>,
    pub asset_url: Option<String>,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct MediaCitation {
    pub time: String,
    pub seconds: u64,
    pub asset_url: String,
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
    pub remote_url: Option<String>,
    pub thumbnail_url: Option<String>,
    pub asset_url: Option<String>,
    pub media_kind: Option<String>,
    pub mime: Option<String>,
    pub path: String,
    pub caption: String,
    pub keyframes: Vec<MediaKeyframe>,
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
    search_with_filters_and_embedder(pack, query, filters, None)
}

pub fn search_with_filters_and_embedder(
    pack: &Pack,
    query: &str,
    filters: SearchFilters<'_>,
    semantic_embedder: Option<&dyn Embedder>,
) -> Result<SearchResponse> {
    let started = Instant::now();
    let mode = resolve_search_mode(filters.mode, semantic_embedder.is_some())?;
    let k = filters.k.clamp(1, MAX_SEARCH_K);
    let (source, hits) = match mode {
        SearchMode::Keyword => (
            "sqlite_fts",
            pack.search_keyword_chunks_filtered(
                query,
                k,
                CoreSearchFilters {
                    note_type: filters.note_type,
                    tag: filters.tag,
                    from: filters.from,
                    to: filters.to,
                },
            )?,
        ),
        SearchMode::Vector => {
            let embedder = semantic_embedder.expect("semantic mode availability checked");
            (
                "sqlite_vec",
                filter_semantic_hits(
                    pack,
                    pack.search_vector_chunk_hits_with(query, MAX_SEARCH_K, embedder)?,
                    filters,
                    k,
                )?,
            )
        }
        SearchMode::Hybrid => {
            let embedder = semantic_embedder.expect("semantic mode availability checked");
            (
                "hybrid_rrf",
                filter_semantic_hits(
                    pack,
                    pack.search_hybrid_with(query, MAX_SEARCH_K, embedder)?,
                    filters,
                    k,
                )?,
            )
        }
    };
    Ok(SearchResponse {
        query: query.to_string(),
        mode: search_mode_label(mode).to_string(),
        source: source.to_string(),
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
    let media = media_metadata(
        note.asset.as_deref(),
        note.thumbnail_url.as_deref(),
        note.media_kind.as_deref(),
        note.mime.as_deref(),
    );
    let keyframes = parse_keyframes(&note.body);
    Ok(NoteDetail {
        id: note.id,
        title: note.title,
        note_type: note.note_type,
        tags: note.tags,
        created: note.created,
        asset: note.asset,
        remote_url: note.remote_url,
        thumbnail_url: note.thumbnail_url,
        asset_url: media.asset_url,
        media_kind: media.media_kind,
        mime: media.mime,
        related: note.related,
        keyframes,
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
        let media = media_metadata(
            note.asset.as_deref(),
            note.thumbnail_url.as_deref(),
            note.media_kind.as_deref(),
            note.mime.as_deref(),
        );
        items.push(GalleryItem {
            id: note.id,
            title: note.title,
            note_type: note.note_type,
            tags: note.tags,
            asset: note.asset,
            remote_url: note.remote_url,
            thumbnail_url: note.thumbnail_url,
            asset_url: media.asset_url,
            media_kind: media.media_kind,
            mime: media.mime,
            path: note.path.to_string_lossy().to_string(),
            keyframes: parse_keyframes(&note.body),
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
    capabilities_with_semantic(pack, false)
}

pub fn capabilities_with_semantic(pack: &Pack, semantic_available: bool) -> CapabilitiesResponse {
    let semantic_reason = (!semantic_available).then(|| {
        "pack-server semantic search is disabled; start `pack serve --semantic` or `pack open --semantic` from a real-embed build after running `pack embed`"
            .to_string()
    });
    CapabilitiesResponse {
        default_search_mode: "keyword".to_string(),
        semantic_search: semantic_available,
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
                available: semantic_available,
                source: "sqlite_vec".to_string(),
                reason: semantic_reason.clone(),
            },
            SearchModeCapability {
                mode: "hybrid".to_string(),
                available: semantic_available,
                source: "hybrid_rrf".to_string(),
                reason: semantic_reason,
            },
        ],
    }
}

fn search_card(hit: SearchHit) -> SearchCard {
    let media = media_metadata(
        hit.asset.as_deref(),
        hit.thumbnail_url.as_deref(),
        hit.media_kind.as_deref(),
        hit.mime.as_deref(),
    );
    let media_citation = media_citation_for_hit(
        media.media_kind.as_deref(),
        media.asset_url.as_deref(),
        &hit.snippet,
    );
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
        remote_url: hit.remote_url,
        thumbnail_url: hit.thumbnail_url,
        asset_url: media.asset_url,
        media_kind: media.media_kind,
        mime: media.mime,
        media_citation,
    }
}

struct MediaMetadata {
    asset_url: Option<String>,
    media_kind: Option<String>,
    mime: Option<String>,
}

fn media_metadata(
    asset: Option<&str>,
    thumbnail_url: Option<&str>,
    media_kind: Option<&str>,
    mime: Option<&str>,
) -> MediaMetadata {
    if let Some(asset) = asset {
        let mime = mime.unwrap_or_else(|| mime_for_asset(asset)).to_string();
        return MediaMetadata {
            asset_url: asset_url(asset),
            media_kind: Some(
                media_kind
                    .unwrap_or_else(|| media_kind_for_mime(&mime))
                    .to_string(),
            ),
            mime: Some(mime),
        };
    }
    if let Some(thumbnail_url) = thumbnail_url {
        let mime = mime
            .unwrap_or_else(|| mime_for_url(thumbnail_url))
            .to_string();
        return MediaMetadata {
            asset_url: Some(thumbnail_url.to_string()),
            media_kind: Some(
                media_kind
                    .unwrap_or_else(|| media_kind_for_mime(&mime))
                    .to_string(),
            ),
            mime: Some(mime),
        };
    }
    MediaMetadata {
        asset_url: None,
        media_kind: media_kind.map(str::to_string),
        mime: mime.map(str::to_string),
    }
}

fn media_citation_for_hit(
    media_kind: Option<&str>,
    asset_url: Option<&str>,
    snippet: &str,
) -> Option<MediaCitation> {
    if !matches!(media_kind, Some("video" | "audio")) {
        return None;
    }
    let asset_url = asset_url?;
    let (time, seconds) = parse_first_timecode(snippet)?;
    Some(MediaCitation {
        time,
        seconds,
        asset_url: format!("{asset_url}#t={seconds}"),
    })
}

fn parse_first_timecode(text: &str) -> Option<(String, u64)> {
    let bytes = text.as_bytes();
    for start in 0..bytes.len() {
        if !bytes[start].is_ascii_digit() {
            continue;
        }
        if start > 0 && (bytes[start - 1].is_ascii_digit() || bytes[start - 1] == b':') {
            continue;
        }
        let mut pos = start;
        let mut groups = Vec::new();
        loop {
            let number_start = pos;
            while pos < bytes.len() && bytes[pos].is_ascii_digit() {
                pos += 1;
            }
            let digits = pos.saturating_sub(number_start);
            if digits == 0 || digits > 2 {
                break;
            }
            let value = std::str::from_utf8(&bytes[number_start..pos])
                .ok()?
                .parse::<u64>()
                .ok()?;
            groups.push(value);
            if pos < bytes.len() && bytes[pos] == b':' && groups.len() < 3 {
                pos += 1;
                continue;
            }
            break;
        }
        if !matches!(groups.len(), 2 | 3) {
            continue;
        }
        if pos < bytes.len() && (bytes[pos].is_ascii_digit() || bytes[pos] == b':') {
            continue;
        }
        let (time, seconds) = match groups.as_slice() {
            [minutes, seconds] if *seconds < 60 => {
                (format!("{minutes:02}:{seconds:02}"), minutes * 60 + seconds)
            }
            [hours, minutes, seconds] if *minutes < 60 && *seconds < 60 => (
                format!("{hours:02}:{minutes:02}:{seconds:02}"),
                hours * 3600 + minutes * 60 + seconds,
            ),
            _ => continue,
        };
        return Some((time, seconds));
    }
    None
}

fn parse_keyframes(body: &str) -> Vec<MediaKeyframe> {
    let block = body
        .find(ENRICHMENT_START)
        .and_then(|start| {
            let after_start = start + ENRICHMENT_START.len();
            body[after_start..]
                .find(ENRICHMENT_END)
                .map(|end_rel| &body[after_start..after_start + end_rel])
        })
        .unwrap_or(body);

    let mut in_keyframes = false;
    let mut frames = Vec::new();
    for line in block.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("## ") {
            in_keyframes = trimmed == "## Keyframes";
            continue;
        }
        if !in_keyframes {
            continue;
        }
        let Some(frame) = parse_keyframe_line(trimmed) else {
            continue;
        };
        frames.push(frame);
    }
    frames
}

fn parse_keyframe_line(line: &str) -> Option<MediaKeyframe> {
    let line = line.strip_prefix("- [")?;
    let (time, rest) = line.split_once("] ")?;
    let (text, asset) = if let Some((text, asset_part)) = rest.rsplit_once(" — `") {
        let asset = asset_part.strip_suffix('`')?.to_string();
        (text.to_string(), Some(asset))
    } else {
        (rest.to_string(), None)
    };
    let asset_url = asset.as_deref().and_then(asset_url);
    Some(MediaKeyframe {
        time: time.to_string(),
        text,
        asset,
        asset_url,
    })
}

pub fn mime_for_asset(asset: &str) -> &'static str {
    mime_for_extension(asset.rsplit('.').next().unwrap_or(""))
}

fn mime_for_url(url: &str) -> &'static str {
    let path = url.split(['?', '#']).next().unwrap_or(url);
    mime_for_extension(path.rsplit('.').next().unwrap_or(""))
}

fn mime_for_extension(ext: &str) -> &'static str {
    match ext.to_ascii_lowercase().as_str() {
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

fn resolve_search_mode(mode: Option<&str>, semantic_available: bool) -> Result<SearchMode> {
    match mode.unwrap_or("keyword") {
        "keyword" => Ok(SearchMode::Keyword),
        "vector" if semantic_available => Ok(SearchMode::Vector),
        "hybrid" if semantic_available => Ok(SearchMode::Hybrid),
        "vector" | "hybrid" => bail!(
            "search mode unavailable in pack-server: vector/hybrid require `pack serve --semantic` or `pack open --semantic` from a real-embed build after running `pack embed`"
        ),
        other => bail!("unknown search mode: {other}"),
    }
}

fn search_mode_label(mode: SearchMode) -> &'static str {
    match mode {
        SearchMode::Keyword => "keyword",
        SearchMode::Vector => "vector",
        SearchMode::Hybrid => "hybrid",
    }
}

fn filter_semantic_hits(
    pack: &Pack,
    hits: Vec<SearchHit>,
    filters: SearchFilters<'_>,
    k: usize,
) -> Result<Vec<SearchHit>> {
    let mut out = Vec::new();
    for hit in hits {
        if semantic_hit_matches_filters(pack, &hit, filters)? {
            out.push(hit);
            if out.len() >= k {
                break;
            }
        }
    }
    Ok(out)
}

fn semantic_hit_matches_filters(
    pack: &Pack,
    hit: &SearchHit,
    filters: SearchFilters<'_>,
) -> Result<bool> {
    if filters.note_type.is_none()
        && filters.tag.is_none()
        && filters.from.is_none()
        && filters.to.is_none()
    {
        return Ok(true);
    }
    let Some(note) = pack.note_by_id_or_scan(&hit.note_id)? else {
        return Ok(false);
    };
    if filters
        .note_type
        .is_some_and(|note_type| note.note_type != note_type)
    {
        return Ok(false);
    }
    if filters
        .tag
        .is_some_and(|tag| !note.tags.iter().any(|candidate| candidate == tag))
    {
        return Ok(false);
    }
    if filters
        .from
        .is_some_and(|from| note.created.as_deref().is_none_or(|created| created < from))
    {
        return Ok(false);
    }
    if filters
        .to
        .is_some_and(|to| note.created.as_deref().is_none_or(|created| created > to))
    {
        return Ok(false);
    }
    Ok(true)
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
    fn gallery_api_returns_remote_thumbnail_notes() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("p");
        Pack::init(&root, "p").unwrap();
        std::fs::write(
            root.join("notes/remote.md"),
            "---
type: video
title: Remote Clip
remote_url: https://archive.org/details/example
thumbnail_url: https://archive.org/services/img/example
media_kind: image
mime: image/jpeg
tags: [gallery]
---
원격 썸네일로 보이는 영상",
        )
        .unwrap();
        let pack = Pack::open(&root).unwrap();
        pack.build_index().unwrap();

        let gallery_response = gallery(&pack, Some("video"), 10).unwrap();
        assert_eq!(gallery_response.items.len(), 1);
        assert_eq!(gallery_response.items[0].note_type, "video");
        assert_eq!(gallery_response.items[0].asset, None);
        assert_eq!(
            gallery_response.items[0].asset_url.as_deref(),
            Some("https://archive.org/services/img/example")
        );
        assert_eq!(
            gallery_response.items[0].remote_url.as_deref(),
            Some("https://archive.org/details/example")
        );
        assert_eq!(
            gallery_response.items[0].thumbnail_url.as_deref(),
            Some("https://archive.org/services/img/example")
        );
        assert_eq!(
            gallery_response.items[0].media_kind.as_deref(),
            Some("image")
        );
        assert_eq!(
            gallery_response.items[0].mime.as_deref(),
            Some("image/jpeg")
        );

        let search_response = search(&pack, "원격", None, 10).unwrap();
        assert_eq!(
            search_response.hits[0].asset_url.as_deref(),
            Some("https://archive.org/services/img/example")
        );
        assert_eq!(
            search_response.hits[0].remote_url.as_deref(),
            Some("https://archive.org/details/example")
        );
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
    fn note_api_returns_enrichment_keyframe_assets() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("p");
        Pack::init(&root, "p").unwrap();
        std::fs::write(
            root.join("notes/clip.md"),
            "---
type: video
title: Demo Clip
asset: assets/demo.mp4
---
영상 캡션

<!-- ontopack:enrichment:start -->
## AI Caption
영상 캡션

## Keyframes
- [00:00:00] Representative video frame extracted at 00:00:00. — `assets/.derived/clip/keyframe-0000.jpg`
- [00:00:04] Candidate without asset

## Enrichment Metadata
status: done
<!-- ontopack:enrichment:end -->
",
        )
        .unwrap();
        let pack = Pack::open(&root).unwrap();

        let response = note(&pack, "clip").unwrap();
        assert_eq!(response.keyframes.len(), 2);
        assert_eq!(response.keyframes[0].time, "00:00:00");
        assert_eq!(
            response.keyframes[0].asset.as_deref(),
            Some("assets/.derived/clip/keyframe-0000.jpg")
        );
        assert_eq!(
            response.keyframes[0].asset_url.as_deref(),
            Some("/assets/.derived/clip/keyframe-0000.jpg")
        );
        assert_eq!(response.keyframes[1].asset_url, None);
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
    fn search_api_returns_timeline_media_citation_for_transcript_hits() {
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
영상 설명

## Transcript
[00:01:05] cockpit overview with semantic needle
",
        )
        .unwrap();
        let pack = Pack::open(&root).unwrap();
        pack.build_index().unwrap();

        let response = search(&pack, "semantic needle", None, 10).unwrap();
        let citation = response.hits[0].media_citation.as_ref().unwrap();
        assert_eq!(response.hits[0].note_id, "clip");
        assert_eq!(citation.time, "00:01:05");
        assert_eq!(citation.seconds, 65);
        assert_eq!(citation.asset_url, "/assets/demo%20clip.mp4#t=65");
    }

    #[test]
    fn search_api_does_not_add_media_citation_to_images() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("p");
        Pack::init(&root, "p").unwrap();
        std::fs::write(
            root.join("notes/pic.md"),
            "---
type: image
title: Pic
asset: assets/pic.png
---
[00:01] timestamp-looking image caption
",
        )
        .unwrap();
        let pack = Pack::open(&root).unwrap();
        pack.build_index().unwrap();

        let response = search(&pack, "timestamp-looking", None, 10).unwrap();
        assert_eq!(response.hits[0].media_kind.as_deref(), Some("image"));
        assert_eq!(response.hits[0].media_citation, None);
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
