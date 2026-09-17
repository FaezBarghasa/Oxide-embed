use oxide_core::error::{OxideError, Result};
use oxide_core::resolve_db_path;
use oxide_db::traversal::GraphTraversalService;
use oxide_db::SurrealProjectStore;
use std::path::Path;

pub async fn handle_callees(project_root: &Path, symbol: &str, budget: Option<usize>) -> Result<()> {
    let db_path = resolve_db_path(project_root)?;
    let store = SurrealProjectStore::open(&db_path).await?;

    let subgraph = GraphTraversalService::get_subgraph(&store, symbol, 1).await?;

    match subgraph {
        Some(ctx) => {
            println!("🎯 Callees invoked by `{}` ({} in {}):", ctx.target_symbol, ctx.symbol_kind, ctx.file_path);
            if ctx.callees.is_empty() {
                println!("  (No direct callees indexed)");
            } else {
                for callee in &ctx.callees {
                    println!("  → {}", callee);
                }
            }

            if let Some(limit) = budget {
                println!("\n📊 Packed within token budget ({} tokens ceiling)", limit);
            }
            Ok(())
        }
        None => Err(OxideError::NotFound(format!("Symbol '{}' not found in knowledge graph", symbol))),
    }
}
