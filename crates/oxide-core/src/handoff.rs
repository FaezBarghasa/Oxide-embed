use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandoffCheckpoint {
    pub session_id: String,
    pub timestamp: i64,
    pub active_goal: String,
    pub completed_tasks: Vec<String>,
    pub pending_tasks: Vec<String>,
    pub key_decisions: Vec<String>,
    pub modified_files: Vec<String>,
    pub next_action: String,
}

impl HandoffCheckpoint {
    pub fn new(session_id: &str, active_goal: &str, next_action: &str) -> Self {
        Self {
            session_id: session_id.to_string(),
            timestamp: chrono::Utc::now().timestamp(),
            active_goal: active_goal.to_string(),
            completed_tasks: Vec::new(),
            pending_tasks: Vec::new(),
            key_decisions: Vec::new(),
            modified_files: Vec::new(),
            next_action: next_action.to_string(),
        }
    }

    pub fn to_markdown(&self) -> String {
        let mut md = String::new();
        md.push_str("# Project Handover Status\n\n");
        md.push_str(&format!("- **Session ID**: {}\n", self.session_id));
        md.push_str(&format!(
            "- **Timestamp**: {}\n",
            chrono::DateTime::from_timestamp(self.timestamp, 0)
                .map(|d| d.to_rfc3339())
                .unwrap_or_default()
        ));
        md.push_str(&format!("- **Active Goal**: {}\n\n", self.active_goal));

        md.push_str("## Next Immediate Action\n");
        md.push_str(&format!("> {}\n\n", self.next_action));

        md.push_str("## Completed Tasks\n");
        if self.completed_tasks.is_empty() {
            md.push_str("- *(None)*\n");
        } else {
            for task in &self.completed_tasks {
                md.push_str(&format!("- [x] {}\n", task));
            }
        }
        md.push('\n');

        md.push_str("## Pending Tasks\n");
        if self.pending_tasks.is_empty() {
            md.push_str("- *(None)*\n");
        } else {
            for task in &self.pending_tasks {
                md.push_str(&format!("- [ ] {}\n", task));
            }
        }
        md.push('\n');

        md.push_str("## Key Decisions & Context\n");
        if self.key_decisions.is_empty() {
            md.push_str("- *(None)*\n");
        } else {
            for dec in &self.key_decisions {
                md.push_str(&format!("- {}\n", dec));
            }
        }
        md.push('\n');

        md.push_str("## Modified Files\n");
        if self.modified_files.is_empty() {
            md.push_str("- *(None)*\n");
        } else {
            for file in &self.modified_files {
                md.push_str(&format!("- `{}`\n", file));
            }
        }
        md.push('\n');

        md
    }

    pub fn save_to_file<P: AsRef<Path>>(&self, workspace_root: P) -> Result<PathBuf> {
        let oxide_dir = workspace_root.as_ref().join(".oxide");
        fs::create_dir_all(&oxide_dir)?;
        let status_path = oxide_dir.join("STATUS.md");
        fs::write(&status_path, self.to_markdown())?;
        Ok(status_path)
    }

    pub fn load_from_file<P: AsRef<Path>>(status_path: P) -> Result<String> {
        let content = fs::read_to_string(status_path)?;
        Ok(content)
    }

    pub fn parse_markdown(content: &str) -> Option<Self> {
        let mut session_id = "default".to_string();
        let mut active_goal = "Active Task".to_string();
        let mut next_action = "Continue implementation".to_string();
        let mut completed_tasks = Vec::new();
        let mut pending_tasks = Vec::new();
        let mut key_decisions = Vec::new();
        let mut modified_files = Vec::new();

        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("- **Session ID**:") {
                session_id = trimmed.trim_start_matches("- **Session ID**:").trim().to_string();
            } else if trimmed.starts_with("- **Active Goal**:") {
                active_goal = trimmed.trim_start_matches("- **Active Goal**:").trim().to_string();
            } else if trimmed.starts_with("> ") {
                next_action = trimmed.trim_start_matches("> ").trim().to_string();
            } else if trimmed.starts_with("- [x] ") {
                completed_tasks.push(trimmed.trim_start_matches("- [x] ").trim().to_string());
            } else if trimmed.starts_with("- [ ] ") {
                pending_tasks.push(trimmed.trim_start_matches("- [ ] ").trim().to_string());
            } else if trimmed.starts_with("- `") && trimmed.ends_with('`') {
                modified_files.push(trimmed.trim_matches(|c| c == '-' || c == '`' || c == ' ').to_string());
            } else if trimmed.starts_with("- ") && !trimmed.contains("**") {
                key_decisions.push(trimmed.trim_start_matches("- ").trim().to_string());
            }
        }

        Some(Self {
            session_id,
            timestamp: chrono::Utc::now().timestamp(),
            active_goal,
            completed_tasks,
            pending_tasks,
            key_decisions,
            modified_files,
            next_action,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_handoff_markdown() {
        let mut checkpoint =
            HandoffCheckpoint::new("session_123", "Complete Phase 8", "Run clippy and tests");
        checkpoint
            .completed_tasks
            .push("Implement SurrealDB 3 store".into());
        checkpoint.pending_tasks.push("Wire CLI commands".into());
        checkpoint
            .key_decisions
            .push("SurrealValue via serde_json".into());
        checkpoint
            .modified_files
            .push("crates/oxide-core/src/lib.rs".into());

        let md = checkpoint.to_markdown();
        assert!(md.contains("# Project Handover Status"));
        assert!(md.contains("session_123"));
        assert!(md.contains("- [x] Implement SurrealDB 3 store"));
        assert!(md.contains("- [ ] Wire CLI commands"));
        assert!(md.contains("SurrealValue via serde_json"));
    }
}
