use async_trait::async_trait;
use chrono::{DateTime, Utc};
use oxide_core::error::Result;
use oxide_core::{
    ChunkRecord, FileRecord, MemoryId, MemoryKind, MemoryRecord, SymbolId, SymbolRecord,
};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchQuery {
    pub text: String,
    pub embedding: Option<Vec<f32>>,
    pub limit: usize,
    pub language: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchHit {
    pub file_path: String,
    pub symbol_name: Option<String>,
    pub start_line: usize,
    pub end_line: usize,
    pub score: f32,
    pub outline_or_signature: Option<String>,
    pub text: String,
}

#[async_trait]
pub trait ProjectStore: Send + Sync {
    async fn open<P: AsRef<Path> + Send + Sync>(path: P) -> Result<Self>
    where
        Self: Sized;

    async fn upsert_file(&self, file: &FileRecord) -> Result<()>;
    async fn upsert_symbol(&self, symbol: &SymbolRecord) -> Result<()>;
    async fn upsert_chunk(&self, chunk: &ChunkRecord) -> Result<()>;
    async fn upsert_doc_section(&self, section: &oxide_core::DocSection) -> Result<()>;
    async fn upsert_doc_reference(&self, edge: &oxide_core::DocReferenceEdge) -> Result<()>;
    async fn upsert_call_edge(&self, edge: &oxide_core::CallEdge) -> Result<()>;
    async fn upsert_import_edge(&self, edge: &oxide_core::ImportEdge) -> Result<()>;

    // Typed Semantic Memory (Memanto parity)
    async fn upsert_memory(&self, memory: &MemoryRecord, embedding: Option<Vec<f32>>) -> Result<()>;
    async fn get_memory(&self, id: &MemoryId) -> Result<Option<MemoryRecord>>;
    async fn recall_memories(
        &self,
        query_emb: Option<&[f32]>,
        kind: Option<MemoryKind>,
        tags: &[String],
        as_of: Option<DateTime<Utc>>,
        limit: usize,
    ) -> Result<Vec<MemoryRecord>>;
    async fn find_conflicts(
        &self,
        kind: MemoryKind,
        embedding: &[f32],
        threshold: f32,
    ) -> Result<Vec<MemoryRecord>>;
    async fn list_active_rules(&self) -> Result<Vec<MemoryRecord>>;
    async fn link_memory_to_symbol(
        &self,
        mem_id: &MemoryId,
        symbol_id: &SymbolId,
        relation: &str,
    ) -> Result<()>;

    async fn get_file_symbols(&self, file_path: &str) -> Result<Vec<SymbolRecord>>;
    async fn get_file_outline(&self, file_path: &str) -> Result<Option<String>>;

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchHit>>;
    async fn count_files(&self) -> Result<usize>;
    async fn count_symbols(&self) -> Result<usize>;
    async fn count_chunks(&self) -> Result<usize>;

    async fn export_surql(&self) -> Result<String>;
    async fn import_surql(&self, content: &str) -> Result<()>;
}
