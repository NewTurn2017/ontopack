#[derive(Debug, Clone, PartialEq)]
pub struct NoteHit {
    pub id: String,
    pub title: String,
    pub note_type: String,
    /// FTS5 bm25 score. Lower is better.
    pub score: f64,
}

#[cfg(test)]
mod tests {
    use crate::index::Index;
    use crate::note::parse_str;

    #[test]
    fn keyword_search_ranks_match() {
        let mut idx = Index::open_in_memory().unwrap();
        let a = parse_str("a", "---\ntitle: 고래\n---\n바다 고래 이야기").unwrap();
        let b = parse_str("b", "---\ntitle: 자동차\n---\n도로 자동차 이야기").unwrap();
        idx.rebuild(&[a, b]).unwrap();

        let hits = idx.search_keyword("고래", 10).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].id, "a");
        assert_eq!(hits[0].title, "고래");

        let none = idx.search_keyword("비행기", 10).unwrap();
        assert!(none.is_empty());
    }
}
