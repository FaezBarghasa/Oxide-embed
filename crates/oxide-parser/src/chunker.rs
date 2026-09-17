use crate::outline::OutlineGenerator;
use chrono::Utc;
use oxide_core::id::{ChunkId, FileId, bytes_to_hex};
use oxide_core::{ChunkKind, ChunkRecord, SymbolRecord};
use sha2::{Digest, Sha256};

pub struct Chunker {
    chunk_max_tokens: usize,
}

impl Chunker {
    pub fn new(chunk_max_tokens: usize) -> Self {
        Self { chunk_max_tokens }
    }

    pub fn chunk_max_tokens(&self) -> usize {
        self.chunk_max_tokens
    }

    pub fn chunk_file(
        &self,
        file_id: &FileId,
        content: &str,
        symbols: &[SymbolRecord],
    ) -> Vec<ChunkRecord> {
        let mut chunks = Vec::new();
        let content_lines: Vec<&str> = content.lines().collect();
        let now = Utc::now();

        // 1. File Outline Chunk
        if !symbols.is_empty() {
            let outline = OutlineGenerator::generate_outline(symbols);
            let mut hasher = Sha256::new();
            hasher.update(outline.as_bytes());
            let hash = bytes_to_hex(&hasher.finalize());

            let chunk_id = ChunkId::new(file_id, None, ChunkKind::FileOutline.as_str(), &hash);
            chunks.push(ChunkRecord {
                id: chunk_id,
                file_id: file_id.clone(),
                symbol_id: None,
                kind: ChunkKind::FileOutline,
                text: outline.clone(),
                outline: Some(outline),
                content_hash: hash,
                start_line: 1,
                end_line: content_lines.len().max(1),
                embedding: None,
                embedding_model: None,
                embedding_dim: None,
                vector_set_id: None,
                updated_at: now,
            });
        }

        // 2. Symbol Chunks
        for sym in symbols {
            if sym.start_line > 0
                && sym.end_line >= sym.start_line
                && sym.start_line <= content_lines.len()
            {
                let start_idx = sym.start_line - 1;
                let end_idx = sym.end_line.min(content_lines.len());
                let symbol_text = content_lines[start_idx..end_idx].join("\n");

                let mut hasher = Sha256::new();
                hasher.update(symbol_text.as_bytes());
                let hash = bytes_to_hex(&hasher.finalize());

                let chunk_id = ChunkId::new(
                    file_id,
                    Some(&sym.id),
                    ChunkKind::SymbolChunk.as_str(),
                    &hash,
                );

                chunks.push(ChunkRecord {
                    id: chunk_id,
                    file_id: file_id.clone(),
                    symbol_id: Some(sym.id.clone()),
                    kind: ChunkKind::SymbolChunk,
                    text: symbol_text,
                    outline: sym.signature.clone(),
                    content_hash: hash,
                    start_line: sym.start_line,
                    end_line: sym.end_line,
                    embedding: None,
                    embedding_model: None,
                    embedding_dim: None,
                    vector_set_id: None,
                    updated_at: now,
                });
            }
        }

        // 3. Fallback file summary if no symbols extracted
        if chunks.is_empty() && !content.trim().is_empty() {
            let mut hasher = Sha256::new();
            hasher.update(content.as_bytes());
            let hash = bytes_to_hex(&hasher.finalize());

            let chunk_id = ChunkId::new(file_id, None, ChunkKind::FileSummary.as_str(), &hash);
            chunks.push(ChunkRecord {
                id: chunk_id,
                file_id: file_id.clone(),
                symbol_id: None,
                kind: ChunkKind::FileSummary,
                text: content.to_string(),
                outline: None,
                content_hash: hash,
                start_line: 1,
                end_line: content_lines.len().max(1),
                embedding: None,
                embedding_model: None,
                embedding_dim: None,
                vector_set_id: None,
                updated_at: now,
            });
        }

        chunks
    }
}
