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
                        },
                        {
                            "name": "oxide_remember",
                            "description": "Store a typed semantic memory (instruction, decision, preference, fact, goal, learning, etc.) in the project's long-term memory.",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "content": { "type": "string", "description": "Memory text or assertion to remember" },
                                    "kind": { "type": "string", "description": "Category: instruction, fact, decision, goal, commitment, preference, relationship, context, event, learning, observation, artifact, error" },
                                    "title": { "type": "string", "description": "Optional descriptive title" },
                                    "tags": { "type": "array", "items": { "type": "string" }, "description": "Optional tags" },
                                    "symbol_ref": { "type": "string", "description": "Target symbol governed or referenced by this memory" },
                                    "auto_resolve": { "type": "boolean", "description": "Automatically supersede prior conflicting memories" }
                                },
                                "required": ["content"]
                            }
                        },
                        {
                            "name": "oxide_recall",
                            "description": "Recall typed semantic memories with category, vector similarity, and temporal point-in-time filters.",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "query": { "type": "string", "description": "Topic, question, or search text" },
                                    "kind": { "type": "string", "description": "Optional category filter" },
                                    "tags": { "type": "array", "items": { "type": "string" }, "description": "Optional tag filter" },
                                    "as_of": { "type": "string", "description": "Optional RFC3339 point-in-time timestamp" },
                                    "budget": { "type": "integer", "description": "Token budget ceiling" },
                                    "limit": { "type": "integer", "description": "Max results (default 5)" }
                                },
                                "required": ["query"]
                            }
                        },
                        {
                            "name": "oxide_get_rules",
                            "description": "Returns active consolidated architectural constraints, decisions, and instructions governing the codebase.",
                            "inputSchema": {
                                "type": "object",
                                "properties": {}
                            }
                        },
                        {
                            "name": "oxide_answer",
                            "description": "Generate direct grounded answers synthesizing recalled memories, rules, and live AST code graph.",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "question": { "type": "string", "description": "Question or query to answer" },
                                    "kind": { "type": "string", "description": "Optional category filter" },
                                    "budget": { "type": "integer", "description": "Token budget ceiling (default 1000)" }
                                },
                                "required": ["question"]
                            }
                        },
                        {
                            "name": "oxide_distill",
                            "description": "Distills decisions, learnings, and resolved errors from recent git commit history into project memory.",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "commits": { "type": "string", "description": "Git commit range (e.g. HEAD~5..HEAD)" },
                                    "save": { "type": "boolean", "description": "Persist directly to database (default true)" }
                                }
                            }
                        },
                        {
                            "name": "oxide_sync_memories",
                            "description": "Synchronizes memories bidirectionally with Obsidian / Markdown notes in .oxide/memories/.",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "direction": { "type": "string", "description": "Direction: export, import, bidirectional (default)" }
                                }
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

            "oxide_remember" => {
                let content = args.get("content").and_then(|v| v.as_str()).unwrap_or("");
                let kind_str = args.get("kind").and_then(|v| v.as_str());
                let title = args.get("title").and_then(|v| v.as_str());
                let symbol_ref = args.get("symbol_ref").and_then(|v| v.as_str());
                let auto_resolve = args
                    .get("auto_resolve")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true);

                let tags: Vec<String> = args
                    .get("tags")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|x| x.as_str().map(|s| s.to_string()))
                            .collect()
                    })
                    .unwrap_or_default();

                crate::commands::handle_remember(
                    &self.workspace_dir,
                    content,
                    kind_str,
                    title,
                    &tags,
                    symbol_ref,
                    auto_resolve,
                )
                .await?;

                Ok(format!(
                    "Successfully remembered: \"{}\"",
                    title.unwrap_or(content)
                ))
            }

            "oxide_recall" => {
                let query = args.get("query").and_then(|v| v.as_str()).unwrap_or("");
                let kind_str = args.get("kind").and_then(|v| v.as_str());
                let as_of_str = args.get("as_of").and_then(|v| v.as_str());
                let budget = args
                    .get("budget")
                    .and_then(|v| v.as_u64())
                    .map(|b| b as usize);
                let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(5) as usize;

                let tags: Vec<String> = args
                    .get("tags")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|x| x.as_str().map(|s| s.to_string()))
                            .collect()
                    })
                    .unwrap_or_default();

                if let Some(st) = store {
                    let kind_opt = match kind_str {
                        Some(k) => Some(k.parse::<oxide_core::MemoryKind>()?),
                        None => None,
                    };
                    let as_of_opt = match as_of_str {
                        Some(s) => chrono::DateTime::parse_from_rfc3339(s)
                            .map(|dt| dt.with_timezone(&chrono::Utc))
                            .ok(),
                        None => None,
                    };

                    let embedder = CandleBertEmbedder::new_offline();
                    let query_emb = embedder.embed(query).await.ok();

                    let memories = st
                        .recall_memories(query_emb.as_deref(), kind_opt, &tags, as_of_opt, limit)
                        .await?;

                    let mut out = String::new();
                    let mut total_tokens = 0;
                    for m in memories {
                        let block = format!(
                            "[{}] {} ({})\n   {}\n",
                            m.kind.as_str().to_uppercase(),
                            m.title,
                            m.created_at.format("%Y-%m-%d %H:%M"),
                            m.content
                        );
                        let cost = oxide_core::budget::TokenEstimator::estimate_tokens(&block);
                        if let Some(b) = budget
                            && total_tokens + cost > b
                        {
                            break;
                        }
                        total_tokens += cost;
                        out.push_str(&block);
                        out.push('\n');
                    }

                    if out.is_empty() {
                        Ok("No matching memories found.".into())
                    } else {
                        Ok(out)
                    }
                } else {
                    Err(OxideError::Config("Oxide database not found.".into()))
                }
            }

            "oxide_get_rules" => {
                if let Some(st) = store {
                    let rules = st.list_active_rules().await?;
                    if rules.is_empty() {
                        Ok("No active rules found.".into())
                    } else {
                        let mut out = format!(
                            "📜 Active Architectural Rules & Decisions (Total: {})\n\n",
                            rules.len()
                        );
                        for r in rules {
                            out.push_str(&format!(
                                "- [{}] **{}** ({})\n  {}\n",
                                r.kind.as_str().to_uppercase(),
                                r.title,
                                r.id,
                                r.content
                            ));
                        }
                        Ok(out)
                    }
                } else {
                    Err(OxideError::Config("Oxide database not found.".into()))
                }
            }

            "oxide_answer" => {
                if let Some(st) = store {
                    let question = args.get("question").and_then(|v| v.as_str()).unwrap_or("");
                    let kind_str = args.get("kind").and_then(|v| v.as_str());
                    let budget =
                        args.get("budget").and_then(|v| v.as_u64()).unwrap_or(1000) as usize;

                    let kind_opt: Option<MemoryKind> = match kind_str {
                        Some(k) => Some(k.parse()?),
                        None => None,
                    };

                    let embedder = CandleBertEmbedder::new_offline();
                    let query_emb = embedder.embed(question).await.ok();
                    let memories = st
                        .recall_memories(query_emb.as_deref(), kind_opt, &[], None, 5)
                        .await?;

                    let active_rules = st.list_active_rules().await?;
                    let rule_strings: Vec<String> = active_rules
                        .into_iter()
                        .map(|r| {
                            format!(
                                "[{}] {}: {}",
                                r.kind.as_str().to_uppercase(),
                                r.title,
                                r.content
                            )
                        })
                        .collect();

                    let mut code_snippets = Vec::new();
                    let query = SearchQuery {
                        text: question.to_string(),
                        embedding: query_emb,
                        limit: 3,
                        language: None,
                    };
                    if let Ok(hits) = st.search(&query).await {
                        for hit in hits {
                            code_snippets.push(format!(
                                "File: {} (L{}-L{})\n{}",
                                hit.file_path, hit.start_line, hit.end_line, hit.text
                            ));
                        }
                    }

                    let answer = oxide_core::answer::AnswerSynthesizer::answer(
                        question,
                        &memories,
                        &rule_strings,
                        &code_snippets,
                        budget,
                    );

                    let mut full_ans = answer.answer;
                    if !answer.citations.is_empty() {
                        full_ans.push_str("\n\n📚 Grounded Citations:\n");
                        for c in answer.citations {
                            full_ans.push_str(&format!(
                                "- [{}] {} (`{}`)\n",
                                c.source_type.to_uppercase(),
                                c.title,
                                c.source_id
                            ));
                        }
                    }

                    Ok(full_ans)
                } else {
                    Err(OxideError::Config("Oxide database not found.".into()))
                }
            }

            "oxide_distill" => {
                let commits = args
                    .get("commits")
                    .and_then(|v| v.as_str())
                    .unwrap_or("HEAD~10..HEAD");
                let save = args.get("save").and_then(|v| v.as_bool()).unwrap_or(true);

                let manifest =
                    oxide_core::manifest::OxideManifest::load_from_dir(&self.workspace_root)
                        .map(|m| m.project_id)
                        .unwrap_or_else(|_| ProjectId::new_v7());

                let output = std::process::Command::new("git")
                    .arg("-C")
                    .arg(&self.workspace_root)
                    .arg("log")
                    .arg(commits)
                    .arg("--pretty=format:%h|%an|%s%n%b%n---COMMIT_END---")
                    .output()
                    .map_err(|e| OxideError::Config(format!("Failed to run git log: {e}")))?;

                if !output.status.success() {
                    return Ok("Git command failed or not a git repository.".into());
                }

                let log_str = String::from_utf8_lossy(&output.stdout);
                let distilled =
                    oxide_core::distiller::MemoryDistiller::distill_git_log(manifest, &log_str);

                if distilled.is_empty() {
                    return Ok("No conventional commit patterns detected in range.".into());
                }

                if save && let Some(st) = store {
                    for mem in &distilled {
                        st.upsert_memory(mem, None).await?;
                    }
                }

                let mut out = format!(
                    "🔍 Distilled {} memories from git range `{}`:\n\n",
                    distilled.len(),
                    commits
                );
                for (i, m) in distilled.iter().enumerate() {
                    out.push_str(&format!(
                        "{}. [{}] **{}**\n   {}\n\n",
                        i + 1,
                        m.kind.as_str().to_uppercase(),
                        m.title,
                        m.content
                    ));
                }

                Ok(out)
            }

            "oxide_sync_memories" => {
                if let Some(st) = store {
                    let direction = args
                        .get("direction")
                        .and_then(|v| v.as_str())
                        .unwrap_or("bidirectional");
                    let memories_dir = self.workspace_root.join(".oxide").join("memories");
                    let manifest =
                        oxide_core::manifest::OxideManifest::load_from_dir(&self.workspace_root)
                            .map(|m| m.project_id)
                            .unwrap_or_else(|_| ProjectId::new_v7());

                    match direction {
                        "export" => {
                            let memories = st.list_all_memories().await?;
                            let paths =
                                oxide_core::markdown_sync::MarkdownMemorySync::export_to_dir(
                                    &memories_dir,
                                    &memories,
                                )?;
                            Ok(format!(
                                "Exported {} memories across {} markdown files in `.oxide/memories/`.",
                                memories.len(),
                                paths.len()
                            ))
                        }
                        "import" => {
                            let mut total = 0;
                            if memories_dir.exists() {
                                for entry in std::fs::read_dir(&memories_dir)? {
                                    let entry = entry?;
                                    let path = entry.path();
                                    if path.extension().and_then(|s| s.to_str()) == Some("md") {
                                        let records = oxide_core::markdown_sync::MarkdownMemorySync::import_from_file(manifest.clone(), &path)?;
                                        total += records.len();
                                        st.sync_all_memories(&records).await?;
                                    }
                                }
                            }
                            Ok(format!(
                                "Imported {} memory records from `.oxide/memories/`.",
                                total
                            ))
                        }
                        "bidirectional" | _ => {
                            let mut imported = Vec::new();
                            if memories_dir.exists() {
                                for entry in std::fs::read_dir(&memories_dir)? {
                                    let entry = entry?;
                                    let path = entry.path();
                                    if path.extension().and_then(|s| s.to_str()) == Some("md") {
                                        if let Ok(records) = oxide_core::markdown_sync::MarkdownMemorySync::import_from_file(manifest.clone(), &path) {
                                            imported.extend(records);
                                        }
                                    }
                                }
                                if !imported.is_empty() {
                                    st.sync_all_memories(&imported).await?;
                                }
                            }
                            let all = st.list_all_memories().await?;
                            let paths =
                                oxide_core::markdown_sync::MarkdownMemorySync::export_to_dir(
                                    &memories_dir,
                                    &all,
                                )?;
                            Ok(format!(
                                "Bidirectionally synchronized {} memories across {} files in `.oxide/memories/`.",
                                all.len(),
                                paths.len()
                            ))
                        }
                    }
                } else {
                    Err(OxideError::Config("Oxide database not found.".into()))
                }
            }

            _ => Err(OxideError::Config(format!("Unknown tool: {}", tool_name))),
        }
    }
}
