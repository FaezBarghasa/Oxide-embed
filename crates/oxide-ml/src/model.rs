use std::fs;
use std::path::PathBuf;
use directories::ProjectDirs;
use oxide_core::error::{OxideError, Result};

pub struct ModelManager;

impl ModelManager {
    pub fn default_models_dir() -> Result<PathBuf> {
        let proj_dirs = ProjectDirs::from("com", "oxide", "oxide-embed")
            .ok_or_else(|| OxideError::Ml("Failed to resolve project directories".to_string()))?;
        let cache_dir = proj_dirs.cache_dir().join("models");
        fs::create_dir_all(&cache_dir)?;
        Ok(cache_dir)
    }

    pub fn get_model_path(model_filename: &str) -> Result<PathBuf> {
        let dir = Self::default_models_dir()?;
        Ok(dir.join(model_filename))
    }

    pub fn is_model_available(model_filename: &str) -> bool {
        Self::get_model_path(model_filename)
            .map(|p| p.exists())
            .unwrap_or(false)
    }
}
