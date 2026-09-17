use chrono::Utc;
use oxide_core::error::{OxideError, Result};
use oxide_core::id::{FileId, ProjectId, SymbolId};
use oxide_core::manifest::OxideManifest;
use oxide_core::memory::{ConflictDetector, MemoryKind, MemoryRecord};
use oxide_core::resolve_db_path;
use oxide_db::{ProjectStore, SurrealProjectStore};
use oxide_ml::Embedder;
use oxide_ml::candle_embedder::CandleBertEmbedder;
use std::path::Path;

pub async fn handle_remember(
    project_root: &Path,
    content: &str,
    kind_str: Option<&str>,
    title_opt: Option<&str>,
    tags: &[String],
    symbol_ref: Option<&str>,
    auto_resolve: bool,
) -> Result<()> {
    let manifest_path = project_root.join(".oxide").join("manifest.json");
    let manifest = OxideManifest::load(&manifest_path).unwrap_or_default();
    let project_id = ProjectId(
        manifest
            .project_id
            .clone()
            .unwrap_or_else(uuid::Uuid::now_v7),
    );

    let db_path = resolve_db_path(project_root)?;
    let store = SurrealProjectStore::open(&db_path).await?;

    let kind: MemoryKind = match kind_str {
        Some(k) => k.parse()?,
        None => MemoryKind::Fact,
    };

    let title = title_opt.unwrap_or_else(|| {
        let first_line = content.lines().next().unwrap_or(content);
        if first_line.len() > 60 {
            &first_line[..60]
        } else {
            first_line
        }
    });

    let mut record = MemoryRecord::new(project_id, kind, title, content);
    record.tags = tags.to_vec();
    record.symbol_ref = symbol_ref.map(|s| s.to_string());

    // Generate local vector embedding
    let embedder = CandleBertEmbedder::new_offline();
    let embedding = embedder.embed(content).await.ok();

    // Check for potential contradictions / conflicts
    if let Some(ref emb) = embedding {
        let conflicts = store.find_conflicts(kind, emb, 0.85).await?;
        for mut prior in conflicts {
            if ConflictDetector::is_potential_conflict(&prior, content, kind) {
                if auto_resolve {
                    prior.supersede(&record.id);
                    store.upsert_memory(&prior, None).await?;
                    println!(
                        "  🔄 Auto-superseded conflicting prior memory `{}` [{}] ({})",
                        prior.title,
                        prior.kind.as_str(),
                        prior.id
                    );
                } else {
                    println!(
                        "  ⚠️  Potential contradiction detected with prior memory `{}` [{}] ({})",
                        prior.title,
                        prior.kind.as_str(),
                        prior.id
                    );
                    println!("     Existing: \"{}\"", prior.content);
                    println!("     New:      \"{}\"", content);
                    println!("     (Use --auto-resolve to automatically supersede prior entry)");
                }
            }
        }
    }

    let mem_id = record.id.clone();
    store.upsert_memory(&record, embedding).await?;

    // Link to symbol if symbol reference is provided
    if let Some(sym_name) = symbol_ref {
        let dummy_fid = FileId::from_relative_path("workspace");
        let sym_id = SymbolId::new(&dummy_fid, sym_name);
        let _ = store.link_memory_to_symbol(&mem_id, &sym_id, "governs").await;
    }

    println!("🧠 Remembered [{}] `{}`", kind.as_str().to_uppercase(), title);
    println!("   ID:         {}", mem_id);
    println!("   Created At: {}", Utc::now().to_rfc3339());
    if !tags.is_empty() {
        println!("   Tags:       {}", tags.join(", "));
    }
    if let Some(sym) = symbol_ref {
        println!("   Governs:    `{}`", sym);
    }

    Ok(())
}
