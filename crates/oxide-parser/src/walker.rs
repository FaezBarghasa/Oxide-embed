use std::fs;
use std::path::{Path, PathBuf};
use ignore::WalkBuilder;
use sha2::{Digest, Sha256};
use oxide_core::error::Result;
use oxide_core::id::{normalize_relative_path, FileId, ProjectId};
use oxide_core::FileRecord;
use crate::language::Language;

#[derive(Debug)]
pub struct ProjectWalker {
    project_root: PathBuf,
    project_id: ProjectId,
    max_file_kb: u64,
    custom_ignores: Vec<String>,
}

impl ProjectWalker {
    pub fn new<P: AsRef<Path>>(
        project_root: P,
        project_id: ProjectId,
        max_file_kb: u64,
        custom_ignores: Vec<String>,
    ) -> Self {
        Self {
            project_root: project_root.as_ref().to_path_buf(),
            project_id,
            max_file_kb,
            custom_ignores,
        }
    }

    pub fn walk(&self) -> Result<Vec<(FileRecord, String)>> {
        let mut builder = WalkBuilder::new(&self.project_root);
        builder.hidden(false);
        builder.git_ignore(true);
        builder.git_global(true);
        builder.git_exclude(true);

        // Always ignore .oxide directory and custom ignores
        let mut custom_ignore_path = self.project_root.join(".oxideignore");
        if custom_ignore_path.exists() {
            builder.add_custom_ignore_filename(".oxideignore");
        }

        let mut results = Vec::new();
        let max_bytes = self.max_file_kb * 1024;

        for result in builder.build() {
            let entry = match result {
                Ok(e) => e,
                Err(_) => continue,
            };

            let path = entry.path();
            if !path.is_file() {
                continue;
            }

            // Check if within .oxide
            if let Ok(rel) = path.strip_prefix(&self.project_root) {
                let rel_str = normalize_relative_path(rel);
                if rel_str.starts_with(".oxide") || rel_str.starts_with(".git") {
                    continue;
                }

                // Check custom ignore list strings
                let should_ignore = self.custom_ignores.iter().any(|ign| {
                    rel_str.starts_with(ign.trim_end_matches('/'))
                });
                if should_ignore {
                    continue;
                }

                let metadata = match fs::metadata(path) {
                    Ok(m) => m,
                    Err(_) => continue,
                };

                let size_bytes = metadata.len();
                if size_bytes > max_bytes {
                    continue;
                }

                // Read file content
                let content = match fs::read_to_string(path) {
                    Ok(c) => c,
                    Err(_) => continue, // Ignore binary or non-utf8 files
                };

                // Compute hash
                let mut hasher = Sha256::new();
                hasher.update(content.as_bytes());
                let content_hash = format!("{:x}", hasher.finalize());

                let language = Language::from_path(path);
                let file_id = FileId::from_relative_path(rel);

                let record = FileRecord {
                    id: file_id,
                    project_id: self.project_id.clone(),
                    relative_path: rel_str,
                    language: Some(language.as_str().to_string()),
                    content_hash: Some(content_hash),
                    size_bytes,
                    last_indexed_at: None,
                };

                results.push((record, content));
            }
        }

        Ok(results)
    }
}
