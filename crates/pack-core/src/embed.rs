#[cfg(test)]
use anyhow::anyhow;
use anyhow::{bail, Result};
#[cfg(test)]
use std::collections::HashMap;
#[cfg(feature = "real-embed")]
use std::str::FromStr;
#[cfg(feature = "real-embed")]
use std::sync::Mutex;

pub trait Embedder {
    fn dimension(&self) -> usize;
    fn embed_passages(&self, passages: &[String]) -> Result<Vec<Vec<f32>>>;
    fn embed_query(&self, query: &str) -> Result<Vec<f32>>;
}

pub fn f32s_to_vec_blob(values: &[f32]) -> Vec<u8> {
    let mut out = Vec::with_capacity(std::mem::size_of_val(values));
    for value in values {
        out.extend_from_slice(&value.to_le_bytes());
    }
    out
}

pub fn vec_blob_to_f32s(bytes: &[u8]) -> Result<Vec<f32>> {
    if !bytes.len().is_multiple_of(std::mem::size_of::<f32>()) {
        bail!("vector blob length is not a multiple of float32 bytes");
    }
    Ok(bytes
        .chunks_exact(std::mem::size_of::<f32>())
        .map(|chunk| f32::from_le_bytes(chunk.try_into().expect("chunk size checked")))
        .collect())
}

#[cfg(feature = "real-embed")]
pub struct FastEmbedder {
    model: Mutex<fastembed::TextEmbedding>,
    dimension: usize,
    batch_size: Option<usize>,
}

#[cfg(feature = "real-embed")]
impl FastEmbedder {
    pub fn bge_m3(dimension: usize, show_download_progress: bool) -> Result<Self> {
        Self::try_new("bge-m3", dimension, show_download_progress)
    }

    pub fn try_new(model_name: &str, dimension: usize, show_download_progress: bool) -> Result<Self> {
        let model_name = fastembed_model_from_name(model_name)?;
        let options =
            fastembed::InitOptions::new(model_name).with_show_download_progress(show_download_progress);
        let model = fastembed::TextEmbedding::try_new(options)?;
        Ok(Self {
            model: Mutex::new(model),
            dimension,
            batch_size: None,
        })
    }

    pub fn with_batch_size(mut self, batch_size: usize) -> Self {
        self.batch_size = Some(batch_size);
        self
    }
}

#[cfg(feature = "real-embed")]
impl Embedder for FastEmbedder {
    fn dimension(&self) -> usize {
        self.dimension
    }

    fn embed_passages(&self, passages: &[String]) -> Result<Vec<Vec<f32>>> {
        let prefixed: Vec<String> = passages
            .iter()
            .map(|passage| format!("passage: {passage}"))
            .collect();
        let mut model = self
            .model
            .lock()
            .map_err(|_| anyhow::anyhow!("fastembed model lock poisoned"))?;
        Ok(model.embed(prefixed, self.batch_size)?)
    }

    fn embed_query(&self, query: &str) -> Result<Vec<f32>> {
        let mut model = self
            .model
            .lock()
            .map_err(|_| anyhow::anyhow!("fastembed model lock poisoned"))?;
        let embeddings = model.embed([format!("query: {query}")], self.batch_size)?;
        embeddings
            .into_iter()
            .next()
            .ok_or_else(|| anyhow::anyhow!("fastembed returned no query embedding"))
    }
}

#[cfg(feature = "real-embed")]
pub fn fastembed_model_from_name(model_name: &str) -> Result<fastembed::EmbeddingModel> {
    let normalized = model_name.trim();
    if normalized.eq_ignore_ascii_case("bge-m3")
        || normalized.eq_ignore_ascii_case("BAAI/bge-m3")
    {
        return Ok(fastembed::EmbeddingModel::BGEM3);
    }
    fastembed::EmbeddingModel::from_str(normalized)
        .map_err(|err| anyhow::anyhow!("unsupported fastembed model '{model_name}': {err}"))
}

#[cfg(test)]
pub struct FakeEmbedder {
    dimension: usize,
    passages: HashMap<String, Vec<f32>>,
    queries: HashMap<String, Vec<f32>>,
}

#[cfg(test)]
impl FakeEmbedder {
    pub fn new(dimension: usize) -> Self {
        Self {
            dimension,
            passages: HashMap::new(),
            queries: HashMap::new(),
        }
    }

    pub fn with_passage(mut self, text: &str, vector: Vec<f32>) -> Self {
        assert_eq!(vector.len(), self.dimension);
        self.passages.insert(text.to_string(), vector);
        self
    }

    pub fn with_query(mut self, text: &str, vector: Vec<f32>) -> Self {
        assert_eq!(vector.len(), self.dimension);
        self.queries.insert(text.to_string(), vector);
        self
    }
}

#[cfg(test)]
impl Embedder for FakeEmbedder {
    fn dimension(&self) -> usize {
        self.dimension
    }

    fn embed_passages(&self, passages: &[String]) -> Result<Vec<Vec<f32>>> {
        passages
            .iter()
            .map(|text| {
                self.passages
                    .get(text)
                    .cloned()
                    .ok_or_else(|| anyhow!("missing fake passage embedding: {text}"))
            })
            .collect()
    }

    fn embed_query(&self, query: &str) -> Result<Vec<f32>> {
        self.queries
            .get(query)
            .cloned()
            .ok_or_else(|| anyhow!("missing fake query embedding: {query}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vector_blob_round_trips_f32_values() {
        let blob = f32s_to_vec_blob(&[0.25, -1.5, 3.0]);
        assert_eq!(blob.len(), 12);
        assert_eq!(vec_blob_to_f32s(&blob).unwrap(), vec![0.25, -1.5, 3.0]);
    }

    #[test]
    fn vector_blob_rejects_partial_f32_bytes() {
        let err = vec_blob_to_f32s(&[1, 2, 3]).unwrap_err();
        assert!(err.to_string().contains("float32"));
    }

    #[test]
    fn fake_embedder_returns_configured_passage_and_query_vectors() {
        let embedder = FakeEmbedder::new(3)
            .with_passage("수업 설계", vec![1.0, 0.0, 0.0])
            .with_query("강의 준비", vec![0.9, 0.1, 0.0]);

        assert_eq!(embedder.dimension(), 3);
        assert_eq!(
            embedder.embed_passages(&["수업 설계".to_string()]).unwrap(),
            vec![vec![1.0, 0.0, 0.0]]
        );
        assert_eq!(
            embedder.embed_query("강의 준비").unwrap(),
            vec![0.9, 0.1, 0.0]
        );
    }

    #[cfg(feature = "real-embed")]
    #[test]
    fn fastembed_model_mapping_accepts_bge_m3_aliases() {
        assert_eq!(
            fastembed_model_from_name("bge-m3").unwrap(),
            fastembed::EmbeddingModel::BGEM3
        );
        assert_eq!(
            fastembed_model_from_name("BAAI/bge-m3").unwrap(),
            fastembed::EmbeddingModel::BGEM3
        );
    }
}
