use crate::error::{OxideError, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

pub const CONFIG_FILENAME: &str = "config.toml";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectSection {
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IndexSection {
    pub ignore: Vec<String>,
    pub max_file_kb: u64,
    pub chunk_max_tokens: usize,
    pub outline_max_tokens: usize,
}

impl Default for IndexSection {
    fn default() -> Self {
        Self {
            ignore: vec![
                ".git".to_string(),
                "target".to_string(),
                "node_modules".to_string(),
                "dist".to_string(),
                "build".to_string(),
                ".venv".to_string(),
                "__pycache__".to_string(),
                "*.lock".to_string(),
            ],
            max_file_kb: 512,
            chunk_max_tokens: 1024,
            outline_max_tokens: 1024,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EmbeddingSection {
    pub model: String,
    pub file: String,
    pub device: String,
    pub max_seq_len: usize,
    pub batch_size: usize,
}

impl Default for EmbeddingSection {
    fn default() -> Self {
        Self {
            model: "qwen3-embedding-0.6b".to_string(),
            file: "Qwen3-Embedding-0.6B-Q4_K_M.gguf".to_string(),
            device: "auto".to_string(),
            max_seq_len: 1024,
            batch_size: 32,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StorageSection {
    pub engine: String,
    pub mode: String,
    pub path: String,
}

impl Default for StorageSection {
    fn default() -> Self {
        Self {
            engine: "surrealdb".to_string(),
            mode: "embedded".to_string(),
            path: "project.db".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UpdatesSection {
    pub auto_migrate: bool,
    pub backup_before_migrate: bool,
    pub read_only_if_newer_schema: bool,
}

impl Default for UpdatesSection {
    fn default() -> Self {
        Self {
            auto_migrate: true,
            backup_before_migrate: true,
            read_only_if_newer_schema: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PrivacySection {
    pub store_raw_chunks: bool,
    pub store_global_memory: bool,
    pub store_agent_transcripts: bool,
}

impl Default for PrivacySection {
    fn default() -> Self {
        Self {
            store_raw_chunks: true,
            store_global_memory: false,
            store_agent_transcripts: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OxideConfig {
    pub version: u32,
    pub project: ProjectSection,
    pub index: IndexSection,
    pub embedding: EmbeddingSection,
    pub storage: StorageSection,
    pub updates: UpdatesSection,
    pub privacy: PrivacySection,
}

impl OxideConfig {
    pub fn default_for_project(project_name: &str) -> Self {
        Self {
            version: 1,
            project: ProjectSection {
                name: project_name.to_string(),
            },
            index: IndexSection::default(),
            embedding: EmbeddingSection::default(),
            storage: StorageSection::default(),
            updates: UpdatesSection::default(),
            privacy: PrivacySection::default(),
        }
    }

    pub fn load_from_dir<P: AsRef<Path>>(dir: P) -> Result<Self> {
        let path = dir.as_ref().join(CONFIG_FILENAME);
        if !path.exists() {
            return Err(OxideError::Config(format!(
                "Config file not found at {}",
                path.display()
            )));
        }
        let content = fs::read_to_string(&path)?;
        let config: Self = toml::from_str(&content)?;
        Ok(config)
    }

    pub fn save_to_dir<P: AsRef<Path>>(&self, dir: P) -> Result<()> {
        let path = dir.as_ref().join(CONFIG_FILENAME);
        let content =
            toml::to_string_pretty(self).map_err(|e| OxideError::Config(e.to_string()))?;
        fs::write(path, content)?;
        Ok(())
    }
}
