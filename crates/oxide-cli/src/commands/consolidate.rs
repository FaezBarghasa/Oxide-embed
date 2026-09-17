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
    let _store = SurrealProjectStore::open(&db_path).await?;

    println!("Consolidating incident memory into Cerebrum rules...");

    let engine = MemifyEngine::default();

    // Check for any local bug log files or stored incidents
    let bug_log_file = oxide_dir.join("buglogs.json");
    let bug_logs: Vec<BugLogRecord> = if bug_log_file.exists() {
        std::fs::read_to_string(&bug_log_file)
            .ok()
            .and_then(|c| serde_json::from_str(&c).ok())
            .unwrap_or_default()
    } else {
        Vec::new()
    };

    let rules = engine.consolidate_bugs_to_rules(&bug_logs);
    let out_file = engine.export_cerebrum_markdown(project_root, &rules)?;

    println!(
        "Cerebrum knowledge synthesis complete. {} rules active.",
        rules.len()
    );
    println!("Exported to: {}", out_file.display());

    Ok(())
}
