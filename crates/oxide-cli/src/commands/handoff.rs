use oxide_core::HandoffCheckpoint;
use oxide_core::error::{OxideError, Result};
use std::path::Path;

pub async fn handle_handoff(
    project_root: &Path,
    goal: Option<String>,
    next_action: Option<String>,
) -> Result<()> {
    let oxide_dir = project_root.join(".oxide");
    if !oxide_dir.exists() {
        return Err(OxideError::NotInitialized(project_root.to_path_buf()));
    }

    let session_id = format!("session-{}", chrono::Utc::now().timestamp());
    let goal_str = goal.unwrap_or_else(|| "Active development and implementation".to_string());
    let next_str = next_action.unwrap_or_else(|| "Continue pending tasks".to_string());

    let mut checkpoint = HandoffCheckpoint::new(&session_id, &goal_str, &next_str);

    // Auto-detect git modified files if available
    if let Ok(output) = std::process::Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(project_root)
        .output()
    {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            if line.len() > 3 {
                checkpoint.modified_files.push(line[3..].trim().to_string());
            }
        }
    }

    let saved_path = checkpoint.save_to_file(project_root)?;
    println!(
        "Session handover checkpoint successfully written to: {}",
        saved_path.display()
    );
    println!("\nPreview:\n--------------------------------------------------");
    println!("{}", checkpoint.to_markdown());
    println!("--------------------------------------------------");

    Ok(())
}
