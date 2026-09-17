use oxide_core::TokenLedger;
use oxide_core::error::{OxideError, Result};
use std::path::Path;

pub async fn handle_report(project_root: &Path) -> Result<()> {
    let oxide_dir = project_root.join(".oxide");
    if !oxide_dir.exists() {
        return Err(OxideError::NotInitialized(project_root.to_path_buf()));
    }

    let ledger_path = oxide_dir.join("ledger.json");
    let ledger: TokenLedger = if ledger_path.exists() {
        std::fs::read_to_string(&ledger_path)
            .ok()
            .and_then(|c| serde_json::from_str(&c).ok())
            .unwrap_or_default()
    } else {
        TokenLedger::default()
    };

    println!("{}", ledger.generate_summary_report());

    Ok(())
}
