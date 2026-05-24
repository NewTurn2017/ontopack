#[derive(Debug, Clone, PartialEq)]
pub struct NoteHit {
    pub id: String,
    pub title: String,
    pub note_type: String,
    /// FTS5 bm25 score. Lower is better.
    pub score: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RankSource {
    Keyword,
    Vector,
    Hybrid,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SearchHit {
    pub note_id: String,
    pub chunk_id: String,
    pub title: String,
    pub note_type: String,
    pub snippet: String,
    pub path: String,
    pub asset: Option<String>,
    pub remote_url: Option<String>,
    pub thumbnail_url: Option<String>,
    pub media_kind: Option<String>,
    pub mime: Option<String>,
    /// Higher is better for SearchHit/RRF results.
    pub score: f64,
    pub rank_source: RankSource,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct SearchFilters<'a> {
    pub note_type: Option<&'a str>,
    pub tag: Option<&'a str>,
    pub from: Option<&'a str>,
    pub to: Option<&'a str>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchMode {
    Keyword,
    Vector,
    Hybrid,
}

pub fn rrf_fuse(keyword_hits: &[SearchHit], vector_hits: &[SearchHit], k: usize) -> Vec<SearchHit> {
    const RRF_K: f64 = 60.0;
    let mut fused: Vec<SearchHit> = Vec::new();

    add_ranked_hits(&mut fused, keyword_hits, RRF_K, RankSource::Keyword);
    add_ranked_hits(&mut fused, vector_hits, RRF_K, RankSource::Vector);

    fused.sort_by(|a, b| {
        b.score
            .total_cmp(&a.score)
            .then_with(|| a.note_id.cmp(&b.note_id))
            .then_with(|| a.chunk_id.cmp(&b.chunk_id))
    });
    fused.truncate(k);
    fused
}

fn add_ranked_hits(fused: &mut Vec<SearchHit>, hits: &[SearchHit], rrf_k: f64, source: RankSource) {
    for (rank, hit) in hits.iter().enumerate() {
        let contribution = 1.0 / (rrf_k + rank as f64 + 1.0);
        if let Some(existing) = fused.iter_mut().find(|h| h.chunk_id == hit.chunk_id) {
            existing.score += contribution;
            if existing.rank_source != source {
                existing.rank_source = RankSource::Hybrid;
            }
        } else {
            let mut cloned = hit.clone();
            cloned.score = contribution;
            cloned.rank_source = source;
            fused.push(cloned);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rrf_fusion_promotes_hits_seen_by_both_rankers() {
        let keyword = vec![
            fixture_hit("a", "a#0000", RankSource::Keyword),
            fixture_hit("b", "b#0000", RankSource::Keyword),
        ];
        let vector = vec![
            fixture_hit("b", "b#0000", RankSource::Vector),
            fixture_hit("c", "c#0000", RankSource::Vector),
        ];

        let fused = rrf_fuse(&keyword, &vector, 10);
        assert_eq!(fused[0].note_id, "b");
        assert_eq!(fused[0].chunk_id, "b#0000");
        assert_eq!(fused[0].rank_source, RankSource::Hybrid);
        assert!(fused[0].score > fused[1].score);
    }

    #[test]
    fn rrf_fusion_breaks_ties_deterministically() {
        let keyword = vec![
            fixture_hit("b", "b#0000", RankSource::Keyword),
            fixture_hit("a", "a#0000", RankSource::Keyword),
        ];

        let fused = rrf_fuse(&keyword, &[], 10);
        assert_eq!(fused[0].note_id, "b");
        assert_eq!(fused[1].note_id, "a");
    }

    fn fixture_hit(note_id: &str, chunk_id: &str, rank_source: RankSource) -> SearchHit {
        SearchHit {
            note_id: note_id.to_string(),
            chunk_id: chunk_id.to_string(),
            title: note_id.to_string(),
            note_type: "note".to_string(),
            snippet: format!("{note_id} snippet"),
            path: format!("notes/{note_id}.md"),
            asset: None,
            remote_url: None,
            thumbnail_url: None,
            media_kind: None,
            mime: None,
            score: 0.0,
            rank_source,
        }
    }
}
