use oxide_core::budget::TokenEstimator;
use oxide_core::error::Result;
use std::collections::BTreeMap;
use std::path::Path;
use walkdir::WalkDir;

pub async fn handle_tokenmap(project_root: &Path, max_depth: usize) -> Result<()> {
    println!(
        "🌳 Project Token & Context Map ({})\n",
        project_root.display()
    );

    let mut dir_tokens: BTreeMap<String, usize> = BTreeMap::new();
    let mut file_tokens: Vec<(String, usize)> = Vec::new();
    let mut total_tokens = 0;

    for entry in WalkDir::new(project_root)
        .max_depth(max_depth)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        let str_rep = path.to_string_lossy();
        if str_rep.contains("/target/")
            || str_rep.contains("/.git/")
            || str_rep.contains("/.oxide/")
            || str_rep.contains("/node_modules/")
        {
            continue;
        }

        if path.is_file()
            && let Ok(content) = std::fs::read_to_string(path)
        {
            let tokens = TokenEstimator::estimate_tokens(&content);
            total_tokens += tokens;

            let rel = path.strip_prefix(project_root).unwrap_or(path);
            let rel_str = rel.to_string_lossy().to_string();
            file_tokens.push((rel_str.clone(), tokens));

            if let Some(parent) = rel.parent() {
                let parent_str = parent.to_string_lossy().to_string();
                let key = if parent_str.is_empty() {
                    ".".to_string()
                } else {
                    parent_str
                };
                *dir_tokens.entry(key).or_insert(0) += tokens;
            }
        }
    }

    println!("📁 Folder Token Densities:");
    for (dir, tokens) in &dir_tokens {
        let pct = if total_tokens > 0 {
            (*tokens as f64 / total_tokens as f64) * 100.0
        } else {
            0.0
        };
        println!("  {:<35} {:>8} tokens  ({:>5.1}%)", dir, tokens, pct);
    }

    println!("\n📄 Top 10 Largest Context Files:");
    file_tokens.sort_by_key(|b| std::cmp::Reverse(b.1));
    for (file, tokens) in file_tokens.iter().take(10) {
        let pct = if total_tokens > 0 {
            (*tokens as f64 / total_tokens as f64) * 100.0
        } else {
            0.0
        };
        println!("  {:<45} {:>8} tokens  ({:>5.1}%)", file, tokens, pct);
    }

    println!(
        "\n⚡ Total Workspace Weight: {} tokens (~{:.2} KB content)",
        total_tokens,
        total_tokens as f64 * 4.0 / 1024.0
    );

    Ok(())
}
