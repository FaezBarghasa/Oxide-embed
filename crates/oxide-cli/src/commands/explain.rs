use oxide_core::OxideManifest;
use oxide_core::error::{OxideError, Result};
use oxide_db::{GraphTraversalService, ProjectStore, SurrealProjectStore};
use std::path::Path;

pub async fn handle_explain(project_root: &Path, symbol_query: &str, hops: usize) -> Result<()> {
    let oxide_dir = project_root.join(".oxide");
    if !oxide_dir.exists() {
        return Err(OxideError::NotInitialized(project_root.to_path_buf()));
    }

    let manifest = OxideManifest::load_from_dir(&oxide_dir)?;
    let db_path = oxide_dir.join(&manifest.storage.path);
    let store = SurrealProjectStore::open(&db_path).await?;

    let subgraph = GraphTraversalService::get_subgraph(&store, symbol_query, hops).await?;

    match subgraph {
        Some(ctx) => {
            println!(
                "================ EXPLAIN SYMBOL: {} ================",
                ctx.target_symbol
            );
            println!("* Kind:      {}", ctx.symbol_kind);
            println!("* File Path: {}", ctx.file_path);
            if let Some(sig) = &ctx.signature {
                println!("* Signature: {}", sig);
            }
            println!("--------------------------------------------------");
            println!("Multi-hop Knowledge Graph Context ({} hops):", hops);
            println!("{}", ctx.to_compact_string());
            println!("==================================================");
        }
        None => {
            println!(
                "Symbol '{}' not found in project knowledge graph.",
                symbol_query
            );
        }
    }

    Ok(())
}
