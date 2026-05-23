use anyhow::{bail, Result};
use pack_core::pack::Pack;
use pack_core::search::{RankSource, SearchFilters as CoreSearchFilters, SearchHit};
use serde::Serialize;

const MAX_SEARCH_K: usize = 100;

#[derive(Debug, Serialize, PartialEq)]
pub struct SearchResponse {
    pub query: String,
    pub hits: Vec<SearchCard>,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct SearchFilters<'a> {
    pub note_type: Option<&'a str>,
    pub tag: Option<&'a str>,
    pub from: Option<&'a str>,
    pub to: Option<&'a str>,
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
}

#[derive(Debug, Serialize, PartialEq)]
pub struct AskResponse {
    pub question: String,
    pub answer_mode: String,
    pub instruction: String,
    pub context_blocks: Vec<SearchCard>,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct NoteDetail {
    pub id: String,
    pub title: String,
    pub note_type: String,
    pub tags: Vec<String>,
    pub created: Option<String>,
    pub asset: Option<String>,
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
pub struct GalleryItem {
    pub id: String,
    pub title: String,
    pub note_type: String,
    pub tags: Vec<String>,
    pub asset: Option<String>,
    pub path: String,
    pub caption: String,
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
        hits: hits.into_iter().map(search_card).collect(),
    })
}

pub fn ask(pack: &Pack, question: &str, k: usize) -> Result<AskResponse> {
    let k = k.clamp(1, MAX_SEARCH_K);
    Ok(AskResponse {
        question: question.to_string(),
        answer_mode: "external_llm_required".to_string(),
        instruction:
            "Use context_blocks to synthesize an answer with citations outside deterministic pack-core."
                .to_string(),
        context_blocks: pack
            .search_keyword_chunks(question, k)?
            .into_iter()
            .map(search_card)
            .collect(),
    })
}

pub fn note(pack: &Pack, id: &str) -> Result<NoteDetail> {
    let Some(note) = pack.scan_notes()?.into_iter().find(|note| note.id == id) else {
        bail!("note not found: {id}");
    };
    Ok(NoteDetail {
        id: note.id,
        title: note.title,
        note_type: note.note_type,
        tags: note.tags,
        created: note.created,
        asset: note.asset,
        related: note.related,
        body: note.body,
        path: note.path.to_string_lossy().to_string(),
    })
}

pub fn related(pack: &Pack, note_id: &str, depth: usize) -> Result<RelatedResponse> {
    Ok(RelatedResponse {
        note_id: note_id.to_string(),
        related: pack
            .related_notes(note_id, depth)?
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
            .timeline_notes(from, to, note_type, k)?
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
    let notes = pack.scan_notes()?;
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
                .map(|to| GraphEdge {
                    from: note.id.clone(),
                    to: to.clone(),
                })
        })
        .collect();
    Ok(GraphResponse { nodes, edges })
}

pub fn facets(pack: &Pack) -> Result<FacetsResponse> {
    let mut types = std::collections::BTreeSet::new();
    let mut tags = std::collections::BTreeSet::new();
    let mut created_values = Vec::new();
    for note in pack.scan_notes()? {
        types.insert(note.note_type);
        tags.extend(note.tags);
        if let Some(created) = note.created {
            created_values.push(created);
        }
    }
    created_values.sort();
    Ok(FacetsResponse {
        types: types.into_iter().collect(),
        tags: tags.into_iter().collect(),
        created_min: created_values.first().cloned(),
        created_max: created_values.last().cloned(),
    })
}

pub fn gallery(pack: &Pack, note_type: Option<&str>, k: usize) -> Result<GalleryResponse> {
    let mut items = Vec::new();
    for note in pack.scan_notes()? {
        if note.asset.is_none() || note_type.is_some_and(|note_type| note.note_type != note_type) {
            continue;
        }
        items.push(GalleryItem {
            id: note.id,
            title: note.title,
            note_type: note.note_type,
            tags: note.tags,
            asset: note.asset,
            path: note.path.to_string_lossy().to_string(),
            caption: note.body.trim().to_string(),
        });
        if items.len() >= k.max(1) {
            break;
        }
    }
    Ok(GalleryResponse { items })
}

fn search_card(hit: SearchHit) -> SearchCard {
    SearchCard {
        note_id: hit.note_id,
        chunk_id: hit.chunk_id,
        title: hit.title,
        note_type: hit.note_type,
        snippet: hit.snippet,
        path: hit.path,
        score: hit.score,
        rank_source: rank_source_label(hit.rank_source).to_string(),
    }
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
    }
}
