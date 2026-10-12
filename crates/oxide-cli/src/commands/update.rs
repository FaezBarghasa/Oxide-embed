use oxide_core::error::{OxideError, Result};
use oxide_core::{CrateCatalogEntry, MemoryDistiller, OxideConfig, OxideManifest};
use oxide_db::{ProjectStore, SurrealProjectStore};
use oxide_ml::device::{device_name, optimal_batch_size};
use oxide_ml::{CandleBertEmbedder, Embedder};
use oxide_parser::languages::get_extractor;
use oxide_parser::{AnatomyScanner, Chunker, DocLinker, Language, ProjectWalker};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::Instant;
use tokio::process::Command;

pub async fn handle_update(
    project_root: &Path,
    device_override: Option<&str>,
    batch_size_override: Option<usize>,
    scan_crates: bool,
    distill_git: bool,
) -> Result<()> {
    let oxide_dir = crate::commands::init::ensure_initialized(project_root, None)?;
    let manifest = OxideManifest::load_from_dir(&oxide_dir)?;
    let config = OxideConfig::load_from_dir(&oxide_dir)?;
    let db_path = manifest.db_path(&oxide_dir);
    let store = SurrealProjectStore::open(&db_path).await?;

    let start_time = Instant::now();
    println!(
        "Incrementally updating context & agent intelligence for '{}'...",
        manifest.project_name
    );

    let device_pref = device_override.unwrap_or(&config.embedding.device);
    let embedder = CandleBertEmbedder::new_offline_with_device(device_pref);
    let dev = embedder.device();
    let dev_label = device_name(dev);
    let batch_size = batch_size_override
        .unwrap_or_else(|| optimal_batch_size(dev))
        .max(1);

    println!("Accelerator: {} | Batch size: {}", dev_label, batch_size);

    // ---------------------------------------------------------
    // 1. TIER 1: Incremental Project Context Update
    // ---------------------------------------------------------
    let walker = ProjectWalker::new(
        project_root,
        manifest.project_id.clone(),
        config.index.max_file_kb,
        config.index.ignore.clone(),
    );

    let disk_files = tokio::task::spawn_blocking(move || walker.walk())
        .await
        .map_err(|e| OxideError::Other(e.to_string()))??;
    let mut disk_by_rel_path: HashMap<String, (&oxide_core::FileRecord, &String)> = HashMap::new();
    for (f, c) in &disk_files {
        disk_by_rel_path.insert(f.relative_path.clone(), (f, c));
    }

    // Query existing files in DB
    let mut db_files: HashMap<String, (String, String)> = HashMap::new(); // rel_path -> (file_id, hash)
    let sql = "SELECT id, relative_path, content_hash FROM file;";
    if let Ok(mut res) = store.db().query(sql).await {
        let raw_rows: Vec<serde_json::Value> = res.take(0).unwrap_or_default();
        for r in raw_rows {
            let id_str = match r.get("id") {
                Some(serde_json::Value::String(s)) => {
                    s.strip_prefix("file:").unwrap_or(s).to_string()
                }
                Some(val) => val.to_string(),
                None => continue,
            };
            if let Some(rel_path) = r.get("relative_path").and_then(|v| v.as_str()) {
                let hash = r
                    .get("content_hash")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                db_files.insert(rel_path.to_string(), (id_str, hash));
            }
        }
    }

    // Identify Deleted Files
    let mut files_removed = 0;
    for (rel_path, (file_id, _)) in &db_files {
        if !disk_by_rel_path.contains_key(rel_path) {
            store.delete_file_artifacts(file_id, rel_path).await?;
            let _ = store
                .mark_mental_models_stale_for_files(std::slice::from_ref(rel_path))
                .await;
            files_removed += 1;
        }
    }

    // Identify Added, Modified, and Unchanged Files
    let mut files_added = 0;
    let mut files_updated = 0;
    let mut files_unchanged = 0;
    let mut active_files_to_index = Vec::new();

    for (file_record, content) in &disk_files {
        match db_files.get(&file_record.relative_path) {
            None => {
                // Brand new file
                files_added += 1;
                active_files_to_index.push((file_record, content));
            }
            Some((file_id, prev_hash)) => {
                if file_record.content_hash.as_ref() == Some(prev_hash) {
                    // Unchanged! Skip parsing and re-embedding
                    files_unchanged += 1;
                } else {
                    // Modified file: clean old artifacts before replacing
                    files_updated += 1;
                    store
                        .delete_file_artifacts(file_id, &file_record.relative_path)
                        .await?;
                    let _ = store
                        .mark_mental_models_stale_for_files(std::slice::from_ref(
                            &file_record.relative_path,
                        ))
                        .await;
                    active_files_to_index.push((file_record, content));
                }
            }
        }
    }

    let chunker = Chunker::new(config.index.chunk_max_tokens);
    let mut total_symbols = 0;
    let mut total_chunks = 0;
    let mut total_call_edges = 0;
    let mut total_import_edges = 0;
    let mut all_project_symbols = Vec::new();
    let mut all_chunks = Vec::new();

    for (file_record, content) in &active_files_to_index {
        store.upsert_file(file_record).await?;

        let lang = Language::from_path(&file_record.relative_path);
        let (symbols, call_edges, import_edges) = if let Some(extractor) = get_extractor(lang) {
            let syms = extractor.extract_symbols(&file_record.id, content);
            let calls = extractor.extract_call_edges(&file_record.id, content, &syms);
            let imports = extractor.extract_import_edges(&file_record.id, content);
            (syms, calls, imports)
        } else {
            (Vec::new(), Vec::new(), Vec::new())
        };

        for sym in &symbols {
            store.upsert_symbol(sym).await?;
            all_project_symbols.push(sym.clone());
        }
        total_symbols += symbols.len();

        for call in &call_edges {
            let _ = store.upsert_call_edge(call).await;
            total_call_edges += 1;
        }

        for import in &import_edges {
            let _ = store.upsert_import_edge(import).await;
            total_import_edges += 1;
        }

        let chunks = chunker.chunk_file(&file_record.id, content, &symbols);
        all_chunks.extend(chunks);
    }

    // Embed only delta chunks
    let chunks_count = all_chunks.len();
    if chunks_count > 0 {
        println!(
            "Delta embedding: processing {} new/modified chunks...",
            chunks_count
        );
        for chunk_batch in all_chunks.chunks_mut(batch_size) {
            let texts: Vec<String> = chunk_batch.iter().map(|c| c.text.clone()).collect();
            let embeddings = embedder.embed_batch(&texts).await?;

            for (chunk, emb) in chunk_batch.iter_mut().zip(embeddings) {
                chunk.embedding = Some(emb);
                chunk.embedding_model = Some(manifest.embedding.model.clone());
                chunk.embedding_dim = Some(embedder.dimension());
                chunk.vector_set_id = Some(manifest.embedding.vector_set_id.clone());
            }

            store.upsert_chunks_batch(chunk_batch).await?;
            total_chunks += chunk_batch.len();
        }
    }

    // Delta Documentation sections
    let mut doc_sections_count = 0;
    for (file_record, content) in &active_files_to_index {
        let is_markdown = file_record.relative_path.ends_with(".md")
            || file_record.relative_path.ends_with(".markdown")
            || file_record.relative_path.ends_with(".rst");

        if is_markdown {
            let mut sections = DocLinker::extract_doc_sections(&file_record.relative_path, content);
            if !sections.is_empty() {
                let texts: Vec<String> = sections.iter().map(|s| s.content.clone()).collect();
                if let Ok(embeddings) = embedder.embed_batch(&texts).await {
                    for (sec, emb) in sections.iter_mut().zip(embeddings) {
                        sec.embedding = Some(emb);
                        let _ = store.upsert_doc_section(sec).await;
                        doc_sections_count += 1;
                    }
                }
                let doc_refs = DocLinker::link_sections_to_symbols(&sections, &all_project_symbols);
                for r in doc_refs {
                    let _ = store.upsert_doc_reference(&r).await;
                }
            }
        }
    }

    // Refresh anatomy index
    let anatomy_index = AnatomyScanner::scan_project(project_root, None)?;
    let anatomy_path = oxide_dir.join("anatomy_index.json");
    if let Ok(json) = serde_json::to_string_pretty(&anatomy_index) {
        let _ = std::fs::write(anatomy_path, json);
    }

    // ---------------------------------------------------------
    // 2. TIER 2: Agent Intelligence & Anti-Duplication Update
    // ---------------------------------------------------------
    let mut crates_cataloged = 0;
    if scan_crates {
        println!("Scanning workspace for reusable crates (Anti-Duplication Catalog)...");
        let home_projects = std::env::var("HOME")
            .ok()
            .map(|h| PathBuf::from(h).join("RustroverProjects"));
        let candidate_roots = [project_root.parent(), home_projects.as_deref()];

        let mut scanned_crates = HashSet::new();
        for root_opt in candidate_roots.into_iter().flatten() {
            if root_opt.exists() {
                crates_cataloged +=
                    scan_and_catalog_crates(root_opt, &store, &embedder, &mut scanned_crates)
                        .await?;
            }
        }
    }

    let mut git_memories_distilled = 0;
    if distill_git {
        println!("Distilling recent git commits into agent decision/learning memories...");
        git_memories_distilled =
            distill_git_history(project_root, &manifest, &store, &embedder).await?;
    }

    // Temporal decay & rule consolidation
    let decay_sql = r#"
        UPDATE calls SET 
            weight = weight * math::pow(0.5, (time::now() - valid_from) / 1209600.0)
        WHERE valid_from IS NOT NONE;
    "#;
    let _ = store.db().query(decay_sql).await;
    let _ = store.db().query("DELETE calls WHERE weight < 0.15;").await;
    let rules_count = store.list_active_rules().await.unwrap_or_default().len();

    let elapsed = start_time.elapsed();
    println!(
        "\nOxide-Embed Context & Intelligence Sync Complete in {:.2?}:",
        elapsed
    );
    println!("  [Project Context Delta]");
    println!(
        "    Unchanged files:     {} (0 re-embeddings needed)",
        files_unchanged
    );
    println!("    New files added:     {}", files_added);
    println!("    Updated files:       {}", files_updated);
    println!("    Removed files:       {}", files_removed);
    println!("    Chunks updated:      {}", total_chunks);
    println!("    Symbols re-indexed:  {}", total_symbols);
    println!("    Call edges updated:  {}", total_call_edges);
    println!("    Import edges updated:{}", total_import_edges);
    println!("    Doc sections linked: {}", doc_sections_count);
    println!("  [Agent Metacognitive Intelligence]");
    println!("    Crates cataloged:    {}", crates_cataloged);
    println!("    Git memories added:  {}", git_memories_distilled);
    println!("    Active rules:        {}", rules_count);

    Ok(())
}

async fn scan_and_catalog_crates(
    search_root: &Path,
    store: &SurrealProjectStore,
    embedder: &CandleBertEmbedder,
    already_scanned: &mut HashSet<String>,
) -> Result<usize> {
    let mut count = 0;
    let Ok(mut entries) = tokio::fs::read_dir(search_root).await else {
        return Ok(0);
    };

    while let Ok(Some(entry)) = entries.next_entry().await {
        let p = entry.path();
        let is_dir = match entry.file_type().await {
            Ok(ft) => ft.is_dir(),
            Err(_) => false,
        };
        if !is_dir {
            continue;
        }

        let cargo_toml = p.join("Cargo.toml");
        if cargo_toml.exists() {
            let Ok(content) = tokio::fs::read_to_string(&cargo_toml).await else {
                continue;
            };

            let Ok(val) = toml::from_str::<toml::Value>(&content) else {
                continue;
            };

            let Some(pkg) = val.get("package") else {
                continue;
            };

            let name = pkg
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            if name.is_empty() || !already_scanned.insert(name.clone()) {
                continue;
            }

            let version = pkg
                .get("version")
                .and_then(|v| v.as_str())
                .unwrap_or("0.1.0")
                .to_string();

            let description = pkg
                .get("description")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            let mut entry = CrateCatalogEntry::new(name, p.to_string_lossy().to_string(), version);
            entry.description = description.clone();

            // Check if no_std in lib.rs
            let lib_rs = p.join("src").join("lib.rs");
            if lib_rs.exists()
                && let Ok(src) = tokio::fs::read_to_string(&lib_rs).await
            {
                if src.contains("#![no_std]") {
                    entry.target_arch = Some("no_std".into());
                } else {
                    entry.target_arch = Some("std".into());
                }
            }

            // Embedding for semantic discovery
            let text_to_embed = format!(
                "crate {} {} {}",
                entry.crate_name,
                description.as_deref().unwrap_or(""),
                entry.target_arch.as_deref().unwrap_or("")
            );

            if let Ok(embs) = embedder.embed_batch(&[text_to_embed]).await
                && let Some(emb) = embs.into_iter().next()
            {
                entry.embedding = Some(emb);
            }

            let _ = store.upsert_crate_catalog(&entry).await;
            count += 1;
        }
    }

    Ok(count)
}

async fn distill_git_history(
    project_root: &Path,
    manifest: &OxideManifest,
    store: &SurrealProjectStore,
    embedder: &CandleBertEmbedder,
) -> Result<usize> {
    let output = Command::new("git")
        .args([
            "log",
            "-n",
            "15",
            "--pretty=format:%H|%an|%s%n%b%n---COMMIT_END---",
        ])
        .current_dir(project_root)
        .output()
        .await;

    let Ok(output) = output else {
        return Ok(0);
    };

    if !output.status.success() {
        return Ok(0);
    }

    let raw_log = String::from_utf8_lossy(&output.stdout);
    let distilled = MemoryDistiller::distill_git_log(manifest.project_id.clone(), &raw_log);

    let mut count = 0;
    for mem in distilled {
        let sql = "SELECT id FROM memory_record WHERE source_hash = $hash LIMIT 1;";
        let hash = mem.source_hash.clone().unwrap_or_default();
        let already_stored = if let Ok(mut res) = store.db().query(sql).bind(("hash", hash)).await {
            let rows: Vec<serde_json::Value> = res.take(0).unwrap_or_default();
            !rows.is_empty()
        } else {
            false
        };

        if !already_stored {
            let embed_text = format!("{}: {}", mem.title, mem.content);
            let emb = if let Ok(embs) = embedder.embed_batch(&[embed_text]).await {
                embs.into_iter().next()
            } else {
                None
            };

            let _ = store.upsert_memory(&mem, emb).await;
            count += 1;
        }
    }

    Ok(count)
}
