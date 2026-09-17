use oxide_core::error::{OxideError, Result};
use oxide_core::{resolve_db_path, OxideConfig, OxideManifest};
use oxide_db::{ProjectStore, SurrealProjectStore};
use oxide_ml::{CandleBertEmbedder, Embedder};
use oxide_parser::languages::get_extractor;
use oxide_parser::{AnatomyScanner, Chunker, DocLinker, Language, ProjectWalker};
use std::path::Path;
use std::time::Instant;

pub async fn handle_index(project_root: &Path, _force: bool) -> Result<()> {
    let oxide_dir = project_root.join(".oxide");
    if !oxide_dir.exists() {
        return Err(OxideError::NotInitialized(project_root.to_path_buf()));
    }

    let manifest = OxideManifest::load_from_dir(&oxide_dir)?;
    let config = OxideConfig::load_from_dir(&oxide_dir)?;
    let db_path = resolve_db_path(project_root)?;
    let store = SurrealProjectStore::open(&db_path).await?;

    let start_time = Instant::now();
    println!(
        "Indexing & Cognifying project '{}'...",
        manifest.project_name
    );

    let walker = ProjectWalker::new(
        project_root,
        manifest.project_id.clone(),
        config.index.max_file_kb,
        config.index.ignore.clone(),
    );

    let files = walker.walk()?;
    println!("Found {} indexable files", files.len());

    let chunker = Chunker::new(config.index.chunk_max_tokens);
    let embedder = CandleBertEmbedder::new_offline();

    let mut total_symbols = 0;
    let mut total_chunks = 0;
    let mut total_call_edges = 0;
    let mut total_import_edges = 0;
    let mut total_doc_sections = 0;
    let mut total_doc_references = 0;
    let mut all_project_symbols = Vec::new();

    // 1. First Pass: Files, Symbols, Chunks, Call Graph, and Imports
    for (file_record, content) in &files {
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

        let mut chunks = chunker.chunk_file(&file_record.id, content, &symbols);
        for chunk in &mut chunks {
            let emb = embedder.embed(&chunk.text).await?;
            chunk.embedding = Some(emb);
            chunk.embedding_model = Some(manifest.embedding.model.clone());
            chunk.embedding_dim = Some(embedder.dimension());
            chunk.vector_set_id = Some(manifest.embedding.vector_set_id.clone());

            store.upsert_chunk(chunk).await?;
        }
        total_chunks += chunks.len();
    }

    // 2. Second Pass: ECL Pipeline (Doc Section Extraction and Doc-to-Symbol Linking)
    println!("Extracting documentation sections and linking to knowledge graph...");
    for (file_record, content) in &files {
        let is_markdown = file_record.relative_path.ends_with(".md")
            || file_record.relative_path.ends_with(".markdown")
            || file_record.relative_path.ends_with(".rst");

        if is_markdown {
            let sections = DocLinker::extract_doc_sections(&file_record.relative_path, content);
            for mut section in sections {
                let emb = embedder.embed(&section.content).await.ok();
                section.embedding = emb;
                store.upsert_doc_section(&section).await?;

                let edges = DocLinker::link_sections_to_symbols(
                    std::slice::from_ref(&section),
                    &all_project_symbols,
                );
                for edge in edges {
                    store.upsert_doc_reference(&edge).await?;
                    total_doc_references += 1;
                }
                total_doc_sections += 1;
            }
        }
    }

    // 3. Build & Save Anatomy Index
    let anatomy_index = AnatomyScanner::scan_project(project_root, None)?;
    let anatomy_path = oxide_dir.join("anatomy_index.json");
    if let Ok(json) = serde_json::to_string_pretty(&anatomy_index) {
        let _ = std::fs::write(anatomy_path, json);
    }

    let elapsed = start_time.elapsed();
    println!("Indexing completed in {:.2?}", elapsed);
    println!("  Files indexed:          {}", files.len());
    println!("  Symbols extracted:      {}", total_symbols);
    println!("  Chunks indexed:         {}", total_chunks);
    println!("  Call edges created:     {}", total_call_edges);
    println!("  Import edges created:   {}", total_import_edges);
    println!("  Doc sections linked:    {}", total_doc_sections);
    println!("  Doc references created: {}", total_doc_references);

    Ok(())
}
