use std::path::Path;
use oxide_core::error::{OxideError, Result};
use oxide_core::OxideManifest;
use oxide_db::{OxemBundle, ProjectStore, SurrealProjectStore};

pub async fn handle_export(project_root: &Path, output_path: &Path) -> Result<()> {
    let oxide_dir = project_root.join(".oxide");
    if !oxide_dir.exists() {
        return Err(OxideError::NotInitialized(project_root.to_path_buf()));
    }

    let manifest = OxideManifest::load_from_dir(&oxide_dir)?;
    let db_path = oxide_dir.join(&manifest.storage.path);
    let store = SurrealProjectStore::open(&db_path).await?;

    OxemBundle::export(&store, &manifest, output_path).await?;
    println!("Exported project memory to bundle: {}", output_path.display());
    Ok(())
}

pub async fn handle_import(project_root: &Path, bundle_path: &Path) -> Result<()> {
    let oxide_dir = project_root.join(".oxide");
    std::fs::create_dir_all(&oxide_dir)?;

    let db_path = oxide_dir.join("project.db");
    let store = SurrealProjectStore::open(&db_path).await?;

    let mut manifest = OxemBundle::import(&store, bundle_path).await?;
    manifest.save_to_dir(&oxide_dir)?;

    println!("Imported memory bundle from: {}", bundle_path.display());
    println!("Project:    {}", manifest.project_name);
    println!("Project ID: {}", manifest.project_id);
    Ok(())
}
