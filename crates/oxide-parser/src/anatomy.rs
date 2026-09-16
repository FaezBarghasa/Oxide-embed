use std::collections::HashMap;
use std::fmt::Write;
use std::fs;
use std::path::Path;
use serde::{Deserialize, Serialize};
use oxide_core::error::Result;
use oxide_core::id::FileId;
use crate::language::Language;
use crate::languages::get_extractor;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnatomySymbolEntry {
    pub name: String,
    pub qualified_name: String,
    pub kind: String,
    pub start_line: usize,
    pub end_line: usize,
    pub signature: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnatomyFileEntry {
    pub relative_path: String,
    pub language: String,
    pub size_bytes: u64,
    pub line_count: usize,
    pub content_hash: String,
    pub symbols: Vec<AnatomySymbolEntry>,
    pub last_scanned_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AnatomyIndex {
    pub project_name: String,
    pub root_path: String,
    pub files: HashMap<String, AnatomyFileEntry>,
    pub total_symbols: usize,
    pub last_updated: chrono::DateTime<chrono::Utc>,
}

impl AnatomyIndex {
    pub fn new<P: AsRef<Path>>(project_name: &str, root_path: P) -> Self {
        Self {
            project_name: project_name.to_string(),
            root_path: root_path.as_ref().to_string_lossy().to_string(),
            files: HashMap::new(),
            total_symbols: 0,
            last_updated: chrono::Utc::now(),
        }
    }

    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = fs::read_to_string(path)?;
        let index: Self = serde_json::from_str(&content)?;
        Ok(index)
    }

    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        if let Some(parent) = path.as_ref().parent() {
            fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(self)?;
        fs::write(path, content)?;
        Ok(())
    }

    pub fn insert_or_update(&mut self, entry: AnatomyFileEntry) {
        self.files.insert(entry.relative_path.clone(), entry);
        self.recalculate_totals();
    }

    pub fn recalculate_totals(&mut self) {
        self.total_symbols = self.files.values().map(|f| f.symbols.len()).sum();
        self.last_updated = chrono::Utc::now();
    }

    pub fn find_symbol(&self, query: &str) -> Vec<(&AnatomyFileEntry, &AnatomySymbolEntry)> {
        let q_lower = query.to_lowercase();
        let mut results = Vec::new();
        for file in self.files.values() {
            for sym in &file.symbols {
                if sym.name.to_lowercase().contains(&q_lower)
                    || sym.qualified_name.to_lowercase().contains(&q_lower)
                {
                    results.push((file, sym));
                }
            }
        }
        results
    }

    pub fn find_file(&self, query: &str) -> Option<&AnatomyFileEntry> {
        let q_lower = query.to_lowercase();
        self.files.get(query).or_else(|| {
            self.files
                .values()
                .find(|f| f.relative_path.to_lowercase().contains(&q_lower))
        })
    }

    pub fn summary_map(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!(
            "# Project Anatomy Map: {}\nTotal Files: {} | Total Symbols: {}\n\n",
            self.project_name,
            self.files.len(),
            self.total_symbols
        ));

        let mut sorted_files: Vec<_> = self.files.values().collect();
        sorted_files.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));

        for file in sorted_files {
            out.push_str(&format!(
                "- **{}** ({} lines, {})\n",
                file.relative_path, file.line_count, file.language
            ));
            for sym in &file.symbols {
                out.push_str(&format!(
                    "  - [{}] {} (L{}-L{})\n",
                    sym.kind, sym.qualified_name, sym.start_line, sym.end_line
                ));
            }
        }
        out
    }
}

pub struct AnatomyScanner;

impl AnatomyScanner {
    pub fn scan_project<P: AsRef<Path>>(
        root: P,
        existing_index: Option<AnatomyIndex>,
    ) -> Result<AnatomyIndex> {
        let root_path = root.as_ref();
        let project_name = root_path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("project");

        let mut index = existing_index.unwrap_or_else(|| AnatomyIndex::new(project_name, root_path));

        let walker = ignore::WalkBuilder::new(root_path)
            .hidden(false)
            .git_ignore(true)
            .build();

        for result in walker {
            let entry = match result {
                Ok(e) => e,
                Err(_) => continue,
            };

            let path = entry.path();
            if !path.is_file() {
                continue;
            }

            let rel_path = match path.strip_prefix(root_path) {
                Ok(p) => p.to_string_lossy().to_string(),
                Err(_) => continue,
            };

            if rel_path.starts_with(".git/") || rel_path.starts_with(".oxide/") {
                continue;
            }

            let lang = Language::from_path(path);
            if lang.is_binary() || lang == Language::Unknown {
                continue;
            }

            let content = match fs::read_to_string(path) {
                Ok(c) => c,
                Err(_) => continue,
            };

            use sha2::{Digest, Sha256};
            let mut hasher = Sha256::new();
            hasher.update(content.as_bytes());
            let hash_bytes = hasher.finalize();
            let mut hash = String::with_capacity(64);
            for b in hash_bytes {
                let _ = write!(hash, "{:02x}", b);
            }

            // Check if unchanged in existing index
            if let Some(existing) = index.files.get(&rel_path) {
                if existing.content_hash == hash {
                    continue;
                }
            }

            let line_count = content.lines().count();
            let file_id = FileId::from_relative_path(&rel_path);

            let symbols = if let Some(extractor) = get_extractor(lang) {
                let records = extractor.extract_symbols(&file_id, &content);
                records
                    .into_iter()
                    .map(|r| AnatomySymbolEntry {
                        name: r.name,
                        qualified_name: r.qualified_name.unwrap_or_else(|| "".to_string()),
                        kind: r.kind.as_str().to_string(),
                        start_line: r.start_line,
                        end_line: r.end_line,
                        signature: r.signature,
                    })
                    .collect()
            } else {
                Vec::new()
            };

            let file_entry = AnatomyFileEntry {
                relative_path: rel_path,
                language: lang.as_str().to_string(),
                size_bytes: content.len() as u64,
                line_count,
                content_hash: hash,
                symbols,
                last_scanned_at: chrono::Utc::now(),
            };

            index.insert_or_update(file_entry);
        }

        index.recalculate_totals();
        Ok(index)
    }
}
