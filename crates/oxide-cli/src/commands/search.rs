use oxide_core::budget::{BudgetCandidate, TokenBudgetPacker};
use oxide_core::error::{OxideError, Result};
use oxide_core::OxideManifest;
use oxide_db::{GraphTraversalService, ProjectStore, SearchQuery, SurrealProjectStore};
use oxide_ml::{CandleBertEmbedder, Embedder};
use std::path::Path;

pub async fn handle_search(
    project_root: &Path,
    query_str: &str,
    limit: usize,
    with_graph: bool,
    hops: usize,
    budget: Option<usize>,
) -> Result<()> {
    let oxide_dir = project_root.join(".oxide");
    if !oxide_dir.exists() {
        return Err(OxideError::NotInitialized(project_root.to_path_buf()));
    }

    let manifest = OxideManifest::load_from_dir(&oxide_dir)?;
    let db_path = oxide_dir.join(&manifest.storage.path);
    let store = SurrealProjectStore::open(&db_path).await?;

    let embedder = CandleBertEmbedder::new_offline();
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

    let mut candidates = Vec::new();

    for (idx, hit) in hits.iter().enumerate() {
        let sym = hit.symbol_name.as_deref().unwrap_or("<file>");
        let mut item_block = format!(
            "{}. [{:.2}] {} :: {} (L{}-L{})\n",
            idx + 1,
            hit.score,
            hit.file_path,
            sym,
            hit.start_line,
            hit.end_line
        );
        if let Some(out) = &hit.outline_or_signature {
            item_block.push_str(&format!("   {}\n", out));
        }

        if with_graph
            && let Some(symbol_name) = &hit.symbol_name
            && let Ok(Some(subgraph)) =
                GraphTraversalService::get_subgraph(&store, symbol_name, hops).await
        {
            item_block.push_str(&format!(
                "\n   [Knowledge Subgraph ({} hops)]:\n   {}\n",
                hops,
                subgraph.to_compact_string().replace('\n', "\n   ")
            ));
        }

        candidates.push(BudgetCandidate::new(
            format!("{}:{}", hit.file_path, hit.start_line),
            sym,
            item_block,
            hit.score,
        ));
    }

    if let Some(b) = budget {
        let packed = TokenBudgetPacker::pack(candidates, b);
        println!(
            "🎯 Search Results for '{}' (Packed {}/{} within {} token budget, used ~{} tokens):\n",
            query_str,
            packed.included.len(),
            hits.len(),
            b,
            packed.used_tokens
        );
        for item in packed.included {
            println!("{}", item.content);
        }
    } else {
        println!("Top {} results for '{}':\n", hits.len(), query_str);
        for item in candidates {
            println!("{}", item.content);
        }
    }

    Ok(())
}
