use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionReadRecord {
    pub file_path: String,
    pub content_hash: String,
    pub read_count: usize,
    pub last_read_timestamp: i64,
    pub byte_size: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReadGuardDecision {
    FullRead {
        content: String,
        is_first_read: bool,
    },
    DuplicateSuppressed {
        file_path: String,
        content_hash: String,
        read_count: usize,
        byte_size: usize,
        stub_message: String,
    },
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct SessionReadGuard {
    reads: HashMap<String, SessionReadRecord>,
}

impl SessionReadGuard {
    pub fn new() -> Self {
        Self {
            reads: HashMap::new(),
        }
    }

    pub fn compute_hash(content: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        let result = hasher.finalize();
        result.iter().map(|b| format!("{:02x}", b)).collect()
    }

    pub fn process_read<P: AsRef<Path>>(
        &mut self,
        file_path: P,
        content: &str,
        symbol_summary: Option<&str>,
        force_full: bool,
    ) -> ReadGuardDecision {
        let path_str = file_path.as_ref().to_string_lossy().to_string();
        let hash = Self::compute_hash(content);
        let now = chrono::Utc::now().timestamp();
        let byte_size = content.len();

        if !force_full
            && let Some(record) = self.reads.get_mut(&path_str)
            && record.content_hash == hash
        {
            record.read_count += 1;
            record.last_read_timestamp = now;

            let summary_text = symbol_summary.unwrap_or("No symbol summary provided.");
            let stub_message = format!(
                "[Pre-Read Guard: File '{}' is unchanged since previous read (Hash: {}, Read Count: {}, {} bytes).\nSuppressed redundant full file dump to conserve tokens.\nAST Symbol Structure:\n{}]",
                path_str,
                &hash[..8],
                record.read_count,
                byte_size,
                summary_text
            );

            return ReadGuardDecision::DuplicateSuppressed {
                file_path: path_str,
                content_hash: hash,
                read_count: record.read_count,
                byte_size,
                stub_message,
            };
        }

        // Record new or modified read
        let is_first_read = !self.reads.contains_key(&path_str);
        self.reads.insert(
            path_str,
            SessionReadRecord {
                file_path: file_path.as_ref().to_string_lossy().to_string(),
                content_hash: hash,
                read_count: 1,
                last_read_timestamp: now,
                byte_size,
            },
        );

        ReadGuardDecision::FullRead {
            content: content.to_string(),
            is_first_read,
        }
    }

    pub fn clear(&mut self) {
        self.reads.clear();
    }

    pub fn get_reads(&self) -> &HashMap<String, SessionReadRecord> {
        &self.reads
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_guard_suppression() {
        let mut guard = SessionReadGuard::new();
        let file_path = "src/main.rs";
        let content = "fn main() { println!(\"hello\"); }";

        // First read: full content
        let d1 = guard.process_read(file_path, content, Some("- fn main"), false);
        match d1 {
            ReadGuardDecision::FullRead {
                content: c,
                is_first_read,
            } => {
                assert_eq!(c, content);
                assert!(is_first_read);
            }
            _ => panic!("Expected FullRead on first access"),
        }

        // Second read: identical content -> suppressed
        let d2 = guard.process_read(file_path, content, Some("- fn main"), false);
        match d2 {
            ReadGuardDecision::DuplicateSuppressed {
                read_count,
                stub_message,
                ..
            } => {
                assert_eq!(read_count, 2);
                assert!(stub_message.contains("Pre-Read Guard"));
                assert!(stub_message.contains("- fn main"));
            }
            _ => panic!("Expected DuplicateSuppressed on second identical access"),
        }

        // Third read with modified content -> full read
        let modified = "fn main() { println!(\"world\"); }";
        let d3 = guard.process_read(file_path, modified, Some("- fn main"), false);
        match d3 {
            ReadGuardDecision::FullRead {
                content: c,
                is_first_read,
            } => {
                assert_eq!(c, modified);
                assert!(!is_first_read);
            }
            _ => panic!("Expected FullRead on modified content"),
        }
    }
}
