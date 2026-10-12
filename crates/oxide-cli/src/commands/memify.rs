use oxide_core::error::{OxideError, Result};
use oxide_core::{MemifyEngine, OxideManifest};
use oxide_db::{ProjectStore, SurrealProjectStore};
use std::path::Path;

pub async fn handle_memify(
    project_root: &Path,
    decay_days: f64,
    prune_threshold: f32,
) -> Result<()> {
    let oxide_dir = project_root.join(".oxide");
    if !oxide_dir.exists() {
        return Err(OxideError::NotInitialized(project_root.to_path_buf()));
    }

    let manifest = OxideManifest::load_from_dir(&oxide_dir)?;
    let db_path = oxide_dir.join(&manifest.storage.path);
    let store = SurrealProjectStore::open(&db_path).await?;

    println!(
        "Running Memify Active Forgetting & Edge Weight Decay (Half-life: {} days, Threshold: {})...",
        decay_days, prune_threshold
    );

    let engine = MemifyEngine::new(decay_days, prune_threshold);
    let symbol_count = store.count_symbols().await?;
    println!("Evaluated {} symbols in memory graph.", symbol_count);

    // Apply live decay to calls edges in SurrealDB
    let decay_sql = r#"
        UPDATE calls SET 
            weight = weight * math::pow(0.5, (time::now() - valid_from) / ($half_life_secs))
        WHERE valid_from IS NOT NONE;
    "#;
    let half_life_secs = (decay_days * 86400.0).max(1.0);
    let _ = store
        .db()
        .query(decay_sql)
        .bind(("half_life_secs", half_life_secs))
        .await;

    // Prune stale calls edges below prune_threshold
    let prune_sql = "DELETE calls WHERE weight < $threshold;";
    let _ = store
        .db()
        .query(prune_sql)
        .bind(("threshold", prune_threshold as f64))
        .await;

    println!("Memify cycle completed. Stale graph connections pruned below threshold {}.", prune_threshold);

    Ok(())
}
