use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum OxideError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("TOML error: {0}")]
    Toml(#[from] toml::de::Error),

    #[error("Manifest error: {0}")]
    Manifest(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Database error: {0}")]
    Database(String),

    #[error("Parser error: {0}")]
    Parser(String),

    #[error("Embedding/ML error: {0}")]
    Ml(String),

    #[error("Project memory not initialized at {0}. Run `oxide-embed init` first.")]
    NotInitialized(PathBuf),

    #[error("Incompatible schema version: found {found}, supported {supported}")]
    IncompatibleSchema { found: u32, supported: u32 },

    #[error("Incompatible vector set: stored {stored}, current {current}")]
    IncompatibleVectorSet { stored: String, current: String },

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Other error: {0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, OxideError>;
