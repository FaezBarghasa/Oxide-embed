use oxide_core::budget::BudgetCandidate;
use oxide_core::context_builder::ContextSynthesizer;
use oxide_core::error::{OxideError, Result};
use oxide_core::handoff::HandoffCheckpoint;
use oxide_core::memify::CerebrumRule;
use oxide_db::traversal::GraphTraversalService;
use oxide_db::{ProjectStore, SearchQuery, SurrealProjectStore};
use oxide_ml::Embedder;
use oxide_ml::candle_embedder::CandleBertEmbedder;
use std::path::Path;

pub async fn handle_context(
    project_root: &Path,
    task_query: &str,
    budget_tokens: usize,
) -> Result<()> {
    let oxide_dir = project_root.join(".oxide");
    if !oxide_dir.exists() {
        return Err(OxideError::Config(
            "Project not initialized. Run 'oxide-embed init' first.".into(),
        ));
    }

    // 1. Read Active Handoff Checkpoint
    let status_path = oxide_dir.join("STATUS.md");
    let handoff = if status_path.exists() {
        let content = std::fs::read_to_string(&status_path)?;
        HandoffCheckpoint::parse_markdown(&content)
    } else {
        None
    };

    // 2. Read Cerebrum Rules & Active Typed Semantic Memories
    let mut rules = Vec::new();
    let cerebrum_path = oxide_dir.join("docs").join("CEREBRUM.md");
    if cerebrum_path.exists() {
        let c_text = std::fs::read_to_string(&cerebrum_path)?;
        for (idx, line) in c_text.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
                let rule_str = trimmed
                    .trim_start_matches("- ")
                    .trim_start_matches("* ")
                    .to_string();
                rules.push(CerebrumRule {
                    id: format!("rule_{}", idx),
                    title: format!("Rule {}", idx + 1),
                    condition_pattern: "context".into(),
                    prescribed_solution: rule_str,
                    confidence: 0.9,
                    source_incidents: vec![],
                    created_at: chrono::Utc::now().timestamp(),
                });
            }
        }
    }

    // Read active typed semantic rules from database
    if let Ok(db_path) = oxide_core::resolve_db_path(project_root)
        && let Ok(store) = SurrealProjectStore::open(&db_path).await
        && let Ok(db_memories) = store.list_active_rules().await
    {
        for (idx, mem) in db_memories.into_iter().enumerate() {
            rules.push(CerebrumRule {
                id: format!("mem_{}", idx),
                title: format!("[{}] {}", mem.kind.as_str().to_uppercase(), mem.title),
                condition_pattern: mem.kind.as_str().to_string(),
                prescribed_solution: mem.content,
                confidence: 0.95,
                source_incidents: vec![],
                created_at: mem.created_at.timestamp(),
            });
        }
    }

    // 3. Search Knowledge Graph & Vectors
    let mut candidates = Vec::new();
    if let Ok(db_path) = oxide_core::resolve_db_path(project_root) {
        let store = SurrealProjectStore::open(&db_path).await?;
        let embedder = CandleBertEmbedder::new_offline();
        let embedding = embedder.embed(task_query).await.ok();

        let query = SearchQuery {
            text: task_query.to_string(),
            embedding,
            limit: 8,
            language: None,
        };

        let hits = store.search(&query).await?;
        for hit in hits {
            let mut snippet = format!(
                "File: {} (L{}-L{})\n{}",
                hit.file_path, hit.start_line, hit.end_line, hit.text
            );

            // Fetch 2-hop subgraph if symbol
            if let Some(ref sym_name) = hit.symbol_name
                && let Ok(Some(subgraph)) =
                    GraphTraversalService::get_subgraph(&store, sym_name, 2).await
            {
                snippet.push_str(&format!("\n\n{}", subgraph.to_compact_string()));
            }

            let title = hit.symbol_name.unwrap_or(hit.file_path);
            candidates.push(BudgetCandidate::new(
                format!("{}:{}", hit.start_line, title),
                title,
                snippet,
                hit.score,
            ));
        }
    }

    // 4. Synthesize context fitting exact token budget
    let ctx = ContextSynthesizer::build(task_query, handoff, rules, candidates, budget_tokens);
    println!("{}", ctx.formatted_markdown);

    Ok(())
}
