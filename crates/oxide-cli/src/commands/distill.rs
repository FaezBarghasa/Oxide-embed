use oxide_core::distiller::MemoryDistiller;
use oxide_core::error::{OxideError, Result};
use oxide_core::id::ProjectId;
use oxide_core::manifest::OxideManifest;
use oxide_core::resolve_db_path;
use oxide_db::{ProjectStore, SurrealProjectStore};
use std::path::Path;
use std::process::Command;

pub async fn handle_distill(
    project_root: &Path,
    commit_range: Option<&str>,
    save: bool,
) -> Result<()> {
    let range = commit_range.unwrap_or("HEAD~10..HEAD");
    let manifest = OxideManifest::load_from_dir(project_root)
        .map(|m| m.project_id)
        .unwrap_or_else(|_| ProjectId::new_v7());

    // Run git log to extract conventional commits
    let output = Command::new("git")
        .arg("-C")
        .arg(project_root)
        .arg("log")
        .arg(range)
        .arg("--pretty=format:%h|%an|%s%n%b%n---COMMIT_END---")
        .output()
        .map_err(|e| OxideError::Config(format!("Failed to run git log: {e}")))?;

    if !output.status.success() {
        return Err(OxideError::Config(
            "Git command failed. Is this a valid git repository with commit history?".into(),
        ));
    }

    let log_str = String::from_utf8_lossy(&output.stdout);
    let distilled = MemoryDistiller::distill_git_log(manifest, &log_str);

    println!(
        "🔍 Distilled {} Semantic Memories from git range `{}`:\n",
        distilled.len(),
        range
    );

    if distilled.is_empty() {
        println!(
            "  No conventional commit patterns (feat:, fix:, refactor:, perf:, docs:) detected in range."
        );
        return Ok(());
    }

    let db_path = resolve_db_path(project_root).ok();
    let store = if save && let Some(ref path) = db_path {
        Some(SurrealProjectStore::open(path).await?)
    } else {
        None
    };

    for (i, mem) in distilled.iter().enumerate() {
        println!(
            "{}. [{}] **{}**",
            i + 1,
            mem.kind.as_str().to_uppercase(),
            mem.title
        );
        if let Some(ref author) = mem.author {
            println!("   Author: {}", author);
        }
        if let Some(ref hash) = mem.source_hash {
            println!("   Commit: {}", hash);
        }
        println!("   Content: {}\n", mem.content);

        if let Some(ref st) = store {
            st.upsert_memory(mem, None).await?;
        }
    }

    if save {
        println!(
            "✅ Successfully saved {} distilled memories to SurrealDB!",
            distilled.len()
        );
    } else {
        println!("💡 Tip: Pass `--save` to persist these memories directly into project database.");
    }

    Ok(())
}
