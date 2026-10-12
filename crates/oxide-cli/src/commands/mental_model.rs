use oxide_core::error::{OxideError, Result};
use oxide_core::{MentalModel, OxideManifest};
use oxide_db::{ProjectStore, SurrealProjectStore};
use oxide_ml::{CandleBertEmbedder, Embedder};
use std::path::Path;

pub async fn handle_model_create(
    project_root: &Path,
    topic: &str,
    question: &str,
    answer: &str,
    symbols: Vec<String>,
    files: Vec<String>,
) -> Result<()> {
    let oxide_dir = project_root.join(".oxide");
    if !oxide_dir.exists() {
        return Err(OxideError::NotInitialized(project_root.to_path_buf()));
    }

    let manifest = OxideManifest::load_from_dir(&oxide_dir)?;
    let db_path = oxide_dir.join(&manifest.storage.path);
    let store = SurrealProjectStore::open(&db_path).await?;

    let mut model = MentalModel::new(topic, question, answer);
    model.referenced_symbols = symbols;
    model.referenced_files = files;

    let embedder = CandleBertEmbedder::new_offline();
    let text_to_embed = format!("{}: {} {}", topic, question, answer);
    if let Ok(emb) = embedder.embed(&text_to_embed).await {
        model.embedding = Some(emb);
    }

    store.upsert_mental_model(&model).await?;
    println!("✅ Created Mental Model '{}': {}", model.topic, model.question);

    Ok(())
}

pub async fn handle_model_list(project_root: &Path) -> Result<()> {
    let oxide_dir = project_root.join(".oxide");
    if !oxide_dir.exists() {
        return Err(OxideError::NotInitialized(project_root.to_path_buf()));
    }

    let manifest = OxideManifest::load_from_dir(&oxide_dir)?;
    let db_path = oxide_dir.join(&manifest.storage.path);
    let store = SurrealProjectStore::open(&db_path).await?;

    let models = store.list_mental_models().await?;
    if models.is_empty() {
        println!(
            "No Mental Models found in project. Create one using `oxide-embed model create <topic>`."
        );
        return Ok(());
    }

    println!("🧠 Active Mental Models ({})", models.len());
    println!(
        "{:<20} {:<10} {:<40} {:<20}",
        "TOPIC", "STATUS", "QUESTION", "LAST REFRESHED"
    );
    println!("{:-<92}", "");

    for m in models {
        let status = if m.is_stale { "⚠️ STALE" } else { "✅ FRESH" };
        let display_q = if m.question.len() > 37 {
            format!("{}...", &m.question[..37])
        } else {
            m.question
        };
        println!(
            "{:<20} {:<10} {:<40} {:<20}",
            m.topic,
            status,
            display_q,
            m.last_refreshed_at.format("%Y-%m-%d %H:%M")
        );
    }

    Ok(())
}

pub async fn handle_model_get(project_root: &Path, topic: &str) -> Result<()> {
    let oxide_dir = project_root.join(".oxide");
    if !oxide_dir.exists() {
        return Err(OxideError::NotInitialized(project_root.to_path_buf()));
    }

    let manifest = OxideManifest::load_from_dir(&oxide_dir)?;
    let db_path = oxide_dir.join(&manifest.storage.path);
    let store = SurrealProjectStore::open(&db_path).await?;

    if let Some(model) = store.get_mental_model(topic).await? {
        println!("{}", model.to_markdown());
    } else {
        println!("❌ No Mental Model found for topic '{}'", topic);
    }

    Ok(())
}
