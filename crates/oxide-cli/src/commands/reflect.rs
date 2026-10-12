use oxide_core::error::{OxideError, Result};
use oxide_core::{OxideManifest, ReflectEngine, RrfCandidate};
use oxide_db::{ProjectStore, SearchQuery, SurrealProjectStore};
use oxide_ml::{CandleBertEmbedder, Embedder};
use std::path::Path;

pub async fn handle_reflect(
    project_root: &Path,
    query: &str,
    token_budget: usize,
    force_fresh: bool,
) -> Result<()> {
    let oxide_dir = project_root.join(".oxide");
    if !oxide_dir.exists() {
        return Err(OxideError::NotInitialized(project_root.to_path_buf()));
    }

    let manifest = OxideManifest::load_from_dir(&oxide_dir)?;
    let db_path = oxide_dir.join(&manifest.storage.path);
    let store = SurrealProjectStore::open(&db_path).await?;

    let embedder = CandleBertEmbedder::new_offline();
    let query_emb = embedder.embed(query).await?;

    let engine = ReflectEngine::new(token_budget);

    // 1. Check standing Mental Models unless force_fresh is set
    if !force_fresh {
        let matching_models = store.search_mental_models(&query_emb, 3).await?;
        if let Some(model) = engine.resolve_standing_model(&matching_models) {
            println!(
                "💡 [Mental Model Hit (Zero-Token Synthesis)]: {}\n",
                model.topic
            );
            println!("{}", model.to_markdown());
            return Ok(());
        }
    }

    // 2. Fall back to 4-Way RRF Recall
    println!("🔍 [Reflect] No fresh standing model found. Executing 4-Way RRF Recall...\n");

    // Stream 1: Dense Vector (chunks)
    let search_hits = store
        .search(&SearchQuery {
            text: query.to_string(),
            embedding: Some(query_emb.clone()),
            limit: 20,
            language: None,
        })
        .await?;

    let vector_ranked: Vec<RrfCandidate> = search_hits
        .into_iter()
        .map(|hit| RrfCandidate {
            key: format!("{}:{}-{}", hit.file_path, hit.start_line, hit.end_line),
            title: hit.symbol_name.unwrap_or_else(|| hit.file_path.clone()),
            content: hit.text,
            source_type: "code_chunk".to_string(),
            raw_score: hit.score,
            file_path: Some(hit.file_path),
            start_line: Some(hit.start_line),
            end_line: Some(hit.end_line),
        })
        .collect();

    // Stream 2: STAIR Lexical
    let stair_hits = store.stair_search(query, 20).await?;
    let lexical_ranked: Vec<RrfCandidate> = stair_hits
        .into_iter()
        .map(|hit| RrfCandidate {
            key: format!("{}:{}", hit.file_path, hit.leaf_symbol),
            title: hit.leaf_symbol.clone(),
            content: format!(
                "Signature: {}\nCode: {}",
                hit.signature.unwrap_or_default(),
                hit.code_body
            ),
            source_type: "stair_symbol".to_string(),
            raw_score: hit.confidence,
            file_path: Some(hit.file_path),
            start_line: Some(hit.start_line),
            end_line: Some(hit.end_line),
        })
        .collect();

    // Stream 3: Memories
    let memories = store
        .recall_memories(Some(&query_emb), None, &[], None, 10)
        .await?;
    let temporal_ranked: Vec<RrfCandidate> = memories
        .into_iter()
        .map(|mem| RrfCandidate {
            key: mem.id.0.clone(),
            title: format!("[{}] {}", mem.kind, mem.title),
            content: mem.content,
            source_type: "memory".to_string(),
            raw_score: mem.confidence,
            file_path: None,
            start_line: None,
            end_line: None,
        })
        .collect();

    // Fuse using RRF
    let fused = engine
        .rrf_scorer
        .fuse(&vector_ranked, &lexical_ranked, &[], &temporal_ranked);

    // Synthesize reflection
    let reflect_result = engine.synthesize_from_fused(query, &fused);

    println!("{}", reflect_result.answer);
    println!(
        "Citations ({} total, ~{} tokens):",
        reflect_result.citations.len(),
        reflect_result.token_count
    );
    for cit in &reflect_result.citations {
        println!(" - {}", cit);
    }

    Ok(())
}
