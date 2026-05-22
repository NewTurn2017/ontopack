use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InferredType {
    Note,
    Image,
    Video,
    Asset,
}

impl InferredType {
    pub fn as_note_type(&self) -> &'static str {
        match self {
            InferredType::Note => "note",
            InferredType::Image => "image",
            InferredType::Video => "video",
            InferredType::Asset => "asset",
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ProcessReport {
    pub processed: usize,
    pub created: Vec<PathBuf>,
}

pub fn infer_type(file_name: &str) -> InferredType {
    let ext = std::path::Path::new(file_name)
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    match ext.as_str() {
        "md" | "markdown" | "txt" => InferredType::Note,
        "png" | "jpg" | "jpeg" | "gif" | "webp" => InferredType::Image,
        "mp4" | "mov" | "mkv" | "webm" => InferredType::Video,
        _ => InferredType::Asset,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn infers_type_from_extension() {
        assert_eq!(infer_type("memo.md"), InferredType::Note);
        assert_eq!(infer_type("pic.png"), InferredType::Image);
        assert_eq!(infer_type("clip.mp4"), InferredType::Video);
        assert_eq!(infer_type("data.bin"), InferredType::Asset);
    }
}
