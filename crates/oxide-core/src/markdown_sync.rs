use crate::error::Result;
use crate::id::{MemoryId, ProjectId};
use crate::memory::{MemoryKind, MemoryRecord, MemoryStatus};
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Bidirectional Markdown / Obsidian Vault Sync Engine.
/// Mirrors memories into human-editable `.oxide/memories/*.md` files and parses edits back into memory records.
pub struct MarkdownMemorySync;

impl MarkdownMemorySync {
    /// Exports memory records to category-specific markdown files in `.oxide/memories/`.
    pub fn export_to_dir<P: AsRef<Path>>(
        memories_dir: P,
        memories: &[MemoryRecord],
    ) -> Result<Vec<PathBuf>> {
        let base = memories_dir.as_ref();
        fs::create_dir_all(base)?;

        let mut categorized: HashMap<MemoryKind, Vec<&MemoryRecord>> = HashMap::new();
        for m in memories {
            categorized.entry(m.kind).or_default().push(m);
        }

        let mut generated_paths = Vec::new();

        for kind in MemoryKind::all() {
            let items = categorized.get(kind).cloned().unwrap_or_default();
            let filename = format!("{}.md", kind.as_str());
            let file_path = base.join(filename);

            let mut md = String::new();
            md.push_str(&format!(
                "# 🧠 Oxide Memories: {}\n\n",
                kind.as_str().to_uppercase()
            ));
            md.push_str("> Auto-synchronized with Oxide-Embed memory graph & vector store.\n");
            md.push_str(
                "> You can edit, add, or refine items directly in Obsidian or your IDE.\n\n",
            );

            if items.is_empty() {
                md.push_str("_No active records in this category._\n");
            } else {
                for item in items {
                    md.push_str(&format!("## {}\n", item.title));
                    md.push_str(&format!("- **ID**: `{}`\n", item.id));
                    md.push_str(&format!("- **Status**: `{}`\n", item.status));
                    md.push_str(&format!(
                        "- **Created**: `{}`\n",
                        item.created_at.to_rfc3339()
                    ));
                    if !item.tags.is_empty() {
                        md.push_str(&format!(
                            "- **Tags**: {}\n",
                            item.tags
                                .iter()
                                .map(|t| format!("#{t}"))
                                .collect::<Vec<_>>()
                                .join(" ")
                        ));
                    }
                    if let Some(ref sym) = item.symbol_ref {
                        md.push_str(&format!("- **Governs Symbol**: `{}`\n", sym));
                    }
                    if let Some(ref author) = item.author {
                        md.push_str(&format!("- **Author**: `{}`\n", author));
                    }
                    md.push_str("\n");
                    md.push_str(&item.content);
                    md.push_str("\n\n---\n\n");
                }
            }

            fs::write(&file_path, md)?;
            generated_paths.push(file_path);
        }

        Ok(generated_paths)
    }

    /// Parses an edited markdown file back into a list of MemoryRecords.
    pub fn import_from_file<P: AsRef<Path>>(
        project_id: ProjectId,
        path: P,
    ) -> Result<Vec<MemoryRecord>> {
        let content = fs::read_to_string(&path)?;
        let stem = path
            .as_ref()
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("fact");
        let fallback_kind = stem.parse().unwrap_or(MemoryKind::Fact);

        Self::parse_markdown_content(project_id, &content, fallback_kind)
    }

    /// Parses raw markdown text into MemoryRecords.
    pub fn parse_markdown_content(
        project_id: ProjectId,
        content: &str,
        default_kind: MemoryKind,
    ) -> Result<Vec<MemoryRecord>> {
        let mut records = Vec::new();
        let sections: Vec<&str> = content.split("\n## ").collect();

        for (idx, sec) in sections.into_iter().enumerate() {
            if idx == 0 && !content.starts_with("## ") {
                // Header section, skip
                continue;
            }

            let mut lines = sec.lines();
            let title = lines.next().unwrap_or("Untitled").trim().to_string();

            let mut mem_id: Option<MemoryId> = None;
            let mut status = MemoryStatus::Active;
            let mut tags = Vec::new();
            let mut symbol_ref = None;
            let mut author = None;
            let mut created_at = Utc::now();
            let mut body_lines = Vec::new();
            let mut in_body = false;

            for line in lines {
                let trimmed = line.trim();
                if trimmed == "---" {
                    continue;
                }

                if in_body {
                    body_lines.push(line);
                } else if let Some(rest) = trimmed.strip_prefix("- **ID**:") {
                    let raw = rest.trim().replace('`', "");
                    mem_id = Some(MemoryId::from_string(raw));
                } else if let Some(rest) = trimmed.strip_prefix("- **Status**:") {
                    let raw = rest.trim().replace('`', "");
                    if raw == "superseded" {
                        status = MemoryStatus::Superseded;
                    } else if raw == "contradicted" {
                        status = MemoryStatus::Contradicted;
                    }
                } else if let Some(rest) = trimmed.strip_prefix("- **Created**:") {
                    let raw = rest.trim().replace('`', "");
                    if let Ok(ts) = DateTime::parse_from_rfc3339(&raw) {
                        created_at = ts.with_timezone(&Utc);
                    }
                } else if let Some(rest) = trimmed.strip_prefix("- **Tags**:") {
                    for tag in rest.split_whitespace() {
                        let clean = tag.trim_start_matches('#');
                        if !clean.is_empty() {
                            tags.push(clean.to_string());
                        }
                    }
                } else if let Some(rest) = trimmed.strip_prefix("- **Governs Symbol**:") {
                    symbol_ref = Some(rest.trim().replace('`', ""));
                } else if let Some(rest) = trimmed.strip_prefix("- **Author**:") {
                    author = Some(rest.trim().replace('`', ""));
                } else if trimmed.is_empty() {
                    in_body = true;
                } else {
                    body_lines.push(line);
                    in_body = true;
                }
            }

            let body = body_lines.join("\n").trim().to_string();
            let final_content = if body.is_empty() { title.clone() } else { body };

            let record = MemoryRecord {
                id: mem_id.unwrap_or_else(MemoryId::new_v7),
                project_id: project_id.clone(),
                session_id: None,
                kind: default_kind,
                title,
                content: final_content,
                tags,
                symbol_ref,
                status,
                superseded_by: None,
                author,
                confidence: 1.0,
                source_hash: None,
                created_at,
                valid_until: None,
            };

            records.push(record);
        }

        Ok(records)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_markdown_export_and_import() {
        let pid = ProjectId::new_v7();
        let m1 = MemoryRecord::new(
            pid.clone(),
            MemoryKind::Decision,
            "Use pure no_std for STM32 drivers",
            "Hardware drivers must never allocate on heap.",
        )
        .with_tags(vec!["embedded".into(), "stm32".into()]);

        let tmp_dir = std::env::temp_dir().join(format!("oxide-md-sync-test-{}", pid));
        let exported = MarkdownMemorySync::export_to_dir(&tmp_dir, &[m1.clone()]).expect("export");
        assert!(!exported.is_empty());

        let dec_file = tmp_dir.join("decision.md");
        assert!(dec_file.exists());

        let imported =
            MarkdownMemorySync::import_from_file(pid, &dec_file).expect("import from file");
        assert_eq!(imported.len(), 1);
        assert_eq!(imported[0].title, "Use pure no_std for STM32 drivers");
        assert_eq!(imported[0].kind, MemoryKind::Decision);
        assert_eq!(imported[0].tags, vec!["embedded", "stm32"]);

        let _ = fs::remove_dir_all(&tmp_dir);
    }
}
