use oxide_core::error::{OxideError, Result};
use oxide_core::{BugLogRecord, MemifyEngine, OxideManifest};
use oxide_db::{ProjectStore, SurrealProjectStore};
use std::path::Path;

pub async fn handle_consolidate(project_root: &Path) -> Result<()> {
    let oxide_dir = project_root.join(".oxide");
    if !oxide_dir.exists() {
        return Err(OxideError::NotInitialized(project_root.to_path_buf()));
    }

    let manifest = OxideManifest::load_from_dir(&oxide_dir)?;
    let db_path = oxide_dir.join(&manifest.storage.path);
    let store = SurrealProjectStore::open(&db_path).await?;

    println!("Consolidating incident memory into Cerebrum rules...");

    let engine = MemifyEngine::default();

    // Query live error memories from SurrealDB
    let error_memories = store
        .recall_memories(None, Some(oxide_core::MemoryKind::Error), &[], None, 100)
        .await?;

    let bug_logs: Vec<BugLogRecord> = error_memories
        .into_iter()
        .map(|m| BugLogRecord {
            id: m.id.0,
            timestamp: m.created_at.timestamp(),
            symptom: m.title,
            root_cause: m.content.clone(),
            resolution: m.content,
            component: m.tags.first().cloned().unwrap_or_else(|| "core".to_string()),
            embedding: None,
        })
        .collect();

    let rules = engine.consolidate_bugs_to_rules(&bug_logs);
    let out_file = engine.export_cerebrum_markdown(project_root, &rules)?;

    println!(
        "Cerebrum knowledge synthesis complete. {} rules active.",
        rules.len()
    );
    println!("Exported to: {}", out_file.display());

    Ok(())
}
