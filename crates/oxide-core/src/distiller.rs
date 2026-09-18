use crate::id::ProjectId;
use crate::memory::{MemoryKind, MemoryRecord};

/// Distills structured semantic memories from conventional git commits and transcripts.
pub struct MemoryDistiller;

impl MemoryDistiller {
    /// Parses a raw git log / commit message output into high-value MemoryRecords.
    ///
    /// Recognizes conventional commit prefixes:
    /// - `feat:` -> `Decision` / `Goal`
    /// - `fix:` -> `Error` / `Learning`
    /// - `refactor:` -> `Decision` / `Architecture`
    /// - `perf:` -> `Learning` / `Observation`
    pub fn distill_commit(
        project_id: ProjectId,
        commit_hash: &str,
        author: &str,
        message: &str,
    ) -> Option<MemoryRecord> {
        let trimmed = message.trim();
        if trimmed.is_empty() {
            return None;
        }

        let first_line = trimmed.lines().next().unwrap_or(trimmed);

        let (kind, title, content) = if let Some(rest) = first_line.strip_prefix("feat:") {
            (
                MemoryKind::Decision,
                rest.trim().to_string(),
                format!("Commit {}: {}", commit_hash, trimmed),
            )
        } else if let Some(rest) = first_line.strip_prefix("fix:") {
            (
                MemoryKind::Error,
                rest.trim().to_string(),
                format!("Resolved issue in commit {}: {}", commit_hash, trimmed),
            )
        } else if let Some(rest) = first_line.strip_prefix("refactor:") {
            (
                MemoryKind::Decision,
                rest.trim().to_string(),
                format!("Refactoring in commit {}: {}", commit_hash, trimmed),
            )
        } else if let Some(rest) = first_line.strip_prefix("perf:") {
            (
                MemoryKind::Learning,
                rest.trim().to_string(),
                format!("Performance optimization in {}: {}", commit_hash, trimmed),
            )
        } else if let Some(rest) = first_line.strip_prefix("docs:") {
            (
                MemoryKind::Fact,
                rest.trim().to_string(),
                format!("Documentation update in {}: {}", commit_hash, trimmed),
            )
        } else {
            return None;
        };

        let mut record = MemoryRecord::new(project_id, kind, title, content);
        record.author = Some(format!("git:{}", author));
        record.source_hash = Some(commit_hash.to_string());
        record.tags = vec!["distilled".into(), "git".into(), kind.as_str().into()];

        Some(record)
    }

    /// Distills multiple commit blocks separated by standard git delimiter.
    pub fn distill_git_log(project_id: ProjectId, raw_log: &str) -> Vec<MemoryRecord> {
        let mut memories = Vec::new();

        // Expecting format: "HASH|AUTHOR|SUBJECT\nBODY\n---COMMIT_END---"
        for block in raw_log.split("---COMMIT_END---") {
            let block = block.trim();
            if block.is_empty() {
                continue;
            }

            let mut lines = block.lines();
            if let Some(header) = lines.next() {
                let parts: Vec<&str> = header.splitn(3, '|').collect();
                if parts.len() == 3 {
                    let hash = parts[0].trim();
                    let author = parts[1].trim();
                    let subject = parts[2].trim();
                    let body = lines.collect::<Vec<_>>().join("\n");
                    let full_msg = if body.is_empty() {
                        subject.to_string()
                    } else {
                        format!("{}\n{}", subject, body)
                    };

                    if let Some(mem) =
                        Self::distill_commit(project_id.clone(), hash, author, &full_msg)
                    {
                        memories.push(mem);
                    }
                }
            }
        }

        memories
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_distill_conventional_commits() {
        let pid = ProjectId::new_v7();
        let commit = MemoryDistiller::distill_commit(
            pid.clone(),
            "abc1234",
            "Faez",
            "feat: implement SPI DMA transfer ring buffer",
        );
        assert!(commit.is_some());
        let c = commit.unwrap();
        assert_eq!(c.kind, MemoryKind::Decision);
        assert_eq!(c.title, "implement SPI DMA transfer ring buffer");
        assert_eq!(c.author.as_deref(), Some("git:Faez"));
        assert_eq!(c.source_hash.as_deref(), Some("abc1234"));

        let fix = MemoryDistiller::distill_commit(
            pid,
            "def5678",
            "Faez",
            "fix: resolve stack overflow in RTIC interrupt handler",
        );
        assert!(fix.is_some());
        assert_eq!(fix.unwrap().kind, MemoryKind::Error);
    }
}
