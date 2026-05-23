use anyhow::{bail, Result};
use pack_core::pack::Pack;
use pack_core::search::{RankSource, SearchHit};
use serde::Serialize;

#[derive(Debug, Serialize, PartialEq)]
pub struct SearchResponse {
    pub query: String,
    pub hits: Vec<SearchCard>,
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

pub fn search(
    pack: &Pack,
    query: &str,
    note_type: Option<&str>,
    k: usize,
) -> Result<SearchResponse> {
    let mut hits = pack.search_keyword_chunks(query, k)?;
    if let Some(note_type) = note_type {
        hits.retain(|hit| hit.note_type == note_type);
    }
    Ok(SearchResponse {
        query: query.to_string(),
        hits: hits.into_iter().map(search_card).collect(),
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
}
