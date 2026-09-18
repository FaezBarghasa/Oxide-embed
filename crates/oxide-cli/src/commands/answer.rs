use oxide_core::answer::AnswerSynthesizer;
use oxide_core::error::{OxideError, Result};
use oxide_core::memory::MemoryKind;
use oxide_core::resolve_db_path;
use oxide_db::traversal::GraphTraversalService;
use oxide_db::{ProjectStore, SearchQuery, SurrealProjectStore};
use oxide_ml::Embedder;
use oxide_ml::candle_embedder::CandleBertEmbedder;
use std::path::Path;

pub async fn handle_answer(
    project_root: &Path,
    question: &str,
    kind_str: Option<&str>,
    budget: usize,
) -> Result<()> {
    let oxide_dir = project_root.join(".oxide");
    if !oxide_dir.exists() {
        return Err(OxideError::Config(
            "Project not initialized. Run 'oxide-embed init' first.".into(),
        ));
    }

    let db_path = resolve_db_path(project_root)?;
    let store = SurrealProjectStore::open(&db_path).await?;

    let kind_opt: Option<MemoryKind> = match kind_str {
        Some(k) => Some(k.parse()?),
        None => None,
    };

    // 1. Embed query
    let embedder = CandleBertEmbedder::new_offline();
    let query_emb = embedder.embed(question).await.ok();

    // 2. Recall top matching memories
    let memories = store
        .recall_memories(query_emb.as_deref(), kind_opt, &[], None, 5)
        .await?;

    // 3. Retrieve active rules
    let active_rules = store.list_active_rules().await?;
    let rule_strings: Vec<String> = active_rules
        .into_iter()
        .map(|r| {
            format!(
                "[{}] {}: {}",
                r.kind.as_str().to_uppercase(),
                r.title,
                r.content
            )
        })
        .collect();

    // 4. Retrieve code symbols & 2-hop subgraphs
    let mut code_snippets = Vec::new();
    let query = SearchQuery {
        text: question.to_string(),
        embedding: query_emb,
        limit: 3,
        language: None,
    };
    if let Ok(hits) = store.search(&query).await {
        for hit in hits {
            let mut snippet = format!(
                "File: {} (L{}-L{})\n{}",
                hit.file_path, hit.start_line, hit.end_line, hit.text
            );
            if let Some(ref sym_name) = hit.symbol_name
                && let Ok(Some(subgraph)) =
                    GraphTraversalService::get_subgraph(&store, sym_name, 2).await
            {
                snippet.push_str(&format!("\n\n{}", subgraph.to_compact_string()));
            }
            code_snippets.push(snippet);
        }
    }

    // 5. Synthesize grounded answer
    let answer =
        AnswerSynthesizer::answer(question, &memories, &rule_strings, &code_snippets, budget);

    println!("{}", answer.answer);
    if !answer.citations.is_empty() {
        println!("📚 Grounded Citations:");
        for c in answer.citations {
            println!(
                "  - [{}] {} (`{}`)",
                c.source_type.to_uppercase(),
                c.title,
                c.source_id
            );
        }
    }

    Ok(())
}
