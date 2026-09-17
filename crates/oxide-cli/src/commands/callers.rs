use oxide_core::error::{OxideError, Result};
use oxide_core::resolve_db_path;
use oxide_db::traversal::GraphTraversalService;
use oxide_db::{ProjectStore, SurrealProjectStore};
use std::path::Path;

pub async fn handle_callers(project_root: &Path, symbol: &str, budget: Option<usize>) -> Result<()> {
    let db_path = resolve_db_path(project_root)?;
    let store = SurrealProjectStore::open(&db_path).await?;

    let subgraph = GraphTraversalService::get_subgraph(&store, symbol, 1).await?;

    match subgraph {
        Some(ctx) => {
            println!("📞 Callers of `{}` ({} in {}):", ctx.target_symbol, ctx.symbol_kind, ctx.file_path);
            if ctx.callers.is_empty() {
                println!("  (No direct callers indexed)");
            } else {
                for caller in &ctx.callers {
                    println!("  ← {}", caller);
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
