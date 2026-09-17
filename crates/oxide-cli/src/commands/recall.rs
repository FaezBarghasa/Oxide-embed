use chrono::{DateTime, Utc};
use oxide_core::budget::TokenEstimator;
use oxide_core::error::Result;
use oxide_core::memory::MemoryKind;
use oxide_core::resolve_db_path;
use oxide_db::{ProjectStore, SurrealProjectStore};
use oxide_ml::Embedder;
use oxide_ml::candle_embedder::CandleBertEmbedder;
use std::path::Path;

pub async fn handle_recall(
    project_root: &Path,
    query: &str,
    kind_str: Option<&str>,
    tags: &[String],
    as_of_str: Option<&str>,
    budget: Option<usize>,
    limit: usize,
) -> Result<()> {
    let db_path = resolve_db_path(project_root)?;
    let store = SurrealProjectStore::open(&db_path).await?;

    let kind_opt = match kind_str {
        Some(k) => Some(k.parse::<MemoryKind>()?),
        None => None,
    };

    let as_of_opt = match as_of_str {
        Some(s) => DateTime::parse_from_rfc3339(s)
            .map(|dt| dt.with_timezone(&Utc))
            .ok(),
        None => None,
    };

    let embedder = CandleBertEmbedder::new_offline();
    let query_emb = embedder.embed(query).await.ok();

    let memories = store
        .recall_memories(query_emb.as_deref(), kind_opt, tags, as_of_opt, limit)
        .await?;

    println!("🔍 Recalled Memories for: \"{}\"\n", query);
    if let Some(k) = kind_opt {
        println!("  Filter Category: [{}]", k.as_str());
    }
    if let Some(as_of) = as_of_opt {
        println!("  Point-in-Time:   As of {}", as_of.to_rfc3339());
    }

    if memories.is_empty() {
        println!("  (No matching memories found)");
        return Ok(());
    }

    let mut total_tokens = 0;
    let mut printed = 0;

    for mem in memories {
        let block = format!(
            "[{}] {} ({})\n   {}\n   Tags: {}\n",
            mem.kind.as_str().to_uppercase(),
            mem.title,
            mem.created_at.format("%Y-%m-%d %H:%M"),
            mem.content,
            if mem.tags.is_empty() {
                "none".to_string()
            } else {
                mem.tags.join(", ")
            }
        );

        let cost = TokenEstimator::estimate_tokens(&block);
        if let Some(b) = budget
            && total_tokens + cost > b
        {
            println!(
                "\n📊 Budget ceiling reached ({} / {} tokens). Truncating output.",
                total_tokens, b
            );
            break;
        }
        total_tokens += cost;
        printed += 1;

        println!(
            "🔹 [{}] **{}** `{}`",
            mem.kind.as_str().to_uppercase(),
            mem.title,
            mem.id
        );
        println!("   Status:     {}", mem.status);
        println!("   Timestamp:  {}", mem.created_at.to_rfc3339());
        if let Some(sym) = &mem.symbol_ref {
            println!("   Governs:    `{}`", sym);
        }
        if !mem.tags.is_empty() {
            println!("   Tags:       {}", mem.tags.join(", "));
        }
        println!("   Content:    {}", mem.content);
        println!();
    }

    println!(
        "📈 Displayed {} memories ({} tokens)",
        printed, total_tokens
    );
    Ok(())
}
