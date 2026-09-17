use oxide_core::error::{OxideError, Result};
use oxide_core::{ReadGuardDecision, SessionReadGuard, TokenLedger};
use oxide_parser::slicer::SymbolSlicer;
use oxide_parser::Language;
use oxide_parser::languages::get_extractor;
use std::path::Path;

pub async fn handle_read(
    project_root: &Path,
    target_file: &Path,
    symbol: Option<&str>,
    force: bool,
) -> Result<()> {
    let full_path = if target_file.is_absolute() {
        target_file.to_path_buf()
    } else {
        project_root.join(target_file)
    };

    if !full_path.exists() {
        return Err(OxideError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("File not found: {}", full_path.display()),
        )));
    }

    let content = std::fs::read_to_string(&full_path)?;
    let relative_path = target_file.to_string_lossy();
    let lang = Language::from_path(target_file);

    // Surgical AST Symbol Slicing Mode
    if let Some(target_symbol) = symbol {
        let fid = oxide_core::FileId::from_relative_path(&*relative_path);
        let language = if lang != Language::Unknown { lang } else { Language::Rust };

        if let Some(sliced) = SymbolSlicer::extract_and_slice(&fid, &full_path, &content, target_symbol, language)? {
            println!("// 🔍 Surgical Slice: {} ({:?}) [L{}-L{}]", sliced.name, sliced.kind, sliced.start_line, sliced.end_line);
            if let Some(ref doc) = sliced.doc {
                println!("/// Doc:\n/// {}", doc.replace('\n', "\n/// "));
            }
            println!("{}", sliced.code);

            // Record surgical savings in ledger
            let oxide_dir = project_root.join(".oxide");
            if oxide_dir.exists() {
                let ledger_path = oxide_dir.join("ledger.json");
                let mut ledger: TokenLedger = if ledger_path.exists() {
                    std::fs::read_to_string(&ledger_path)
                        .ok()
                        .and_then(|c| serde_json::from_str(&c).ok())
                        .unwrap_or_default()
                } else {
                    TokenLedger::default()
                };

                let saved_bytes = content.len().saturating_sub(sliced.code.len());
                ledger.record_saving("surgical_read", "symbol_slicer", content.len(), saved_bytes);

                if let Ok(json) = serde_json::to_string_pretty(&ledger) {
                    let _ = std::fs::write(ledger_path, json);
                }
            }

            return Ok(());
        } else {
            return Err(OxideError::Parser(format!(
                "Symbol '{}' not found in {}",
                target_symbol,
                target_file.display()
            )));
        }
    }

    // Standard Pre-Read Guard Full/Stub Mode
    let symbol_summary = if let Some(extractor) = get_extractor(lang) {
        let dummy_id = oxide_core::FileId::from_relative_path(&*relative_path);
        let symbols = extractor.extract_symbols(&dummy_id, &content);
        let lines: Vec<String> = symbols
            .into_iter()
            .map(|s| {
                format!(
                    "  - {:?} {} (L{}-L{})",
                    s.kind, s.name, s.start_line, s.end_line
                )
            })
            .collect();
        if lines.is_empty() {
            None
        } else {
            Some(lines.join("\n"))
        }
    } else {
        None
    };

    let oxide_dir = project_root.join(".oxide");
    let guard_file = oxide_dir.join("read_guard.json");

    let mut guard: SessionReadGuard = if guard_file.exists() {
        std::fs::read_to_string(&guard_file)
            .ok()
            .and_then(|c| serde_json::from_str(&c).ok())
            .unwrap_or_default()
    } else {
        SessionReadGuard::default()
    };

    let decision = guard.process_read(target_file, &content, symbol_summary.as_deref(), force);

    // Persist guard state
    if oxide_dir.exists()
        && let Ok(json) = serde_json::to_string_pretty(&guard)
    {
        let _ = std::fs::write(guard_file, json);
    }

    match decision {
        ReadGuardDecision::FullRead { content: c, .. } => {
            println!("{}", c);
        }
        ReadGuardDecision::DuplicateSuppressed {
            stub_message,
            byte_size,
            ..
        } => {
            println!("{}", stub_message);

            // Record token saving
            if oxide_dir.exists() {
                let ledger_path = oxide_dir.join("ledger.json");
                let mut ledger: TokenLedger = if ledger_path.exists() {
                    std::fs::read_to_string(&ledger_path)
                        .ok()
                        .and_then(|c| serde_json::from_str(&c).ok())
                        .unwrap_or_default()
                } else {
                    TokenLedger::default()
                };

                let saved_bytes = byte_size.saturating_sub(stub_message.len());
                ledger.record_saving("cli_read", "read_guard", byte_size, saved_bytes);

                if let Ok(json) = serde_json::to_string_pretty(&ledger) {
                    let _ = std::fs::write(ledger_path, json);
                }
            }
        }
    }

    Ok(())
}
