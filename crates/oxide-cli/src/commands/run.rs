use oxide_core::error::{OxideError, Result};
use oxide_core::{TerminalCondenser, TokenLedger};
use std::path::Path;
use std::process::Command;

pub async fn handle_run(project_root: &Path, command_args: &[String]) -> Result<()> {
    if command_args.is_empty() {
        return Err(OxideError::Config("No command provided to run".into()));
    }

    let oxide_dir = project_root.join(".oxide");
    let program = &command_args[0];
    let args = &command_args[1..];

    let full_cmd_str = command_args.join(" ");

    let output = Command::new(program)
        .args(args)
        .current_dir(project_root)
        .output()
        .map_err(|e| {
            OxideError::Config(format!("Failed to execute command '{}': {}", program, e))
        })?;

    let stdout_str = String::from_utf8_lossy(&output.stdout);
    let stderr_str = String::from_utf8_lossy(&output.stderr);
    let exit_code = output.status.code().unwrap_or(-1);

    let condenser = TerminalCondenser::default();
    let condensed = condenser.condense(
        if oxide_dir.exists() {
            &oxide_dir
        } else {
            project_root
        },
        &full_cmd_str,
        &stdout_str,
        &stderr_str,
        exit_code,
    )?;

    println!("{}", condensed.condensed_text);

    // Save token saving metric if .oxide exists
    if oxide_dir.exists() && condensed.original_bytes > condensed.condensed_bytes {
        let ledger_path = oxide_dir.join("ledger.json");
        let mut ledger: TokenLedger = if ledger_path.exists() {
            std::fs::read_to_string(&ledger_path)
                .ok()
                .and_then(|c| serde_json::from_str(&c).ok())
                .unwrap_or_default()
        } else {
            TokenLedger::default()
        };

        ledger.record_saving(
            "cli_run",
            "condenser",
            condensed.original_bytes,
            condensed.original_bytes - condensed.condensed_bytes,
        );

        if let Ok(json) = serde_json::to_string_pretty(&ledger) {
            let _ = std::fs::write(ledger_path, json);
        }
    }

    if exit_code != 0 {
        std::process::exit(exit_code);
    }

    Ok(())
}
