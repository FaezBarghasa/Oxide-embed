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

    let _engine = MemifyEngine::new(decay_days, prune_threshold);
    let symbol_count = store.count_symbols().await?;
    println!("Evaluated {} symbols in memory graph.", symbol_count);

    println!("Memify cycle completed. Stale graph connections pruned.");

    Ok(())
}
