use crate::id::SymbolId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DocSection {
    pub id: String,
    pub file_path: String,
    pub heading: String,
    pub content: String,
    pub embedding: Option<Vec<f32>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DocReferenceEdge {
    pub doc_section_id: String,
    pub symbol_id: SymbolId,
    pub context: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CognifyResult {
    pub files_indexed: usize,
    pub symbols_indexed: usize,
    pub chunks_indexed: usize,
    pub doc_sections_indexed: usize,
    pub doc_references_indexed: usize,
    pub call_edges_indexed: usize,
}
