use std::fs;
use std::path::Path;
use oxide_core::error::Result;
use oxide_core::{OxideConfig, OxideManifest};

pub fn handle_init(project_root: &Path, name_override: Option<String>) -> Result<()> {
    let project_name = name_override.unwrap_or_else(|| {
        project_root
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unnamed-project")
            .to_string()
    });

    let oxide_dir = project_root.join(".oxide");
    fs::create_dir_all(&oxide_dir)?;

    // Create subdirectories
    fs::create_dir_all(oxide_dir.join("snapshots"))?;
    fs::create_dir_all(oxide_dir.join("exports"))?;
    fs::create_dir_all(oxide_dir.join("backups"))?;
    fs::create_dir_all(oxide_dir.join("cache"))?;
    fs::create_dir_all(oxide_dir.join("logs"))?;
    fs::create_dir_all(oxide_dir.join("runtime"))?;

    // Create or update manifest.json
    let mut manifest = OxideManifest::new(&project_name);
    manifest.save_to_dir(&oxide_dir)?;

    // Create config.toml
    let config = OxideConfig::default_for_project(&project_name);
    config.save_to_dir(&oxide_dir)?;

    // Create .gitignore in .oxide
    let gitignore_path = oxide_dir.join(".gitignore");
    if !gitignore_path.exists() {
        fs::write(
            &gitignore_path,
            "project.db/\ncache/\nlogs/\nruntime/\n",
        )?;
    }

    println!("Initialized oxide-embed project memory in .oxide/");
    println!("Project: {}", project_name);
    println!("Project ID: {}", manifest.project_id);
    println!("Schema version: {}", manifest.schema_version);
    Ok(())
}
