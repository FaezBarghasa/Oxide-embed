use oxide_core::error::{OxideError, Result};
use oxide_core::resolve_db_path;
use oxide_db::traversal::GraphTraversalService;
use oxide_db::{ProjectStore, SurrealProjectStore};
use std::path::Path;

pub async fn handle_impact(project_root: &Path, symbol: &str) -> Result<()> {
    let db_path = resolve_db_path(project_root)?;
    let store = SurrealProjectStore::open(&db_path).await?;

    let subgraph = GraphTraversalService::get_subgraph(&store, symbol, 2).await?;

    match subgraph {
        Some(ctx) => {
            println!(
                "💥 Blast Radius & Impact Graph for `{}` ({}):",
                ctx.target_symbol, ctx.symbol_kind
            );
            println!("  📁 File: {}", ctx.file_path);
            if let Some(sig) = &ctx.signature {
                println!("  📝 Signature: {}", sig);
            }

            println!("\n  ⬆️ Inbound Impact (Callers that break if signature changes):");
            if ctx.callers.is_empty() {
                println!("    (No direct inbound callers)");
            } else {
                for caller in &ctx.callers {
                    println!("    ← {}", caller);
                }
            }

            println!("\n  ⬇️ Outbound Dependencies (Callees invoked by this symbol):");
            if ctx.callees.is_empty() {
                println!("    (No direct outbound callees)");
            } else {
                for callee in &ctx.callees {
                    println!("    → {}", callee);
                }
            }

            if !ctx.doc_references.is_empty() {
                println!("\n  📖 Document References:");
                for doc in &ctx.doc_references {
                    println!("    • {}", doc);
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
