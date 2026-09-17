use crate::id::{FileId, ProjectId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FileRecord {
    pub id: FileId,
    pub project_id: ProjectId,
    pub relative_path: String,
    pub language: Option<String>,
    pub content_hash: Option<String>,
    pub size_bytes: u64,
    pub last_indexed_at: Option<DateTime<Utc>>,
}
