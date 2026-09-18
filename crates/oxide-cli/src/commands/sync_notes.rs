use oxide_core::error::Result;
use oxide_core::id::ProjectId;
use oxide_core::manifest::OxideManifest;
use oxide_core::markdown_sync::MarkdownMemorySync;
use oxide_core::resolve_db_path;
use oxide_db::{ProjectStore, SurrealProjectStore};
use std::path::Path;

pub async fn handle_sync_notes(project_root: &Path, direction: Option<&str>) -> Result<()> {
    let oxide_dir = project_root.join(".oxide");
    let memories_dir = oxide_dir.join("memories");
    let dir_mode = direction.unwrap_or("bidirectional");

    let db_path = resolve_db_path(project_root)?;
    let store = SurrealProjectStore::open(&db_path).await?;
    let manifest = OxideManifest::load_from_dir(project_root)
        .map(|m| m.project_id)
        .unwrap_or_else(|_| ProjectId::new_v7());

    match dir_mode {
        "export" => {
            let memories = store.list_all_memories().await?;
            let paths = MarkdownMemorySync::export_to_dir(&memories_dir, &memories)?;
            println!(
                "📥 Exported {} memories to {} category markdown files in `.oxide/memories/`:",
                memories.len(),
                paths.len()
            );
            for p in paths {
                println!(
                    "  - {}",
                    p.file_name().unwrap_or_default().to_string_lossy()
                );
            }
        }
        "import" => {
            let mut total_imported = 0;
            if memories_dir.exists() {
                for entry in std::fs::read_dir(&memories_dir)? {
                    let entry = entry?;
                    let path = entry.path();
                    if path.extension().and_then(|s| s.to_str()) == Some("md") {
                        let records =
                            MarkdownMemorySync::import_from_file(manifest.clone(), &path)?;
                        total_imported += records.len();
                        store.sync_all_memories(&records).await?;
                    }
                }
            }
            println!(
                "📤 Imported {} memory records from `.oxide/memories/` into database.",
                total_imported
            );
        }
        _ => {
            // First import any edited notes from disk
            let mut imported_records = Vec::new();
            if memories_dir.exists() {
                for entry in std::fs::read_dir(&memories_dir)? {
                    let entry = entry?;
                    let path = entry.path();
                    if path.extension().and_then(|s| s.to_str()) == Some("md")
                        && let Ok(records) =
                            MarkdownMemorySync::import_from_file(manifest.clone(), &path)
                    {
                        imported_records.extend(records);
                    }
                }
                if !imported_records.is_empty() {
                    store.sync_all_memories(&imported_records).await?;
                }
            }

            // Then export full current state to disk
            let all_memories = store.list_all_memories().await?;
            let paths = MarkdownMemorySync::export_to_dir(&memories_dir, &all_memories)?;
            println!(
                "🔄 Bidirectional sync complete. Synced {} memories across {} files in `.oxide/memories/`.",
                all_memories.len(),
                paths.len()
            );
        }
    }

    Ok(())
}
