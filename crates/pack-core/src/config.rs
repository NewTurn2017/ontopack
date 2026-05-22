use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Deserialize)]
pub struct PackConfig {
    pub name: String,
    #[serde(default)]
    pub types: Vec<String>,
    #[serde(default)]
    pub relations: Vec<String>,
    /// M2에서 사용. 지금은 자리만.
    #[serde(default = "default_embed_model")]
    pub embed_model: String,
    #[serde(default = "default_embed_enabled")]
    pub embed_enabled: bool,
    #[serde(default = "default_embed_dim")]
    pub embed_dim: usize,
    #[serde(default = "default_chunk_size")]
    pub chunk_size: usize,
    #[serde(default = "default_chunk_overlap")]
    pub chunk_overlap: usize,
}

fn default_embed_model() -> String {
    "bge-m3".to_string()
}

fn default_embed_enabled() -> bool {
    true
}

fn default_embed_dim() -> usize {
    1024
}

fn default_chunk_size() -> usize {
    900
}

fn default_chunk_overlap() -> usize {
    120
}

impl PackConfig {
    /// 팩 루트의 pack.toml을 읽는다.
    pub fn load(root: &Path) -> Result<PackConfig> {
        let raw = std::fs::read_to_string(root.join("pack.toml"))?;
        Ok(toml::from_str(&raw)?)
    }

    /// init 시 기본 설정의 직렬화 문자열.
    pub fn default_toml(name: &str) -> String {
        #[derive(Serialize)]
        struct DefaultConfigToml<'a> {
            name: &'a str,
            types: [&'static str; 4],
            relations: [&'static str; 1],
            embed_model: &'static str,
            embed_enabled: bool,
            embed_dim: usize,
            chunk_size: usize,
            chunk_overlap: usize,
        }

        toml::to_string(&DefaultConfigToml {
            name,
            types: ["prompt", "image", "video", "project"],
            relations: ["related"],
            embed_model: "bge-m3",
            embed_enabled: true,
            embed_dim: 1024,
            chunk_size: 900,
            chunk_overlap: 120,
        })
        .expect("default pack config is serializable")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_config_with_defaults() {
        let cfg: PackConfig = toml::from_str("name = \"내 팩\"\n").unwrap();
        assert_eq!(cfg.name, "내 팩");
        assert!(cfg.types.is_empty());
        assert_eq!(cfg.embed_model, "bge-m3");
        assert!(cfg.embed_enabled);
        assert_eq!(cfg.embed_dim, 1024);
        assert_eq!(cfg.chunk_size, 900);
        assert_eq!(cfg.chunk_overlap, 120);
    }

    #[test]
    fn parses_types_and_relations() {
        let cfg: PackConfig = toml::from_str(
            "name = \"p\"\ntypes = [\"prompt\", \"image\"]\nrelations = [\"related\"]\n",
        )
        .unwrap();
        assert_eq!(cfg.types, vec!["prompt", "image"]);
        assert_eq!(cfg.relations, vec!["related"]);
    }
}
