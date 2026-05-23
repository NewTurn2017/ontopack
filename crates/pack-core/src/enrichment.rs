use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

pub const ENRICHMENT_START: &str = "<!-- ontopack:enrichment:start -->";
pub const ENRICHMENT_END: &str = "<!-- ontopack:enrichment:end -->";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EnrichmentStatus {
    NotRequired,
    Pending,
    Done,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnrichmentKeyframe {
    pub time: String,
    pub text: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnrichmentPatch {
    pub caption: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    pub ocr: Option<String>,
    pub transcript: Option<String>,
    pub summary: Option<String>,
    #[serde(default)]
    pub keyframes: Vec<EnrichmentKeyframe>,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub generated_at: Option<String>,
}

impl EnrichmentPatch {
    pub fn is_empty(&self) -> bool {
        self.caption.as_deref().is_none_or(str::is_empty)
            && self.tags.is_empty()
            && self.ocr.as_deref().is_none_or(str::is_empty)
            && self.transcript.as_deref().is_none_or(str::is_empty)
            && self.summary.as_deref().is_none_or(str::is_empty)
            && self.keyframes.is_empty()
    }
}

pub fn status_for_body(body: &str, has_asset: bool) -> EnrichmentStatus {
    if !has_asset {
        return EnrichmentStatus::NotRequired;
    }
    let Some(block) = managed_block(body) else {
        return EnrichmentStatus::Pending;
    };
    if block.contains("status: error") {
        EnrichmentStatus::Error
    } else {
        EnrichmentStatus::Done
    }
}

pub fn apply_enrichment_patch(raw_note: &str, patch: &EnrichmentPatch) -> Result<String> {
    if patch.is_empty() {
        bail!("enrichment patch must contain at least one generated field");
    }
    let base = remove_managed_block(raw_note);
    let mut out = base.trim_end().to_string();
    if !out.is_empty() {
        out.push_str("\n\n");
    }
    out.push_str(&render_managed_block(patch));
    out.push('\n');
    Ok(out)
}

fn managed_block(raw: &str) -> Option<&str> {
    let start = raw.find(ENRICHMENT_START)?;
    let after_start = start + ENRICHMENT_START.len();
    let end = raw[after_start..].find(ENRICHMENT_END)? + after_start;
    Some(&raw[after_start..end])
}

fn remove_managed_block(raw: &str) -> String {
    let Some(start) = raw.find(ENRICHMENT_START) else {
        return raw.to_string();
    };
    let after_start = start + ENRICHMENT_START.len();
    let Some(end_rel) = raw[after_start..].find(ENRICHMENT_END) else {
        return raw.to_string();
    };
    let end = after_start + end_rel + ENRICHMENT_END.len();
    let mut out = String::with_capacity(raw.len());
    out.push_str(raw[..start].trim_end());
    out.push_str(raw[end..].trim_start_matches(['\r', '\n']));
    out
}

fn render_managed_block(patch: &EnrichmentPatch) -> String {
    let mut out = String::new();
    out.push_str(ENRICHMENT_START);
    out.push_str("\n## AI Caption\n");
    out.push_str(patch.caption.as_deref().unwrap_or(""));
    out.push('\n');

    if !patch.tags.is_empty() {
        out.push_str("\n## AI Tags\n");
        for tag in &patch.tags {
            out.push_str("- ");
            out.push_str(tag);
            out.push('\n');
        }
    }
    if let Some(ocr) = patch.ocr.as_deref().filter(|value| !value.is_empty()) {
        out.push_str("\n## OCR\n");
        out.push_str(ocr);
        out.push('\n');
    }
    if let Some(transcript) = patch
        .transcript
        .as_deref()
        .filter(|value| !value.is_empty())
    {
        out.push_str("\n## Transcript\n");
        out.push_str(transcript);
        out.push('\n');
    }
    if !patch.keyframes.is_empty() {
        out.push_str("\n## Keyframes\n");
        for keyframe in &patch.keyframes {
            out.push_str("- [");
            out.push_str(&keyframe.time);
            out.push_str("] ");
            out.push_str(&keyframe.text);
            out.push('\n');
        }
    }
    if let Some(summary) = patch.summary.as_deref().filter(|value| !value.is_empty()) {
        out.push_str("\n## AI Summary\n");
        out.push_str(summary);
        out.push('\n');
    }

    out.push_str("\n## Enrichment Metadata\n");
    out.push_str("status: done\n");
    if let Some(provider) = patch.provider.as_deref().filter(|value| !value.is_empty()) {
        out.push_str("provider: ");
        out.push_str(provider);
        out.push('\n');
    }
    if let Some(model) = patch.model.as_deref().filter(|value| !value.is_empty()) {
        out.push_str("model: ");
        out.push_str(model);
        out.push('\n');
    }
    if let Some(generated_at) = patch
        .generated_at
        .as_deref()
        .filter(|value| !value.is_empty())
    {
        out.push_str("generated_at: ");
        out.push_str(generated_at);
        out.push('\n');
    }
    out.push_str(ENRICHMENT_END);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn patch_replaces_only_managed_block() {
        let first = EnrichmentPatch {
            caption: Some("첫 캡션".to_string()),
            tags: vec!["tag-a".to_string()],
            provider: Some("codex".to_string()),
            ..EnrichmentPatch::default()
        };
        let second = EnrichmentPatch {
            caption: Some("둘째 캡션".to_string()),
            transcript: Some("[00:00] 전사".to_string()),
            ..EnrichmentPatch::default()
        };
        let raw = "---\ntitle: Pic\n---\n사람이 쓴 본문\n";
        let once = apply_enrichment_patch(raw, &first).unwrap();
        let twice = apply_enrichment_patch(&once, &second).unwrap();
        assert!(twice.contains("사람이 쓴 본문"));
        assert!(twice.contains("둘째 캡션"));
        assert!(twice.contains("[00:00] 전사"));
        assert!(!twice.contains("첫 캡션"));
        assert_eq!(twice.matches(ENRICHMENT_START).count(), 1);
    }

    #[test]
    fn status_is_pending_until_managed_block_exists() {
        assert_eq!(
            status_for_body("본문", false),
            EnrichmentStatus::NotRequired
        );
        assert_eq!(status_for_body("본문", true), EnrichmentStatus::Pending);
        let patch = EnrichmentPatch {
            caption: Some("캡션".to_string()),
            ..EnrichmentPatch::default()
        };
        let enriched = apply_enrichment_patch("본문", &patch).unwrap();
        assert_eq!(status_for_body(&enriched, true), EnrichmentStatus::Done);
    }
}
