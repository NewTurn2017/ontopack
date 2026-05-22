#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chunk {
    pub id: String,
    pub note_id: String,
    pub ord: i64,
    pub text: String,
}

pub fn chunk_text(
    note_id: &str,
    body: &str,
    chunk_chars: usize,
    overlap_chars: usize,
) -> Vec<Chunk> {
    let trimmed = body.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }
    let chars: Vec<char> = trimmed.chars().collect();
    let chunk_chars = chunk_chars.max(1);
    let overlap_chars = overlap_chars.min(chunk_chars.saturating_sub(1));
    let step = chunk_chars - overlap_chars;

    let mut chunks = Vec::new();
    let mut start = 0usize;
    let mut ord = 0i64;
    while start < chars.len() {
        let end = (start + chunk_chars).min(chars.len());
        let text: String = chars[start..end].iter().collect();
        chunks.push(Chunk {
            id: format!("{note_id}#{ord:04}"),
            note_id: note_id.to_string(),
            ord,
            text,
        });
        if end == chars.len() {
            break;
        }
        start += step;
        ord += 1;
    }
    chunks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_body_becomes_one_chunk() {
        let chunks = chunk_text("note-a", "짧은 본문", 20, 5);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].id, "note-a#0000");
        assert_eq!(chunks[0].note_id, "note-a");
        assert_eq!(chunks[0].ord, 0);
        assert_eq!(chunks[0].text, "짧은 본문");
    }

    #[test]
    fn long_body_chunks_with_overlap_without_breaking_utf8() {
        let chunks = chunk_text("n", "가나다라마바사아자차카타파하", 6, 2);
        assert_eq!(
            chunks.iter().map(|c| c.text.as_str()).collect::<Vec<_>>(),
            vec!["가나다라마바", "마바사아자차", "자차카타파하"]
        );
        assert_eq!(chunks[1].id, "n#0001");
        assert_eq!(chunks[2].ord, 2);
    }

    #[test]
    fn blank_body_produces_no_chunks() {
        assert!(chunk_text("n", "  \n\t", 10, 2).is_empty());
    }
}
