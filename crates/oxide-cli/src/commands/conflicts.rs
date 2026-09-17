use oxide_core::error::Result;
use oxide_core::memory::{ConflictDetector, MemoryStatus};
use oxide_core::resolve_db_path;
use oxide_db::{ProjectStore, SurrealProjectStore};
use std::path::Path;

pub async fn handle_conflicts(project_root: &Path) -> Result<()> {
    let db_path = resolve_db_path(project_root)?;
    let store = SurrealProjectStore::open(&db_path).await?;

    let rules = store.list_active_rules().await?;

    println!(
        "⚖️  Reviewing Contradictions & Conflicts in Active Rules (Total: {})\n",
        rules.len()
    );

    let mut conflict_count = 0;

    for i in 0..rules.len() {
        for j in (i + 1)..rules.len() {
            let r1 = &rules[i];
            let r2 = &rules[j];

            if r1.kind == r2.kind
                && r1.status == MemoryStatus::Active
                && r2.status == MemoryStatus::Active
                && ConflictDetector::is_potential_conflict(r1, &r2.content, r2.kind)
            {
                conflict_count += 1;
                println!("⚠️  Contradiction #{}", conflict_count);
                println!(
                    "  [Rule 1] [{}] `{}` ({})",
                    r1.kind.as_str().to_uppercase(),
                    r1.title,
                    r1.id
                );
                println!("           \"{}\"", r1.content);
                println!(
                    "  [Rule 2] [{}] `{}` ({})",
                    r2.kind.as_str().to_uppercase(),
                    r2.title,
                    r2.id
                );
                println!("           \"{}\"", r2.content);
                println!();
            }
        }
    }

    if conflict_count == 0 {
        println!("  ✅ No active contradictions detected across rules and decisions!");
    } else {
        println!(
            "  💡 Tip: Use `oxide-embed remember --auto-resolve` to automatically supersede older conflicting entries."
        );
    }

    Ok(())
}
