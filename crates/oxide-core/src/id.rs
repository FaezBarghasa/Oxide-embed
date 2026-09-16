use std::fmt::{self, Write};
use std::path::Path;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProjectId(pub Uuid);

impl ProjectId {
    pub fn new_v7() -> Self {
        Self(Uuid::now_v7())
    }
}

impl fmt::Display for ProjectId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WorkspaceId(pub Uuid);

impl WorkspaceId {
    pub fn new_v7() -> Self {
        Self(Uuid::now_v7())
    }
}

impl fmt::Display for WorkspaceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FileId(pub String);

impl FileId {
    pub fn from_relative_path<P: AsRef<Path>>(path: P) -> Self {
        let normalized = normalize_relative_path(path.as_ref());
        let mut hasher = Sha256::new();
        hasher.update(normalized.as_bytes());
        let hash = bytes_to_hex(&hasher.finalize());
        Self(hash)
    }
}

impl fmt::Display for FileId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SymbolId(pub String);

impl SymbolId {
    pub fn new(file_id: &FileId, qualified_name: &str) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(file_id.0.as_bytes());
        hasher.update(b"::");
        hasher.update(qualified_name.as_bytes());
        Self(bytes_to_hex(&hasher.finalize()))
    }
}

impl fmt::Display for SymbolId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ChunkId(pub String);

impl ChunkId {
    pub fn new(
        file_id: &FileId,
        symbol_id: Option<&SymbolId>,
        chunk_kind: &str,
        content_hash: &str,
    ) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(file_id.0.as_bytes());
        if let Some(sym) = symbol_id {
            hasher.update(sym.0.as_bytes());
        }
        hasher.update(chunk_kind.as_bytes());
        hasher.update(content_hash.as_bytes());
        Self(bytes_to_hex(&hasher.finalize()))
    }
}

impl fmt::Display for ChunkId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

pub fn normalize_relative_path(path: &Path) -> String {
    path.components()
        .map(|c| c.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

pub fn bytes_to_hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        let _ = write!(s, "{:02x}", b);
    }
    s
}
