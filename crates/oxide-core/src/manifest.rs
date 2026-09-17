use crate::embedding::EmbeddingMetadata;
use crate::error::{OxideError, Result};
use crate::id::ProjectId;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

pub const CURRENT_SCHEMA_VERSION: u32 = 1;
pub const MIN_SUPPORTED_SCHEMA_VERSION: u32 = 1;
pub const MANIFEST_FILENAME: &str = "manifest.json";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VersionMeta {
    pub oxide_embed_version: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StorageMeta {
    pub engine: String,
    pub mode: String,
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FeaturesMeta {
    pub tree_sitter: bool,
    pub symbol_graph: bool,
    pub token_stats: bool,
    pub secrets_scan: bool,
    pub egress_scan: bool,
}

impl Default for FeaturesMeta {
    fn default() -> Self {
        Self {
            tree_sitter: true,
            symbol_graph: false,
            token_stats: true,
            secrets_scan: false,
            egress_scan: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OxideManifest {
    pub app: String,
    pub kind: String,
    pub project_id: ProjectId,
    pub project_name: String,
    pub schema_version: u32,
    pub min_supported_schema_version: u32,
    pub created_by: VersionMeta,
    pub updated_by: VersionMeta,
    pub storage: StorageMeta,
    pub embedding: EmbeddingMetadata,
    pub features: FeaturesMeta,
}

impl OxideManifest {
    pub fn new(project_name: &str) -> Self {
        let now = Utc::now();
        let version = env!("CARGO_PKG_VERSION").to_string();
        Self {
            app: "oxide-embed".to_string(),
            kind: "project-memory".to_string(),
            project_id: ProjectId::new_v7(),
            project_name: project_name.to_string(),
            schema_version: CURRENT_SCHEMA_VERSION,
            min_supported_schema_version: MIN_SUPPORTED_SCHEMA_VERSION,
            created_by: VersionMeta {
                oxide_embed_version: version.clone(),
                timestamp: now,
            },
            updated_by: VersionMeta {
                oxide_embed_version: version,
                timestamp: now,
            },
            storage: StorageMeta {
                engine: "surrealdb".to_string(),
                mode: "embedded".to_string(),
                path: "project.db".to_string(),
            },
            embedding: EmbeddingMetadata::qwen3_0_6b_default(),
            features: FeaturesMeta::default(),
        }
    }

    pub fn load_from_dir<P: AsRef<Path>>(dir: P) -> Result<Self> {
        let path = dir.as_ref().join(MANIFEST_FILENAME);
        if !path.exists() {
            return Err(OxideError::Manifest(format!(
                "Manifest file not found at {}",
                path.display()
            )));
        }
        let content = fs::read_to_string(&path)?;
        let manifest: Self = serde_json::from_str(&content)?;
        manifest.validate()?;
        Ok(manifest)
    }

    pub fn save_to_dir<P: AsRef<Path>>(&mut self, dir: P) -> Result<()> {
        let path = dir.as_ref().join(MANIFEST_FILENAME);
        self.updated_by = VersionMeta {
            oxide_embed_version: env!("CARGO_PKG_VERSION").to_string(),
            timestamp: Utc::now(),
        };
        let content = serde_json::to_string_pretty(self)?;
        fs::write(path, content)?;
        Ok(())
    }

    pub fn validate(&self) -> Result<()> {
        if self.schema_version > CURRENT_SCHEMA_VERSION {
            return Err(OxideError::IncompatibleSchema {
                found: self.schema_version,
                supported: CURRENT_SCHEMA_VERSION,
            });
        }
        if self.min_supported_schema_version > CURRENT_SCHEMA_VERSION {
            return Err(OxideError::IncompatibleSchema {
                found: self.min_supported_schema_version,
                supported: CURRENT_SCHEMA_VERSION,
            });
        }
        Ok(())
    }

    pub fn db_path<P: AsRef<Path>>(&self, oxide_dir: P) -> std::path::PathBuf {
        let dir = oxide_dir.as_ref();
        let target = dir.join(&self.storage.path);
        if target.exists() {
            target
        } else if dir.join("project.db").exists() {
            dir.join("project.db")
        } else if dir.join("db").exists() {
            dir.join("db")
        } else {
            target
        }
    }
}

pub fn resolve_db_path<P: AsRef<Path>>(project_root: P) -> Result<std::path::PathBuf> {
    let oxide_dir = project_root.as_ref().join(".oxide");
    if !oxide_dir.exists() {
        return Err(OxideError::NotInitialized(
            project_root.as_ref().to_path_buf(),
        ));
    }

    if let Ok(manifest) = OxideManifest::load_from_dir(&oxide_dir) {
        let p = manifest.db_path(&oxide_dir);
        if p.exists() {
            return Ok(p);
        }
    }

    let project_db = oxide_dir.join("project.db");
    if project_db.exists() {
        return Ok(project_db);
    }

    let legacy_db = oxide_dir.join("db");
    if legacy_db.exists() {
        return Ok(legacy_db);
    }

    Err(OxideError::Config(
        "Oxide database not found. Run 'oxide-embed init' and 'oxide-embed index' first.".into(),
    ))
}
