use std::path::Path;
use oxide_core::error::{OxideError, Result};
use oxide_core::OxideManifest;
use oxide_db::{ProjectStore, SearchQuery, SurrealProjectStore};
use oxide_ml::{Embedder, MockEmbedder};

pub async fn handle_search(project_root: &Path, query_str: &str, limit: usize) -> Result<()> {
    let oxide_dir = project_root.join(".oxide");
    if !oxide_dir.exists() {
        return Err(OxideError::NotInitialized(project_root.to_path_buf()));
    }

    let manifest = OxideManifest::load_from_dir(&oxide_dir)?;
    let db_path = oxide_dir.join(&manifest.storage.path);
    let store = SurrealProjectStore::open(&db_path).await?;

    let embedder = MockEmbedder::new(manifest.embedding.dimension);
    let query_embedding = embedder.embed(query_str).await.ok();

    let query = SearchQuery {
        text: query_str.to_string(),
        embedding: query_embedding,
        limit,
        language: None,
    };

    let hits = store.search(&query).await?;

    if hits.is_empty() {
        println!("No relevant matches found for '{}'", query_str);
        return Ok(());
    }

    println!("Top {} results for '{}':\n", hits.len(), query_str);
    for (idx, hit) in hits.iter().enumerate() {
        let sym = hit.symbol_name.as_deref().unwrap_or("<file>");
        println!(
            "{}. [{:.2}] {} :: {} (L{}-L{})",
            idx + 1,
            hit.score,
            hit.file_path,
            sym,
            hit.start_line,
            hit.end_line
        );
        if let Some(out) = &hit.outline_or_signature {
            println!("   {}", out);
        }
        println!();
    }

    Ok(())
}
