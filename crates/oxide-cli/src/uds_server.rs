use oxide_core::error::{OxideError, Result};
use oxide_core::id::ProjectId;
use oxide_db::traversal::GraphTraversalService;
use oxide_db::{ProjectStore, SearchQuery, SurrealProjectStore};
use oxide_ml::Embedder;
use oxide_ml::candle_embedder::CandleBertEmbedder;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};

#[derive(Debug, Serialize, Deserialize)]
pub struct UdsRequest {
    pub id: u64,
    pub method: String,
    #[serde(default)]
    pub params: Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UdsResponse {
    pub id: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

pub struct UdsServer {
    workspace_dir: PathBuf,
    socket_path: PathBuf,
}

impl UdsServer {
    pub fn new(workspace_dir: impl Into<PathBuf>) -> Self {
        let ws: PathBuf = workspace_dir.into();
        let socket_path = ws.join(".oxide").join("oxide.sock");
        Self {
            workspace_dir: ws,
            socket_path,
        }
    }

    pub async fn run(&self) -> Result<()> {
        let oxide_dir = self.workspace_dir.join(".oxide");
        if !oxide_dir.exists() {
            std::fs::create_dir_all(&oxide_dir)?;
        }

        if self.socket_path.exists() {
            let _ = std::fs::remove_file(&self.socket_path);
        }

        let listener = UnixListener::bind(&self.socket_path)
            .map_err(|e| OxideError::Io(std::io::Error::other(format!("Failed to bind UDS socket: {e}"))))?;

        println!("⚡ [oxide-uds] Unix Domain Socket server listening at {}", self.socket_path.display());

        let db_path = oxide_core::resolve_db_path(&self.workspace_dir)?;
        let store = SurrealProjectStore::open(&db_path).await?;
        let embedder = CandleBertEmbedder::new_offline();

        let manifest = oxide_core::OxideManifest::load_from_dir(&oxide_dir)?;
        let project_id = manifest.project_id;

        loop {
            match listener.accept().await {
                Ok((stream, _addr)) => {
                    let store_clone = store.clone();
                    let embedder_clone = CandleBertEmbedder::new_offline();
                    let pid = project_id.clone();
                    let ws = self.workspace_dir.clone();

                    tokio::spawn(async move {
                        if let Err(e) = Self::handle_client(stream, ws, pid, store_clone, embedder_clone).await {
                            eprintln!("⚠️ [oxide-uds] Client connection error: {e}");
                        }
                    });
                }
                Err(e) => {
                    eprintln!("⚠️ [oxide-uds] Accept failed: {e}");
                }
            }
        }
    }

    async fn handle_client(
        stream: UnixStream,
        workspace_dir: PathBuf,
        project_id: ProjectId,
        store: SurrealProjectStore,
        embedder: CandleBertEmbedder,
    ) -> Result<()> {
        let (reader, mut writer) = stream.into_split();
        let mut lines = BufReader::new(reader).lines();

        while let Some(line) = lines.next_line().await? {
            if line.trim().is_empty() {
                continue;
            }

            let req: UdsRequest = match serde_json::from_str(&line) {
                Ok(r) => r,
                Err(e) => {
                    let err_resp = UdsResponse {
                        id: 0,
                        result: None,
                        error: Some(format!("Invalid JSON-RPC request: {e}")),
                    };
                    writer.write_all(serde_json::to_string(&err_resp)?.as_bytes()).await?;
                    writer.write_all(b"\n").await?;
                    writer.flush().await?;
                    continue;
                }
            };

            let resp = Self::dispatch_method(&req, &workspace_dir, &project_id, &store, &embedder).await;
            let serialized = serde_json::to_string(&resp)?;
            writer.write_all(serialized.as_bytes()).await?;
            writer.write_all(b"\n").await?;
            writer.flush().await?;
        }

        Ok(())
    }

    async fn dispatch_method(
        req: &UdsRequest,
        workspace_dir: &Path,
        project_id: &ProjectId,
        store: &SurrealProjectStore,
        embedder: &CandleBertEmbedder,
    ) -> UdsResponse {
        match req.method.as_str() {
            "ping" => UdsResponse {
                id: req.id,
                result: Some(json!({ "status": "ok", "engine": "Oxide-Embed", "version": "0.3.0" })),
                error: None,
            },
            "stair_search" => {
                let query = req.params.get("query").and_then(|v| v.as_str()).unwrap_or("");
                let limit = req.params.get("limit").and_then(|v| v.as_u64()).unwrap_or(10) as usize;

                match store.stair_search(query, limit).await {
                    Ok(hits) => UdsResponse {
                        id: req.id,
                        result: Some(json!(hits)),
                        error: None,
                    },
                    Err(e) => UdsResponse {
                        id: req.id,
                        result: None,
                        error: Some(e.to_string()),
                    },
                }
            }
            "graph_impact" => {
                let symbol = req.params.get("symbol").and_then(|v| v.as_str()).unwrap_or("");
                match GraphTraversalService::get_subgraph(store, symbol, 2).await {
                    Ok(subgraph) => UdsResponse {
                        id: req.id,
                        result: Some(json!(subgraph)),
                        error: None,
                    },
                    Err(e) => UdsResponse {
                        id: req.id,
                        result: None,
                        error: Some(e.to_string()),
                    },
                }
            }
            "memory_remember" => {
                let content = req.params.get("content").and_then(|v| v.as_str()).unwrap_or("");
                let kind_str = req.params.get("kind").and_then(|v| v.as_str()).unwrap_or("instruction");
                let kind = match kind_str {
                    "instruction" => oxide_core::MemoryKind::Instruction,
                    "decision" => oxide_core::MemoryKind::Decision,
                    "preference" => oxide_core::MemoryKind::Preference,
                    "fact" => oxide_core::MemoryKind::Fact,
                    _ => oxide_core::MemoryKind::Instruction,
                };
                let title = req.params.get("title").and_then(|v| v.as_str()).unwrap_or("Memory").to_string();

                let mem = oxide_core::MemoryRecord::new(project_id.clone(), kind, title, content.to_string());
                let emb = embedder.embed(content).await.ok();

                match store.upsert_memory(&mem, emb).await {
                    Ok(()) => UdsResponse {
                        id: req.id,
                        result: Some(json!({ "id": mem.id.0, "status": "stored" })),
                        error: None,
                    },
                    Err(e) => UdsResponse {
                        id: req.id,
                        result: None,
                        error: Some(e.to_string()),
                    },
                }
            }
            "memory_recall" => {
                let query = req.params.get("query").and_then(|v| v.as_str()).unwrap_or("");
                let limit = req.params.get("limit").and_then(|v| v.as_u64()).unwrap_or(5) as usize;
                let emb = embedder.embed(query).await.ok();

                match store.recall_memories(emb.as_deref(), None, &[], None, limit).await {
                    Ok(recs) => UdsResponse {
                        id: req.id,
                        result: Some(json!(recs)),
                        error: None,
                    },
                    Err(e) => UdsResponse {
                        id: req.id,
                        result: None,
                        error: Some(e.to_string()),
                    },
                }
            }
            _ => UdsResponse {
                id: req.id,
                result: None,
                error: Some(format!("Unknown method '{}'", req.method)),
            },
        }
    }
}
