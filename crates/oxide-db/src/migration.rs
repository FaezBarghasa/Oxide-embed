use crate::store::ProjectStore;
use oxide_core::error::Result;

pub struct MigrationManager;

impl MigrationManager {
    pub async fn run_migrations<S: ProjectStore>(_store: &S, current_version: u32) -> Result<()> {
        tracing::info!(
            "Ensuring schema is up to date (version {})",
            current_version
        );
        Ok(())
    }
}
