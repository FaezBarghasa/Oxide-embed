use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::id::ModelId;

/// A standing mental model synthesis answering a recurring codebase question.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MentalModel {
    pub id: ModelId,
    pub topic: String,
    pub question: String,
    pub standing_answer: String,
    #[serde(default)]
    pub referenced_symbols: Vec<String>,
    #[serde(default)]
    pub referenced_files: Vec<String>,
    #[serde(default)]
    pub source_memory_ids: Vec<String>,
    #[serde(default)]
    pub file_hashes_snapshot: HashMap<String, String>,
    #[serde(default)]
    pub is_stale: bool,
    pub last_refreshed_at: DateTime<Utc>,
    #[serde(default)]
    pub embedding: Option<Vec<f32>>,
}

impl MentalModel {
    pub fn new(
        topic: impl Into<String>,
        question: impl Into<String>,
        standing_answer: impl Into<String>,
    ) -> Self {
        Self {
            id: ModelId::new_v7(),
            topic: topic.into(),
            question: question.into(),
            standing_answer: standing_answer.into(),
            referenced_symbols: Vec::new(),
            referenced_files: Vec::new(),
            source_memory_ids: Vec::new(),
            file_hashes_snapshot: HashMap::new(),
            is_stale: false,
            last_refreshed_at: Utc::now(),
            embedding: None,
        }
    }

    /// Checks if this mental model has become stale given the current file hashes.
    pub fn is_stale_against_hashes(&self, current_hashes: &HashMap<String, String>) -> bool {
        if self.is_stale {
            return true;
        }

        for (file_path, old_hash) in &self.file_hashes_snapshot {
            match current_hashes.get(file_path) {
                Some(current_hash) if current_hash != old_hash => return true,
                None => return true, // File deleted or missing
                _ => {}
            }
        }

        false
    }

    pub fn mark_stale(&mut self) {
        self.is_stale = true;
    }

    pub fn to_markdown(&self) -> String {
        let mut md = String::new();
        md.push_str(&format!("# Mental Model: {}\n\n", self.topic));
        md.push_str(&format!("**Standing Question:** {}\n\n", self.question));
        if self.is_stale {
            md.push_str("> ⚠️ *Note: This mental model is flagged as STALE due to recent code changes.*\n\n");
        }
        md.push_str("## Standing Answer\n\n");
        md.push_str(&self.standing_answer);
        md.push_str("\n\n");

        if !self.referenced_symbols.is_empty() {
            md.push_str("## Referenced Symbols\n");
            for sym in &self.referenced_symbols {
                md.push_str(&format!("- `{}`\n", sym));
            }
            md.push_str("\n");
        }

        if !self.referenced_files.is_empty() {
            md.push_str("## Referenced Files\n");
            for file in &self.referenced_files {
                md.push_str(&format!("- `{}`\n", file));
            }
            md.push_str("\n");
        }

        md.push_str(&format!(
            "*Last Refreshed: {} UTC*\n",
            self.last_refreshed_at.format("%Y-%m-%d %H:%M:%S")
        ));
        md
    }
}
