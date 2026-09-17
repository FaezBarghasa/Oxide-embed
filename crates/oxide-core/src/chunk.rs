use crate::id::{ChunkId, FileId, SymbolId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ChunkKind {
    FileSummary,
    FileOutline,
    SymbolChunk,
    DocChunk,
    ConfigChunk,
}

impl ChunkKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::FileSummary => "file_summary",
            Self::FileOutline => "file_outline",
            Self::SymbolChunk => "symbol_chunk",
            Self::DocChunk => "doc_chunk",
            Self::ConfigChunk => "config_chunk",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChunkRecord {
    pub id: ChunkId,
    pub file_id: FileId,
    pub symbol_id: Option<SymbolId>,
    pub kind: ChunkKind,
    pub text: String,
    pub outline: Option<String>,
    pub content_hash: String,
    pub start_line: usize,
    pub end_line: usize,
    pub embedding: Option<Vec<f32>>,
    pub embedding_model: Option<String>,
    pub embedding_dim: Option<usize>,
    pub vector_set_id: Option<String>,
    pub updated_at: DateTime<Utc>,
}
