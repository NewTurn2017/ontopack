use anyhow::Result;
use regex::Regex;
use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

#[derive(Debug, Clone)]
pub struct Note {
    pub id: String,
    pub path: PathBuf,
    pub note_type: String,
    pub title: String,
    pub tags: Vec<String>,
    pub created: Option<String>,
    pub asset: Option<String>,
    pub related: Vec<String>,
    pub body: String,
    pub mtime: i64,
}

impl Note {
    pub fn content_hash(&self) -> String {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.id.hash(&mut hasher);
        self.note_type.hash(&mut hasher);
        self.title.hash(&mut hasher);
        self.tags.hash(&mut hasher);
        self.created.hash(&mut hasher);
        self.asset.hash(&mut hasher);
        self.related.hash(&mut hasher);
        self.body.hash(&mut hasher);
        format!("{:016x}", hasher.finish())
    }
}

#[derive(Debug, Default, Deserialize)]
struct FrontMatter {
    #[serde(rename = "type")]
    note_type: Option<String>,
    title: Option<String>,
    tags: Option<Vec<String>>,
    created: Option<String>,
    asset: Option<String>,
    related: Option<Vec<String>>,
}

/// `---\n...\n---\n` frontmatter를 본문과 분리한다. frontmatter가 없으면 (None, 전체).
fn split_frontmatter(raw: &str) -> (Option<&str>, &str) {
    let trimmed = raw.strip_prefix('\u{feff}').unwrap_or(raw);
    if let Some(rest) = trimmed.strip_prefix("---\n") {
        if let Some(end) = rest.find("\n---\n") {
            let fm = &rest[..end];
            let body = &rest[end + 5..];
            return (Some(fm), body);
        }
        if let Some(end) = rest.find("\n---") {
            let fm = &rest[..end];
            let body = rest.get(end + 4..).unwrap_or("");
            return (Some(fm), body);
        }
    }
    (None, trimmed)
}

/// 위키링크 문자열 `"[[name|alias]]"` → 정규화된 id `name`.
fn normalize_link(s: &str) -> String {
    let s = s.trim();
    let s = s.strip_prefix("[[").unwrap_or(s);
    let s = s.strip_suffix("]]").unwrap_or(s);
    s.split('|').next().unwrap_or(s).trim().to_string()
}

/// 본문에서 `[[...]]` 위키링크 id 목록을 등장 순서대로(중복 제거) 추출.
pub fn extract_wikilinks(text: &str) -> Vec<String> {
    let re = Regex::new(r"\[\[([^\]]+)\]\]").unwrap();
    let mut out = Vec::new();
    for cap in re.captures_iter(text) {
        let id = normalize_link(&cap[1]);
        if !id.is_empty() && !out.contains(&id) {
            out.push(id);
        }
    }
    out
}

/// id와 원문으로 Note 생성. mtime은 0(파일에서 읽을 땐 parse_file이 채움).
pub fn parse_str(id: &str, raw: &str) -> Result<Note> {
    let (fm_raw, body) = split_frontmatter(raw);
    let fm: FrontMatter = match fm_raw {
        Some(f) => serde_yaml::from_str(f)?,
        None => FrontMatter::default(),
    };
    let mut related: Vec<String> = fm
        .related
        .unwrap_or_default()
        .iter()
        .map(|s| normalize_link(s))
        .collect();
    for link in extract_wikilinks(body) {
        if !related.contains(&link) {
            related.push(link);
        }
    }
    Ok(Note {
        id: id.to_string(),
        path: PathBuf::new(),
        note_type: fm.note_type.unwrap_or_else(|| "note".to_string()),
        title: fm.title.unwrap_or_else(|| id.to_string()),
        tags: fm.tags.unwrap_or_default(),
        created: fm.created,
        asset: fm.asset,
        related,
        body: body.to_string(),
        mtime: 0,
    })
}

/// 파일을 읽어 Note로. id = 파일명(확장자 제외), mtime = 수정시각(초).
pub fn parse_file(path: &Path) -> Result<Note> {
    let id = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("untitled")
        .to_string();
    parse_file_with_id(path, &id)
}

/// 파일을 읽어 지정 id의 Note로. 팩 스캔은 상대 경로 기반 id를 주입한다.
pub fn parse_file_with_id(path: &Path, id: &str) -> Result<Note> {
    let raw = std::fs::read_to_string(path)?;
    let mut note = parse_str(id, &raw)?;
    note.path = path.to_path_buf();
    let meta = std::fs::metadata(path)?;
    note.mtime = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    Ok(note)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_frontmatter_and_body() {
        let raw = "---\ntype: prompt\ntitle: 썸네일 훅\ntags: [thumbnail, hook]\nrelated:\n  - \"[[project_오로라]]\"\n---\n본문 텍스트.\n";
        let note = parse_str("prompt_x", raw).unwrap();
        assert_eq!(note.id, "prompt_x");
        assert_eq!(note.note_type, "prompt");
        assert_eq!(note.title, "썸네일 훅");
        assert_eq!(note.tags, vec!["thumbnail", "hook"]);
        assert_eq!(note.related, vec!["project_오로라"]);
        assert_eq!(note.body.trim(), "본문 텍스트.");
    }

    #[test]
    fn defaults_when_no_frontmatter() {
        let note = parse_str("plain", "그냥 본문만.").unwrap();
        assert_eq!(note.note_type, "note");
        assert_eq!(note.title, "plain");
        assert!(note.tags.is_empty());
        assert_eq!(note.body.trim(), "그냥 본문만.");
    }

    #[test]
    fn extracts_body_wikilinks() {
        let links = extract_wikilinks("앞 [[a]] 중간 [[b|별칭]] 끝 [[c]]");
        assert_eq!(links, vec!["a", "b", "c"]);
    }

    #[test]
    fn related_merges_frontmatter_and_body_dedup() {
        let raw = "---\nrelated:\n  - \"[[a]]\"\n---\n본문 [[a]] 그리고 [[d]]";
        let note = parse_str("x", raw).unwrap();
        assert_eq!(note.related, vec!["a", "d"]);
    }

    #[test]
    fn content_hash_changes_when_body_or_metadata_changes() {
        let a = parse_str(
            "x",
            "---
title: A
---
본문",
        )
        .unwrap();
        let b = parse_str(
            "x",
            "---
title: A
---
본문 changed",
        )
        .unwrap();
        let c = parse_str(
            "x",
            "---
title: B
---
본문",
        )
        .unwrap();
        assert_ne!(a.content_hash(), b.content_hash());
        assert_ne!(a.content_hash(), c.content_hash());
        assert_eq!(a.content_hash(), a.content_hash());
    }
}
