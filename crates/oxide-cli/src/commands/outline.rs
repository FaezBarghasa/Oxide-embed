use oxide_core::OxideManifest;
use oxide_core::error::{OxideError, Result};
use oxide_core::id::FileId;
use oxide_db::{ProjectStore, SurrealProjectStore};
use oxide_parser::languages::get_extractor;
use oxide_parser::{Language, OutlineGenerator};
use std::fs;
use std::path::Path;

pub async fn handle_outline(project_root: &Path, rel_path: &str) -> Result<()> {
    let oxide_dir = project_root.join(".oxide");

    // If .oxide exists and indexed, try fast path from DB first
    if oxide_dir.exists()
        && let Ok(manifest) = OxideManifest::load_from_dir(&oxide_dir)
    {
        let db_path = oxide_dir.join(&manifest.storage.path);
        if let Ok(store) = SurrealProjectStore::open(&db_path).await
            && let Ok(Some(outline)) = store.get_file_outline(rel_path).await
        {
            println!("{}", outline);
            return Ok(());
        }
    }

    // Direct AST parse fallback
    let full_path = project_root.join(rel_path);
    if !full_path.exists() {
        return Err(OxideError::Other(format!(
            "Target file not found: {}",
            full_path.display()
        )));
    }

    let content = fs::read_to_string(&full_path)?;
    let lang = Language::from_path(rel_path);
    let file_id = FileId::from_relative_path(rel_path);

    if let Some(extractor) = get_extractor(lang) {
        let symbols = extractor.extract_symbols(&file_id, &content);
        let outline = OutlineGenerator::generate_outline(&symbols);
        println!("{}", outline);
    } else {
        println!("No AST parser available for file type '{}'", lang.as_str());
    }

    Ok(())
}
