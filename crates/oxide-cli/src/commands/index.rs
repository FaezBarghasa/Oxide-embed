use std::path::Path;
use std::time::Instant;
use oxide_core::error::{OxideError, Result};
use oxide_core::{OxideConfig, OxideManifest};
use oxide_db::{ProjectStore, SurrealProjectStore};
use oxide_ml::{Embedder, MockEmbedder};
use oxide_parser::languages::get_extractor;
use oxide_parser::{Chunker, Language, ProjectWalker};

pub async fn handle_index(project_root: &Path, _force: bool) -> Result<()> {
    let oxide_dir = project_root.join(".oxide");
    if !oxide_dir.exists() {
        return Err(OxideError::NotInitialized(project_root.to_path_buf()));
    }

    let manifest = OxideManifest::load_from_dir(&oxide_dir)?;
    let config = OxideConfig::load_from_dir(&oxide_dir)?;
    let db_path = oxide_dir.join(&manifest.storage.path);
    let store = SurrealProjectStore::open(&db_path).await?;

    let start_time = Instant::now();
    println!("Indexing project '{}'...", manifest.project_name);

    let walker = ProjectWalker::new(
        project_root,
        manifest.project_id.clone(),
        config.index.max_file_kb,
        config.index.ignore.clone(),
    );

    let files = walker.walk()?;
    println!("Found {} indexable files", files.len());

    let chunker = Chunker::new(config.index.chunk_max_tokens);
    let embedder = MockEmbedder::new(manifest.embedding.dimension);

    let mut total_symbols = 0;
    let mut total_chunks = 0;

    for (file_record, content) in &files {
        store.upsert_file(file_record).await?;

        let lang = Language::from_path(&file_record.relative_path);
        let symbols = if let Some(extractor) = get_extractor(lang) {
            extractor.extract_symbols(&file_record.id, content)
        } else {
            Vec::new()
        };

        for sym in &symbols {
            store.upsert_symbol(sym).await?;
        }
        total_symbols += symbols.len();

        let mut chunks = chunker.chunk_file(&file_record.id, content, &symbols);
        for chunk in &mut chunks {
            // Compute vector embedding
            let emb = embedder.embed(&chunk.text).await?;
            chunk.embedding = Some(emb);
            chunk.embedding_model = Some(manifest.embedding.model.clone());
            chunk.embedding_dim = Some(manifest.embedding.dimension);
            chunk.vector_set_id = Some(manifest.embedding.vector_set_id.clone());

            store.upsert_chunk(chunk).await?;
        }
        total_chunks += chunks.len();
    }

    let elapsed = start_time.elapsed();
    println!("Indexing completed in {:.2?}", elapsed);
    println!("  Files indexed:   {}", files.len());
    println!("  Symbols indexed: {}", total_symbols);
    println!("  Chunks indexed:  {}", total_chunks);

    Ok(())
}
