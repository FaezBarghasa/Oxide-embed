use std::path::Path;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use surrealdb::engine::local::{Db, SurrealKv};
use surrealdb::Surreal;
use oxide_core::error::{OxideError, Result};
use oxide_core::{ChunkRecord, FileRecord, SymbolRecord};
use crate::schema::INITIAL_SCHEMA_SURQL;
use crate::store::{ProjectStore, SearchHit, SearchQuery};

#[derive(Clone)]
pub struct SurrealProjectStore {
    db: Surreal<Db>,
}

fn take_vec<T: for<'de> Deserialize<'de>>(
    res: &mut surrealdb::IndexedResults,
    idx: usize,
) -> Result<Vec<T>> {
    let rows: Vec<serde_json::Value> = res.take(idx).map_err(|e| OxideError::Database(e.to_string()))?;
    rows.into_iter()
        .map(|v| serde_json::from_value(v).map_err(|e| OxideError::Database(e.to_string())))
        .collect()
}

#[derive(Debug, Serialize, Deserialize)]
struct CountResult {
    count: usize,
}

#[derive(Debug, Serialize, Deserialize)]
struct OutlineResult {
    outline: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct RawChunkResult {
    file_path: Option<String>,
    symbol_name: Option<String>,
    start_line: usize,
    end_line: usize,
    outline: Option<String>,
    text: String,
    embedding: Option<Vec<f32>>,
}

#[async_trait]
impl ProjectStore for SurrealProjectStore {
    async fn open<P: AsRef<Path> + Send + Sync>(path: P) -> Result<Self> {
        let db_path = path.as_ref();
        let db = Surreal::new::<SurrealKv>(db_path)
            .await
            .map_err(|e| OxideError::Database(e.to_string()))?;

        db.use_ns("oxide")
            .use_db("project")
            .await
            .map_err(|e| OxideError::Database(e.to_string()))?;

        db.query(INITIAL_SCHEMA_SURQL)
            .await
            .map_err(|e| OxideError::Database(e.to_string()))?;

        Ok(Self { db })
    }

    async fn upsert_file(&self, file: &FileRecord) -> Result<()> {
        let id_str = file.id.0.clone();
        let sql = r#"
            UPSERT type::thing('file', $id) SET
                project_id = $project_id,
                relative_path = $relative_path,
                language = $language,
                content_hash = $content_hash,
                size_bytes = $size_bytes,
                last_indexed_at = time::now();
        "#;

        self.db
            .query(sql)
            .bind(("id", id_str))
            .bind(("project_id", file.project_id.0.to_string()))
            .bind(("relative_path", file.relative_path.clone()))
            .bind(("language", file.language.clone()))
            .bind(("content_hash", file.content_hash.clone()))
            .bind(("size_bytes", file.size_bytes as i64))
            .await
            .map_err(|e| OxideError::Database(e.to_string()))?;

        Ok(())
    }

    async fn upsert_symbol(&self, symbol: &SymbolRecord) -> Result<()> {
        let id_str = symbol.id.0.clone();
        let sql = r#"
            UPSERT type::thing('symbol', $id) SET
                file_id = $file_id,
                kind = $kind,
                name = $name,
                qualified_name = $qualified_name,
                start_line = $start_line,
                end_line = $end_line,
                signature = $signature,
                doc = $doc,
                fingerprint = $fingerprint;
        "#;

        self.db
            .query(sql)
            .bind(("id", id_str))
            .bind(("file_id", symbol.file_id.0.clone()))
            .bind(("kind", symbol.kind.as_str()))
            .bind(("name", symbol.name.clone()))
            .bind(("qualified_name", symbol.qualified_name.clone()))
            .bind(("start_line", symbol.start_line as i64))
            .bind(("end_line", symbol.end_line as i64))
            .bind(("signature", symbol.signature.clone()))
            .bind(("doc", symbol.doc.clone()))
            .bind(("fingerprint", symbol.fingerprint.clone()))
            .await
            .map_err(|e| OxideError::Database(e.to_string()))?;

        Ok(())
    }

    async fn upsert_chunk(&self, chunk: &ChunkRecord) -> Result<()> {
        let id_str = chunk.id.0.clone();
        let sql = r#"
            UPSERT type::thing('chunk', $id) SET
                file_id = $file_id,
                symbol_id = $symbol_id,
                kind = $kind,
                text = $text,
                outline = $outline,
                content_hash = $content_hash,
                start_line = $start_line,
                end_line = $end_line,
                embedding = $embedding,
                embedding_model = $embedding_model,
                embedding_dim = $embedding_dim,
                vector_set_id = $vector_set_id,
                updated_at = time::now();
        "#;

        self.db
            .query(sql)
            .bind(("id", id_str))
            .bind(("file_id", chunk.file_id.0.clone()))
            .bind(("symbol_id", chunk.symbol_id.as_ref().map(|s| s.0.clone())))
            .bind(("kind", chunk.kind.as_str()))
            .bind(("text", chunk.text.clone()))
            .bind(("outline", chunk.outline.clone()))
            .bind(("content_hash", chunk.content_hash.clone()))
            .bind(("start_line", chunk.start_line as i64))
            .bind(("end_line", chunk.end_line as i64))
            .bind(("embedding", chunk.embedding.clone()))
            .bind(("embedding_model", chunk.embedding_model.clone()))
            .bind(("embedding_dim", chunk.embedding_dim.map(|d| d as i64)))
            .bind(("vector_set_id", chunk.vector_set_id.clone()))
            .await
            .map_err(|e| OxideError::Database(e.to_string()))?;

        Ok(())
    }

    async fn get_file_symbols(&self, file_path: &str) -> Result<Vec<SymbolRecord>> {
        let fid = oxide_core::id::FileId::from_relative_path(file_path).0;
        let sql = "SELECT * FROM symbol WHERE file_id = $fid ORDER BY start_line ASC;";
        let mut sym_res = self
            .db
            .query(sql)
            .bind(("fid", fid))
            .await
            .map_err(|e| OxideError::Database(e.to_string()))?;

        take_vec(&mut sym_res, 0)
    }

    async fn get_file_outline(&self, file_path: &str) -> Result<Option<String>> {
        let fid = oxide_core::id::FileId::from_relative_path(file_path).0;
        let sql = "SELECT outline FROM chunk WHERE file_id = $fid AND kind = 'file_outline' LIMIT 1;";
        let mut out_res = self
            .db
            .query(sql)
            .bind(("fid", fid))
            .await
            .map_err(|e| OxideError::Database(e.to_string()))?;

        let rows: Vec<OutlineResult> = take_vec(&mut out_res, 0)?;
        Ok(rows.into_iter().next().and_then(|r| r.outline))
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchHit>> {
        let sql = r#"
            SELECT 
                (SELECT VALUE relative_path FROM file WHERE id = type::thing('file', chunk.file_id))[0] AS file_path,
                (SELECT VALUE name FROM symbol WHERE id = type::thing('symbol', chunk.symbol_id))[0] AS symbol_name,
                start_line,
                end_line,
                outline,
                text,
                embedding
            FROM chunk
            LIMIT 200;
        "#;

        let mut res = self
            .db
            .query(sql)
            .await
            .map_err(|e| OxideError::Database(e.to_string()))?;

        let chunks: Vec<RawChunkResult> = take_vec(&mut res, 0)?;
        let query_lower = query.text.to_lowercase();

        let mut hits: Vec<SearchHit> = chunks
            .into_iter()
            .filter_map(|c| {
                let file_path = c.file_path.unwrap_or_else(|| "unknown".to_string());
                let text_lower = c.text.to_lowercase();
                let sym_lower = c.symbol_name.as_ref().map(|s| s.to_lowercase()).unwrap_or_default();
                let outline_lower = c.outline.as_ref().map(|s| s.to_lowercase()).unwrap_or_default();

                let mut score = 0.0f32;

                if sym_lower.contains(&query_lower) && !query_lower.is_empty() {
                    score += 0.5;
                }
                if outline_lower.contains(&query_lower) && !query_lower.is_empty() {
                    score += 0.3;
                }
                if text_lower.contains(&query_lower) && !query_lower.is_empty() {
                    score += 0.2;
                }

                if let (Some(q_emb), Some(c_emb)) = (&query.embedding, &c.embedding) {
                    let sim = cosine_similarity(q_emb, c_emb);
                    score += sim * 0.7;
                }

                if score > 0.05 || query.text.is_empty() {
                    Some(SearchHit {
                        file_path,
                        symbol_name: c.symbol_name,
                        start_line: c.start_line,
                        end_line: c.end_line,
                        score,
                        outline_or_signature: c.outline,
                        text: c.text,
                    })
                } else {
                    None
                }
            })
            .collect();

        hits.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        hits.truncate(query.limit);
        Ok(hits)
    }

    async fn count_files(&self) -> Result<usize> {
        let mut res = self
            .db
            .query("SELECT count() FROM file GROUP ALL;")
            .await
            .map_err(|e| OxideError::Database(e.to_string()))?;
        let rows: Vec<CountResult> = take_vec(&mut res, 0)?;
        Ok(rows.into_iter().next().map(|r| r.count).unwrap_or(0))
    }

    async fn count_symbols(&self) -> Result<usize> {
        let mut res = self
            .db
            .query("SELECT count() FROM symbol GROUP ALL;")
            .await
            .map_err(|e| OxideError::Database(e.to_string()))?;
        let rows: Vec<CountResult> = take_vec(&mut res, 0)?;
        Ok(rows.into_iter().next().map(|r| r.count).unwrap_or(0))
    }

    async fn count_chunks(&self) -> Result<usize> {
        let mut res = self
            .db
            .query("SELECT count() FROM chunk GROUP ALL;")
            .await
            .map_err(|e| OxideError::Database(e.to_string()))?;
        let rows: Vec<CountResult> = take_vec(&mut res, 0)?;
        Ok(rows.into_iter().next().map(|r| r.count).unwrap_or(0))
    }

    async fn export_surql(&self) -> Result<String> {
        let mut f_res = self.db.query("SELECT * FROM file;").await.map_err(|e| OxideError::Database(e.to_string()))?;
        let files: Vec<serde_json::Value> = f_res.take(0).map_err(|e| OxideError::Database(e.to_string()))?;

        let mut s_res = self.db.query("SELECT * FROM symbol;").await.map_err(|e| OxideError::Database(e.to_string()))?;
        let symbols: Vec<serde_json::Value> = s_res.take(0).map_err(|e| OxideError::Database(e.to_string()))?;

        let mut c_res = self.db.query("SELECT * FROM chunk;").await.map_err(|e| OxideError::Database(e.to_string()))?;
        let chunks: Vec<serde_json::Value> = c_res.take(0).map_err(|e| OxideError::Database(e.to_string()))?;

        let dump = serde_json::json!({
            "files": files,
            "symbols": symbols,
            "chunks": chunks,
        });

        serde_json::to_string_pretty(&dump).map_err(|e| OxideError::Database(e.to_string()))
    }

    async fn import_surql(&self, content: &str) -> Result<()> {
        let val: serde_json::Value = serde_json::from_str(content)
            .map_err(|e| OxideError::Database(e.to_string()))?;

        if let Some(files) = val.get("files").and_then(|v| v.as_array()) {
            for f in files {
                let _ = self.db.query("INSERT INTO file $data;").bind(("data", f.clone())).await;
            }
        }
        if let Some(symbols) = val.get("symbols").and_then(|v| v.as_array()) {
            for s in symbols {
                let _ = self.db.query("INSERT INTO symbol $data;").bind(("data", s.clone())).await;
            }
        }
        if let Some(chunks) = val.get("chunks").and_then(|v| v.as_array()) {
            for c in chunks {
                let _ = self.db.query("INSERT INTO chunk $data;").bind(("data", c.clone())).await;
            }
        }
        Ok(())
    }
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        0.0
    } else {
        dot / (norm_a * norm_b)
    }
}
