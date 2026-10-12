use oxide_core::error::Result;
use std::fs;
use std::path::Path;

pub async fn handle_install_hook(project_root: &Path, tool: &str) -> Result<()> {
    let binary_path = std::env::current_exe()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| "oxide-embed".to_string());

    println!("⚡ Installing Oxide-embed agent hooks for tool: `{}`", tool);

    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());

    if tool == "all" || tool == "antigravity" || tool == "gemini" {
        let agy_cfg_path = Path::new(&home).join(".gemini/antigravity-ide/mcp_config.json");
        if let Some(parent) = agy_cfg_path.parent() {
            let _ = fs::create_dir_all(parent);
        }

        let mut root_json: serde_json::Value = if agy_cfg_path.exists() {
            fs::read_to_string(&agy_cfg_path)
                .ok()
                .and_then(|c| serde_json::from_str(&c).ok())
                .unwrap_or_else(|| serde_json::json!({}))
        } else {
            serde_json::json!({})
        };

        let servers = root_json.as_object_mut().and_then(|obj| {
            if !obj.contains_key("mcpServers") {
                obj.insert("mcpServers".to_string(), serde_json::json!({}));
            }
            obj.get_mut("mcpServers").and_then(|v| v.as_object_mut())
        });

        if let Some(servers_map) = servers {
            servers_map.insert(
                "oxide-embed".to_string(),
                serde_json::json!({
                    "command": binary_path,
                    "args": ["mcp-serve"],
                    "env": {
                        "RUST_LOG": "info"
                    }
                }),
            );
            if let Ok(content) = serde_json::to_string_pretty(&root_json) {
                let _ = fs::write(&agy_cfg_path, content);
                println!(
                    "  ✅ Configured Google Antigravity / Gemini MCP in {}",
                    agy_cfg_path.display()
                );
            }
        }
    }

    if tool == "all" || tool == "claude" {
        let claude_cfg = Path::new(&home).join(".claude.json");
        let claude_entry = serde_json::json!({
            "mcpServers": {
                "oxide-embed": {
                    "command": binary_path,
                    "args": ["mcp-serve"]
                }
            }
        });
        println!("  ✅ Claude Code MCP snippet:");
        println!(
            "     claude mcp add oxide-embed -- {} mcp-serve",
            binary_path
        );
        if !claude_cfg.exists()
            && let Ok(content) = serde_json::to_string_pretty(&claude_entry)
        {
            let _ = fs::write(&claude_cfg, content);
            println!("     Created {}", claude_cfg.display());
        }
    }

    if tool == "all" || tool == "cursor" {
        let cursor_dir = project_root.join(".cursor");
        if fs::create_dir_all(&cursor_dir).is_ok() {
            let cursor_mcp = cursor_dir.join("mcp.json");
            let cursor_json = serde_json::json!({
                "mcpServers": {
                    "oxide-embed": {
                        "command": binary_path,
                        "args": ["mcp-serve"]
                    }
                }
            });
            if let Ok(content) = serde_json::to_string_pretty(&cursor_json) {
                let _ = fs::write(&cursor_mcp, content);
                println!(
                    "  ✅ Configured Cursor project MCP at {}",
                    cursor_mcp.display()
                );
            }
        }
    }

    println!("\n🚀 Oxide-Embed hooks successfully installed and ready!");
    Ok(())
}
