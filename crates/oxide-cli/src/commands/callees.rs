use oxide_core::error::{OxideError, Result};
use oxide_core::resolve_db_path;
use oxide_db::traversal::GraphTraversalService;
use oxide_db::{ProjectStore, SurrealProjectStore};
use std::path::Path;

pub async fn handle_callees(
    project_root: &Path,
    symbol: &str,
    budget: Option<usize>,
) -> Result<()> {
    let db_path = resolve_db_path(project_root)?;
    let store = SurrealProjectStore::open(&db_path).await?;

    let subgraph = GraphTraversalService::get_subgraph(&store, symbol, 1).await?;

    match subgraph {
        Some(ctx) => {
            println!(
                "🎯 Callees invoked by `{}` ({} in {}):",
                ctx.target_symbol, ctx.symbol_kind, ctx.file_path
            );
            if let Some(limit) = budget {
                let candidates: Vec<oxide_core::BudgetCandidate> = ctx
                    .callees
                    .iter()
                    .enumerate()
                    .map(|(i, c)| {
                        oxide_core::BudgetCandidate::new(
                            format!("callee:{}", i),
                            c.clone(),
                            format!("→ {}", c),
                            1.0,
                        )
                    })
                    .collect();
                let packed = oxide_core::TokenBudgetPacker::pack(candidates, limit);
                println!("\n📊 Packed {} callees within {} token budget (~{} tokens used):", packed.included.len(), limit, packed.used_tokens);
                for item in packed.included {
                    println!("  {}", item.content);
                }
            } else if ctx.callees.is_empty() {
                println!("  (No direct callees indexed)");
            } else {
                for callee in &ctx.callees {
                    println!("  → {}", callee);
                }
            }
            Ok(())
        }
        None => Err(OxideError::NotFound(format!(
            "Symbol '{}' not found in knowledge graph",
            symbol
        ))),
    }
}
