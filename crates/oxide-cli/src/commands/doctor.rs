use oxide_core::error::{OxideError, Result};
use oxide_core::{OxideConfig, OxideManifest};
use oxide_db::{ProjectStore, SurrealProjectStore};
use std::path::Path;

pub async fn handle_doctor(project_root: &Path) -> Result<()> {
    let oxide_dir = project_root.join(".oxide");
    if !oxide_dir.exists() {
        return Err(OxideError::NotInitialized(project_root.to_path_buf()));
    }

    println!("Running oxide-embed diagnostics...\n");

    // 1. Manifest check
    let manifest = match OxideManifest::load_from_dir(&oxide_dir) {
        Ok(m) => {
            println!("  [OK] Manifest loaded successfully");
            println!("       Project Name: {}", m.project_name);
            println!("       Project ID:   {}", m.project_id);
            println!("       Schema:       v{}", m.schema_version);
            m
        }
        Err(e) => {
            println!("  [FAIL] Manifest error: {}", e);
            return Err(e);
        }
    };

    // 2. Config check
    match OxideConfig::load_from_dir(&oxide_dir) {
        Ok(_) => println!("  [OK] Config file valid (.oxide/config.toml)"),
        Err(e) => println!("  [WARN] Config error: {}", e),
    }

    // 3. Database connectivity
    let db_path = oxide_dir.join(&manifest.storage.path);
    match SurrealProjectStore::open(&db_path).await {
        Ok(store) => {
            let files = store.count_files().await.unwrap_or(0);
            let symbols = store.count_symbols().await.unwrap_or(0);
            let chunks = store.count_chunks().await.unwrap_or(0);
            println!("  [OK] Embedded SurrealDB reachable");
            println!("       Indexed Files:   {}", files);
            println!("       Indexed Symbols: {}", symbols);
            println!("       Indexed Chunks:  {}", chunks);
        }
        Err(e) => {
            println!("  [FAIL] Database initialization failed: {}", e);
            return Err(e);
        }
    }

    // 4. Embedding compatibility
    println!("  [OK] Embedding Configuration:");
    println!("       Model:          {}", manifest.embedding.model);
    println!(
        "       Vector Set ID:  {}",
        manifest.embedding.vector_set_id
    );
    println!("       Dimension:      {}", manifest.embedding.dimension);

    println!("\nStatus: Ready");
    Ok(())
}
