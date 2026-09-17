use crate::error::Result;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CondensedOutput {
    pub command: String,
    pub exit_code: i32,
    pub condensed_text: String,
    pub original_bytes: usize,
    pub condensed_bytes: usize,
    pub cached_log_path: Option<String>,
}

pub struct TerminalCondenser {
    threshold_bytes: usize,
}

impl Default for TerminalCondenser {
    fn default() -> Self {
        Self {
            threshold_bytes: 4096,
        }
    }
}

impl TerminalCondenser {
    pub fn new(threshold_bytes: usize) -> Self {
        Self { threshold_bytes }
    }

    pub fn condense<P: AsRef<Path>>(
        &self,
        cache_dir: P,
        command: &str,
        stdout: &str,
        stderr: &str,
        exit_code: i32,
    ) -> Result<CondensedOutput> {
        let combined = format!("STDOUT:\n{}\n\nSTDERR:\n{}", stdout, stderr);
        let original_bytes = combined.len();

        if original_bytes <= self.threshold_bytes && exit_code == 0 {
            return Ok(CondensedOutput {
                command: command.to_string(),
                exit_code,
                condensed_text: combined.clone(),
                original_bytes,
                condensed_bytes: original_bytes,
                cached_log_path: None,
            });
        }

        // Cache full raw output to disk
        let mut hasher = Sha256::new();
        hasher.update(combined.as_bytes());
        let result = hasher.finalize();
        let hash: String = result.iter().map(|b| format!("{:02x}", b)).collect();
        let log_filename = format!("{}-{}.log", chrono::Utc::now().timestamp(), &hash[..8]);
        let log_dir = cache_dir.as_ref().join("cache").join("bash");
        fs::create_dir_all(&log_dir)?;
        let log_path = log_dir.join(log_filename);
        fs::write(&log_path, &combined)?;

        // Extract key failure and error blocks
        let mut highlights = Vec::new();
        for line in combined.lines() {
            let lower = line.to_lowercase();
            if lower.contains("error:")
                || lower.contains("failed")
                || lower.contains("failure")
                || lower.contains("panic")
                || lower.contains("assertion failed")
                || lower.contains("warning:")
                || lower.contains("exception")
            {
                highlights.push(line);
            }
        }

        let summary = if highlights.is_empty() {
            // Keep head and tail lines
            let all_lines: Vec<&str> = combined.lines().collect();
            let head = all_lines
                .iter()
                .take(15)
                .cloned()
                .collect::<Vec<_>>()
                .join("\n");
            let tail = all_lines
                .iter()
                .rev()
                .take(15)
                .rev()
                .cloned()
                .collect::<Vec<_>>()
                .join("\n");
            format!(
                "{}\n\n[... {} lines condensed ...]\n\n{}",
                head,
                all_lines.len().saturating_sub(30),
                tail
            )
        } else {
            let count = highlights.len();
            let limited = highlights
                .into_iter()
                .take(40)
                .collect::<Vec<_>>()
                .join("\n");
            format!(
                "Key error/warning highlights ({} total found):\n{}",
                count, limited
            )
        };

        let condensed_text = format!(
            "[Output Condenser: {} bytes reduced to {} bytes | Exit Code: {}]\n{}\n\n[Full raw output cached at: {}]",
            original_bytes,
            summary.len(),
            exit_code,
            summary,
            log_path.display()
        );

        let condensed_bytes = condensed_text.len();

        Ok(CondensedOutput {
            command: command.to_string(),
            exit_code,
            condensed_text,
            original_bytes,
            condensed_bytes,
            cached_log_path: Some(log_path.to_string_lossy().to_string()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_condenser_small_output() {
        let condenser = TerminalCondenser::new(1024);
        let temp_dir = std::env::temp_dir().join("oxide_test_condenser_small");
        let result = condenser
            .condense(&temp_dir, "cargo check", "Finished successfully", "", 0)
            .unwrap();
        assert_eq!(result.exit_code, 0);
        assert!(result.cached_log_path.is_none());
        assert!(result.condensed_text.contains("Finished successfully"));
    }

    #[test]
    fn test_condenser_large_error_output() {
        let condenser = TerminalCondenser::new(50);
        let temp_dir = std::env::temp_dir().join("oxide_test_condenser_large");
        let big_stderr =
            "line 1\nerror: something broke badly\nline 3\nline 4\npanic: test panic\n".repeat(20);
        let result = condenser
            .condense(&temp_dir, "cargo build", "", &big_stderr, 101)
            .unwrap();
        assert_eq!(result.exit_code, 101);
        assert!(result.cached_log_path.is_some());
        assert!(result.condensed_text.contains("Output Condenser:"));
        assert!(
            result
                .condensed_text
                .contains("error: something broke badly")
        );
    }
}
