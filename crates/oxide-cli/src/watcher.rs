use notify::{Config, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use oxide_core::FileRecord;
use oxide_core::error::{OxideError, Result};
use oxide_core::id::{FileId, ProjectId};
use oxide_db::{ProjectStore, SurrealProjectStore};
use oxide_ml::Embedder;
use oxide_ml::candle_embedder::CandleBertEmbedder;
use oxide_parser::Language;
use oxide_parser::chunker::Chunker;
use oxide_parser::doc_linker::DocLinker;
use oxide_parser::languages::get_extractor;
use sha2::Digest;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::mpsc::channel;
use std::time::{Duration, Instant};

pub struct WorkspaceWatcher {
    workspace_dir: PathBuf,
}

impl WorkspaceWatcher {
    pub fn new(workspace_dir: impl Into<PathBuf>) -> Self {
        Self {
            workspace_dir: workspace_dir.into(),
        }
    }

    pub async fn run(&self) -> Result<()> {
        let oxide_dir = self.workspace_dir.join(".oxide");
        if !oxide_dir.exists() {
            return Err(OxideError::NotInitialized(self.workspace_dir.clone()));
        }

        let manifest = oxide_core::OxideManifest::load_from_dir(&oxide_dir)?;
        let project_id = manifest.project_id.clone();
        let db_path = oxide_core::resolve_db_path(&self.workspace_dir)?;

        let store = SurrealProjectStore::open(&db_path).await?;
        let embedder = CandleBertEmbedder::new_offline();
        let chunker = Chunker::new(512);

        println!(
            "👀 [oxide-watch] Monitoring workspace: {}",
            self.workspace_dir.display()
        );
        println!("⚡ Incremental sub-millisecond AST re-indexing active. Press Ctrl+C to exit.");

        let (tx, rx) = channel();
        let mut watcher = RecommendedWatcher::new(tx, Config::default())
            .map_err(|e| OxideError::Io(std::io::Error::other(e.to_string())))?;

        watcher
            .watch(&self.workspace_dir, RecursiveMode::Recursive)
            .map_err(|e| OxideError::Io(std::io::Error::other(e.to_string())))?;

        let mut last_processed = Instant::now();
        let mut pending_files: HashSet<PathBuf> = HashSet::new();
        let mut file_hash_cache: std::collections::HashMap<PathBuf, String> = std::collections::HashMap::new();

        loop {
            // Drain incoming events with short timeout
            while let Ok(event_res) = rx.recv_timeout(Duration::from_millis(200)) {
                if let Ok(event) = event_res
                    && matches!(event.kind, EventKind::Modify(_) | EventKind::Create(_))
                {
                    for path in event.paths {
                        if self.should_index(&path) {
                            pending_files.insert(path);
                        }
                    }
                }
            }

            if !pending_files.is_empty() && last_processed.elapsed() >= Duration::from_millis(300) {
                let batch: Vec<PathBuf> = pending_files.drain().collect();
                for file_path in batch {
                    let start = Instant::now();
                    match self
                        .reindex_file(&file_path, &project_id, &store, &embedder, &chunker, &mut file_hash_cache)
                        .await
                    {
                        Ok(Some(sym_count)) => {
                            let rel = file_path
                                .strip_prefix(&self.workspace_dir)
                                .unwrap_or(&file_path);
                            println!(
                                "⚡ [oxide-watch] Re-indexed {} ({} symbols) in {:.2} ms",
                                rel.display(),
                                sym_count,
                                start.elapsed().as_secs_f64() * 1000.0
                            );
                        }
                        Ok(None) => {
                            // File content hash unchanged (deduplicated)
                        }
                        Err(e) => {
                            eprintln!(
                                "❌ [oxide-watch] Error indexing {}: {}",
                                file_path.display(),
                                e
                            );
                        }
                    }
                }
                last_processed = Instant::now();
            }
        }
    }

    fn should_index(&self, path: &Path) -> bool {
        let str_rep = path.to_string_lossy();
        if str_rep.contains("/target/") || str_rep.contains("/.git/") {
            return false;
        }

        if str_rep.contains("/.oxide/memories/")
            && path.extension().and_then(|s| s.to_str()) == Some("md")
        {
            return true;
        }

        if str_rep.contains("/.oxide/") {
            return false;
        }

        let lang = Language::from_path(path);
        lang != Language::Unknown || path.extension().and_then(|s| s.to_str()) == Some("md")
    }

    async fn reindex_file(
        &self,
        path: &Path,
        project_id: &ProjectId,
        store: &SurrealProjectStore,
        embedder: &CandleBertEmbedder,
        chunker: &Chunker,
        file_hash_cache: &mut std::collections::HashMap<PathBuf, String>,
    ) -> Result<Option<usize>> {
        let str_rep = path.to_string_lossy();
        if str_rep.contains("/.oxide/memories/")
            && path.extension().and_then(|s| s.to_str()) == Some("md")
        {
            let records = oxide_core::markdown_sync::MarkdownMemorySync::import_from_file(
                project_id.clone(),
                path,
            )?;
            let count = records.len();
            store.sync_all_memories(&records).await?;
            return Ok(Some(count));
        }

        let content = std::fs::read_to_string(path)?;
        let content_hash = oxide_core::id::bytes_to_hex(&sha2::Sha256::digest(content.as_bytes()));

        if let Some(cached_hash) = file_hash_cache.get(path)
            && cached_hash == &content_hash
        {
            return Ok(None);
        }
        file_hash_cache.insert(path.to_path_buf(), content_hash.clone());

        let rel_path = path.strip_prefix(&self.workspace_dir).unwrap_or(path);
        let rel_str = rel_path.to_string_lossy().to_string();
        let fid = FileId::from_relative_path(&rel_str);

        let lang = Language::from_path(path);
        let lang_str = if lang != Language::Unknown {
            Some(format!("{:?}", lang).to_lowercase())
        } else {
            None
        };

        let file_rec = FileRecord {
            id: fid.clone(),
            project_id: project_id.clone(),
            relative_path: rel_str.clone(),
            language: lang_str,
            content_hash: Some(content_hash),
            size_bytes: content.len() as u64,
            last_indexed_at: Some(chrono::Utc::now()),
        };
        store.upsert_file(&file_rec).await?;

        let mut sym_count = 0;

        if lang != Language::Unknown {
            if let Some(ext) = get_extractor(lang) {
                let symbols = ext.extract_symbols(&fid, &content);
                sym_count = symbols.len();

                for sym in &symbols {
                    store.upsert_symbol(sym).await?;
                }

                let call_edges = ext.extract_call_edges(&fid, &content, &symbols);
                for edge in &call_edges {
                    let _ = store.upsert_call_edge(edge).await;
                }

                let import_edges = ext.extract_import_edges(&fid, &content);
                for edge in &import_edges {
                    let _ = store.upsert_import_edge(edge).await;
                }

                let chunks = chunker.chunk_file(&fid, &content, &symbols);
                for mut chunk in chunks {
                    if let Ok(emb) = embedder.embed(&chunk.text).await {
                        chunk.embedding = Some(emb);
                        chunk.embedding_dim = Some(embedder.dimension());
                        chunk.embedding_model = Some("bge-small-en-v1.5".into());
                    }
                    store.upsert_chunk(&chunk).await?;
                }
            }
        } else if path.extension().and_then(|s| s.to_str()) == Some("md") {
            let sections = DocLinker::extract_doc_sections(&rel_str, &content);
            for mut sec in sections {
                if let Ok(emb) = embedder.embed(&sec.content).await {
                    sec.embedding = Some(emb);
                }
                store.upsert_doc_section(&sec).await?;
            }
        }

        Ok(Some(sym_count))
    }
}
