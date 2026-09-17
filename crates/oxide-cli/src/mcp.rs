use oxide_core::TerminalCondenser;
use oxide_core::budget::BudgetCandidate;
use oxide_core::context_builder::ContextSynthesizer;
use oxide_core::error::{OxideError, Result};
use oxide_core::handoff::HandoffCheckpoint;
use oxide_core::memify::CerebrumRule;
use oxide_db::traversal::GraphTraversalService;
use oxide_db::{ProjectStore, SearchQuery, SurrealProjectStore};
use oxide_ml::Embedder;
use oxide_ml::candle_embedder::CandleBertEmbedder;
use oxide_parser::Language;
use oxide_parser::slicer::SymbolSlicer;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::io::{self, BufRead, Write};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    id: Option<Value>,
    method: String,
    #[serde(default)]
    params: Option<Value>,
}

#[derive(Debug, Serialize, Deserialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonRpcError>,
}

#[derive(Debug, Serialize, Deserialize)]
struct JsonRpcError {
    code: i64,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<Value>,
}

pub struct McpServer {
    workspace_dir: PathBuf,
}

impl McpServer {
    pub fn new(workspace_dir: impl Into<PathBuf>) -> Self {
        Self {
            workspace_dir: workspace_dir.into(),
        }
    }

    pub async fn run_stdio(&self) -> Result<()> {
        let stdin = io::stdin();
        let mut stdout = io::stdout();
        let reader = stdin.lock();

        let store = if let Ok(db_path) = oxide_core::resolve_db_path(&self.workspace_dir) {
            SurrealProjectStore::open(&db_path).await.ok()
        } else {
            None
        };

        for line_res in reader.lines() {
            let line = match line_res {
                Ok(l) => l,
                Err(_) => break,
            };

            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            if let Ok(req) = serde_json::from_str::<JsonRpcRequest>(trimmed) {
                let resp = self.handle_request(&req, store.as_ref()).await;
                if let Ok(json_str) = serde_json::to_string(&resp) {
                    let _ = writeln!(stdout, "{}", json_str);
                    let _ = stdout.flush();
                }
            }
        }

        Ok(())
    }

    async fn handle_request(
        &self,
        req: &JsonRpcRequest,
        store: Option<&SurrealProjectStore>,
    ) -> JsonRpcResponse {
        match req.method.as_str() {
            "initialize" => JsonRpcResponse {
                jsonrpc: "2.0".into(),
                id: req.id.clone(),
                result: Some(json!({
                    "protocolVersion": "2024-11-05",
                    "capabilities": {
                        "tools": {}
                    },
                    "serverInfo": {
                        "name": "oxide-embed",
                        "version": "0.1.0"
                    }
                })),
                error: None,
            },

            "tools/list" => JsonRpcResponse {
                jsonrpc: "2.0".into(),
                id: req.id.clone(),
                result: Some(json!({
                    "tools": [
                        {
                            "name": "oxide_context",
                            "description": "Synthesize token-budgeted cognitive context for a task (rules + active handoff + 2-hop GraphRAG).",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "task": { "type": "string", "description": "The coding or engineering task" },
                                    "budget": { "type": "integer", "description": "Maximum token budget (default 1500)" }
                                },
                                "required": ["task"]
                            }
                        },
                        {
                            "name": "oxide_read_symbol",
                            "description": "Surgically read only the exact AST symbol definition and docstring from a file.",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "file_path": { "type": "string", "description": "Relative path to file" },
                                    "symbol": { "type": "string", "description": "Symbol or method name to extract" }
                                },
                                "required": ["file_path", "symbol"]
                            }
                        },
                        {
                            "name": "oxide_search",
                            "description": "Search code symbols and knowledge graph with optional budget limit.",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "query": { "type": "string", "description": "Search text or identifier" },
                                    "budget": { "type": "integer", "description": "Token budget" },
                                    "with_graph": { "type": "boolean", "description": "Include call & doc graph" }
                                },
                                "required": ["query"]
                            }
                        },
                        {
                            "name": "oxide_explain",
                            "description": "Explain a symbol's multi-hop call graph and documentation topology.",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "symbol": { "type": "string", "description": "Symbol name to explain" },
                                    "hops": { "type": "integer", "description": "Graph traversal depth (default 2)" }
                                },
                                "required": ["symbol"]
                            }
                        },
                        {
                            "name": "oxide_condense",
                            "description": "Condense raw terminal compiler output and cache full log to disk.",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "command": { "type": "string", "description": "Executed command" },
                                    "raw_output": { "type": "string", "description": "Raw terminal stdout/stderr" },
                                    "exit_code": { "type": "integer", "description": "Exit code" }
                                },
                                "required": ["command", "raw_output"]
                            }
                        }
                    ]
                })),
                error: None,
            },

            "tools/call" => {
                let params = req.params.as_ref().cloned().unwrap_or(Value::Null);
                let tool_name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
                let arguments = params.get("arguments").cloned().unwrap_or(json!({}));

                match self.execute_tool(tool_name, &arguments, store).await {
                    Ok(val) => JsonRpcResponse {
                        jsonrpc: "2.0".into(),
                        id: req.id.clone(),
                        result: Some(json!({
                            "content": [
                                {
                                    "type": "text",
                                    "text": val
                                }
                            ]
                        })),
                        error: None,
                    },
                    Err(e) => JsonRpcResponse {
                        jsonrpc: "2.0".into(),
                        id: req.id.clone(),
                        result: None,
                        error: Some(JsonRpcError {
                            code: -32603,
                            message: e.to_string(),
                            data: None,
                        }),
                    },
                }
            }

            _ => JsonRpcResponse {
                jsonrpc: "2.0".into(),
                id: req.id.clone(),
                result: None,
                error: Some(JsonRpcError {
                    code: -32601,
                    message: format!("Method not found: {}", req.method),
                    data: None,
                }),
            },
        }
    }

    async fn execute_tool(
        &self,
        tool_name: &str,
        args: &Value,
        store: Option<&SurrealProjectStore>,
    ) -> Result<String> {
        match tool_name {
            "oxide_context" => {
                let task = args.get("task").and_then(|v| v.as_str()).unwrap_or("");
                let budget = args.get("budget").and_then(|v| v.as_u64()).unwrap_or(1500) as usize;

                let handoff_path = self.workspace_dir.join(".oxide").join("STATUS.md");
                let handoff = if handoff_path.exists() {
                    let content = std::fs::read_to_string(&handoff_path)?;
                    HandoffCheckpoint::parse_markdown(&content)
                } else {
                    None
                };

                let mut candidates = Vec::new();
                let mut rules = Vec::new();

                let cerebrum_path = self
                    .workspace_dir
                    .join(".oxide")
                    .join("docs")
                    .join("CEREBRUM.md");
                if cerebrum_path.exists() {
                    let c_text = std::fs::read_to_string(&cerebrum_path)?;
                    for (idx, line) in c_text.lines().enumerate() {
                        let trimmed = line.trim();
                        if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
                            let rule_str = trimmed
                                .trim_start_matches("- ")
                                .trim_start_matches("* ")
                                .to_string();
                            rules.push(CerebrumRule {
                                id: format!("rule_{}", idx),
                                title: format!("Rule {}", idx + 1),
                                condition_pattern: "context".into(),
                                prescribed_solution: rule_str,
                                confidence: 0.9,
                                source_incidents: vec![],
                                created_at: chrono::Utc::now().timestamp(),
                            });
                        }
                    }
                }

                if let Some(st) = store {
                    let embedder = CandleBertEmbedder::new_offline();
                    let embedding = embedder.embed(task).await.ok();
                    let query = SearchQuery {
                        text: task.to_string(),
                        embedding,
                        limit: 8,
                        language: None,
                    };
                    if let Ok(hits) = st.search(&query).await {
                        for hit in hits {
                            let title = hit
                                .symbol_name
                                .clone()
                                .unwrap_or_else(|| hit.file_path.clone());
                            let score = hit.score;
                            candidates.push(BudgetCandidate::new(
                                format!("{}:{}", hit.file_path, hit.start_line),
                                title,
                                hit.text,
                                score,
                            ));
                        }
                    }
                }

                let ctx = ContextSynthesizer::build(task, handoff, rules, candidates, budget);
                Ok(ctx.formatted_markdown)
            }

            "oxide_read_symbol" => {
                let file_path = args.get("file_path").and_then(|v| v.as_str()).unwrap_or("");
                let symbol_name = args.get("symbol").and_then(|v| v.as_str()).unwrap_or("");

                let full_path = self.workspace_dir.join(file_path);
                if !full_path.exists() {
                    return Err(OxideError::Io(std::io::Error::new(
                        std::io::ErrorKind::NotFound,
                        format!("File not found: {}", file_path),
                    )));
                }

                let content = std::fs::read_to_string(&full_path)?;
                let lang = Language::from_path(&full_path);
                let fid = oxide_core::id::FileId::from_relative_path(file_path);

                if let Some(sym) =
                    SymbolSlicer::extract_and_slice(&fid, &full_path, &content, symbol_name, lang)?
                {
                    Ok(format!(
                        "// Symbol: {} ({:?}) [L{}-L{}]\n{}",
                        sym.name, sym.kind, sym.start_line, sym.end_line, sym.code
                    ))
                } else {
                    Ok(format!(
                        "Symbol '{}' not found in {}",
                        symbol_name, file_path
                    ))
                }
            }

            "oxide_search" => {
                let query_text = args.get("query").and_then(|v| v.as_str()).unwrap_or("");
                let budget = args
                    .get("budget")
                    .and_then(|v| v.as_u64())
                    .map(|b| b as usize);
                let with_graph = args
                    .get("with_graph")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);

                if let Some(st) = store {
                    let embedder = CandleBertEmbedder::new_offline();
                    let embedding = embedder.embed(query_text).await.ok();
                    let query = SearchQuery {
                        text: query_text.to_string(),
                        embedding,
                        limit: 10,
                        language: None,
                    };
                    let hits = st.search(&query).await?;

                    let mut out = format!("# Search Results for `{}`\n\n", query_text);
                    let mut candidates = Vec::new();

                    for h in hits {
                        let mut item_text = format!(
                            "### {} (L{}-L{})\n```\n{}\n```\n",
                            h.file_path, h.start_line, h.end_line, h.text
                        );
                        if with_graph
                            && let Some(ref sname) = h.symbol_name
                            && let Ok(Some(subgraph)) =
                                GraphTraversalService::get_subgraph(st, sname, 2).await
                        {
                            item_text.push_str(&format!("\n{}", subgraph.to_compact_string()));
                        }
                        candidates.push(BudgetCandidate::new(
                            format!("{}:{}", h.file_path, h.start_line),
                            h.symbol_name.unwrap_or(h.file_path),
                            item_text,
                            h.score,
                        ));
                    }

                    if let Some(b) = budget {
                        let packed = oxide_core::budget::TokenBudgetPacker::pack(candidates, b);
                        for item in &packed.included {
                            out.push_str(&item.content);
                            out.push('\n');
                        }
                        out.push_str(&format!(
                            "\n*Packed {} items within {} token budget (used ~{} tokens)*",
                            packed.included.len(),
                            b,
                            packed.used_tokens
                        ));
                    } else {
                        for item in candidates {
                            out.push_str(&item.content);
                            out.push('\n');
                        }
                    }

                    Ok(out)
                } else {
                    Err(OxideError::Config(
                        "Oxide database not found. Run 'oxide-embed init' and 'oxide-embed index'."
                            .into(),
                    ))
                }
            }

            "oxide_explain" => {
                let symbol = args.get("symbol").and_then(|v| v.as_str()).unwrap_or("");
                let hops = args.get("hops").and_then(|v| v.as_u64()).unwrap_or(2) as usize;

                if let Some(st) = store {
                    if let Some(subgraph) =
                        GraphTraversalService::get_subgraph(st, symbol, hops).await?
                    {
                        Ok(subgraph.to_compact_string())
                    } else {
                        Ok(format!("Symbol '{}' not found in graph.", symbol))
                    }
                } else {
                    Err(OxideError::Config("Oxide database not found.".into()))
                }
            }

            "oxide_condense" => {
                let command = args.get("command").and_then(|v| v.as_str()).unwrap_or("");
                let raw_output = args
                    .get("raw_output")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let exit_code = args.get("exit_code").and_then(|v| v.as_i64()).unwrap_or(0) as i32;

                let condenser = TerminalCondenser::new(512);
                let cache_dir = self.workspace_dir.join(".oxide").join("cache").join("bash");
                let condensed =
                    condenser.condense(&cache_dir, command, raw_output, "", exit_code)?;

                let log_str = condensed.cached_log_path.unwrap_or_else(|| "N/A".into());
                let saved_pct = if condensed.original_bytes > 0 {
                    (condensed
                        .original_bytes
                        .saturating_sub(condensed.condensed_bytes) as f64
                        / condensed.original_bytes as f64)
                        * 100.0
                } else {
                    0.0
                };

                Ok(format!(
                    "Exit Code: {}\nCached: {}\n\n{}\n[Summary] Original: {} B, Condensed: {} B (Saved {:.1}%)",
                    condensed.exit_code,
                    log_str,
                    condensed.condensed_text,
                    condensed.original_bytes,
                    condensed.condensed_bytes,
                    saved_pct
                ))
            }

            _ => Err(OxideError::Config(format!("Unknown tool: {}", tool_name))),
        }
    }
}
