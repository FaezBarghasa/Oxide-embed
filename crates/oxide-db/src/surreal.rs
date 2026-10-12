use crate::schema::INITIAL_SCHEMA_SURQL;
use crate::store::{ProjectStore, SearchHit, SearchQuery};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use oxide_core::error::{OxideError, Result};
use oxide_core::{
    ChunkRecord, FileRecord, MemoryId, MemoryKind, MemoryRecord, MemoryStatus, ProjectId, SymbolId,
    SymbolRecord,
};
use serde::{Deserialize, Serialize};
use std::path::Path;
use surrealdb::Surreal;
use surrealdb::engine::local::{Db, SurrealKv};

#[derive(Clone)]
pub struct SurrealProjectStore {
    db: Surreal<Db>,
}

impl SurrealProjectStore {
    pub fn db(&self) -> &Surreal<Db> {
        &self.db
    }
}

fn take_vec<T: for<'de> Deserialize<'de>>(
    res: &mut surrealdb::IndexedResults,
    idx: usize,
) -> Result<Vec<T>> {
    let rows: Vec<serde_json::Value> = res
        .take(idx)
        .map_err(|e| OxideError::Database(e.to_string()))?;
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
struct RawSymbolResult {
    #[serde(default)]
    file_id: Option<String>,
    file_path: Option<String>,
    name: String,
    qualified_name: Option<String>,
    kind: String,
    start_line: usize,
    end_line: usize,
    signature: Option<String>,
    doc: Option<String>,
    #[serde(default, deserialize_with = "oxide_core::deserialize_null_as_false")]
    is_macro_node: bool,
    #[serde(default)]
    parent_id: Option<String>,
    #[serde(
        default,
        deserialize_with = "oxide_core::deserialize_null_as_empty_vec"
    )]
    breadcrumbs: Vec<String>,
    #[serde(default)]
    summary: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct MemoryRow {
    id: serde_json::Value,
    project_id: String,
    session_id: Option<String>,
    kind: String,
    title: String,
    content: String,
    tags: Vec<String>,
    symbol_ref: Option<String>,
    status: String,
    superseded_by: Option<String>,
    #[serde(default)]
    author: Option<String>,
    #[serde(default = "default_row_confidence")]
    confidence: f32,
    #[serde(default)]
    source_hash: Option<String>,
    embedding: Option<Vec<f32>>,
    created_at: DateTime<Utc>,
    valid_until: Option<DateTime<Utc>>,
}

fn default_row_confidence() -> f32 {
    1.0
}

impl MemoryRow {
    fn into_memory_record(self) -> Result<MemoryRecord> {
        let raw_id = match &self.id {
            serde_json::Value::String(s) => s.clone(),
            serde_json::Value::Object(map) => map
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            _ => self.id.to_string(),
        };
        let clean_id = raw_id
            .replace("memory_record:", "")
            .replace(['`', '"'], "")
            .trim()
            .to_string();

        let kind: MemoryKind = self.kind.parse()?;
        let status = match self.status.as_str() {
            "active" => MemoryStatus::Active,
            "superseded" => MemoryStatus::Superseded,
            "contradicted" => MemoryStatus::Contradicted,
            _ => MemoryStatus::Active,
        };
        Ok(MemoryRecord {
            id: MemoryId::from_string(clean_id),
            project_id: ProjectId(
                uuid::Uuid::parse_str(&self.project_id).unwrap_or_else(|_| uuid::Uuid::nil()),
            ),
            session_id: self.session_id,
            kind,
            title: self.title,
            content: self.content,
            tags: self.tags,
            symbol_ref: self.symbol_ref,
            status,
            superseded_by: self.superseded_by.map(MemoryId::from_string),
            author: self.author,
            confidence: self.confidence,
            source_hash: self.source_hash,
            created_at: self.created_at,
            valid_until: self.valid_until,
        })
    }
}

#[derive(Debug, Deserialize)]
struct ConflictRow {
    #[serde(flatten)]
    mem: MemoryRow,
    score: Option<f32>,
}

#[derive(Debug, Serialize, Deserialize)]
struct MentalModelRow {
    id: serde_json::Value,
    topic: String,
    question: String,
    standing_answer: String,
    #[serde(default)]
    referenced_symbols: Vec<String>,
    #[serde(default)]
    referenced_files: Vec<String>,
    #[serde(default)]
    source_memory_ids: Vec<String>,
    #[serde(default)]
    file_hashes_snapshot: std::collections::HashMap<String, String>,
    #[serde(default)]
    is_stale: bool,
    last_refreshed_at: DateTime<Utc>,
    #[serde(default)]
    embedding: Option<Vec<f32>>,
}

impl MentalModelRow {
    fn into_mental_model(self) -> Result<oxide_core::MentalModel> {
        let raw_id = match &self.id {
            serde_json::Value::String(s) => s.clone(),
            serde_json::Value::Object(map) => map
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            _ => self.id.to_string(),
        };
        let clean_id = raw_id
            .replace("mental_model:", "")
            .replace(['`', '"'], "")
            .trim()
            .to_string();

        Ok(oxide_core::MentalModel {
            id: oxide_core::ModelId::from_string(clean_id),
            topic: self.topic,
            question: self.question,
            standing_answer: self.standing_answer,
            referenced_symbols: self.referenced_symbols,
            referenced_files: self.referenced_files,
            source_memory_ids: self.source_memory_ids,
            file_hashes_snapshot: self.file_hashes_snapshot,
            is_stale: self.is_stale,
            last_refreshed_at: self.last_refreshed_at,
            embedding: self.embedding,
        })
    }
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
            UPSERT type::record('file', $id) SET
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
            UPSERT type::record('symbol', $id) SET
                file_id = $file_id,
                kind = $kind,
                name = $name,
                qualified_name = $qualified_name,
                start_line = $start_line,
                end_line = $end_line,
                signature = $signature,
                doc = $doc,
                fingerprint = $fingerprint,
                is_macro_node = $is_macro_node,
                parent_id = $parent_id,
                breadcrumbs = $breadcrumbs,
                summary = $summary;
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
            .bind(("is_macro_node", symbol.is_macro_node))
            .bind(("parent_id", symbol.parent_id.as_ref().map(|p| p.0.clone())))
            .bind(("breadcrumbs", symbol.breadcrumbs.clone()))
            .bind(("summary", symbol.summary.clone()))
            .await
            .map_err(|e| OxideError::Database(e.to_string()))?;

        Ok(())
    }

    async fn upsert_chunk(&self, chunk: &ChunkRecord) -> Result<()> {
        let id_str = chunk.id.0.clone();
        let sql = r#"
            UPSERT type::record('chunk', $id) SET
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

    async fn upsert_doc_section(&self, section: &oxide_core::DocSection) -> Result<()> {
        let sql = r#"
            UPSERT type::record('doc_section', $id) SET
                file_path = $file_path,
                heading = $heading,
                content = $content,
                embedding = $embedding;
        "#;
        self.db
            .query(sql)
            .bind(("id", section.id.clone()))
            .bind(("file_path", section.file_path.clone()))
            .bind(("heading", section.heading.clone()))
            .bind(("content", section.content.clone()))
            .bind(("embedding", section.embedding.clone()))
            .await
            .map_err(|e| OxideError::Database(e.to_string()))?;
        Ok(())
    }

    async fn upsert_doc_reference(&self, edge: &oxide_core::DocReferenceEdge) -> Result<()> {
        let sql = r#"
            UPSERT type::record('doc_reference', $id) SET
                in = type::record('doc_section', $doc_id),
                out = type::record('symbol', $sym_id),
                symbol_name = $sym_name,
                context = $context,
                created_at = time::now();
        "#;
        let edge_id = format!(
            "{}_{}",
            edge.doc_section_id.replace(':', "_"),
            edge.symbol_id.0
        );
        let res = self
            .db
            .query(sql)
            .bind(("id", edge_id))
            .bind(("doc_id", edge.doc_section_id.clone()))
            .bind(("sym_id", edge.symbol_id.0.clone()))
            .bind(("sym_name", edge.symbol_id.0.clone()))
            .bind(("context", edge.context.clone()))
            .await
            .map_err(|e| OxideError::Database(e.to_string()))?;
        res.check()
            .map_err(|e| OxideError::Database(e.to_string()))?;
        Ok(())
    }

    async fn upsert_call_edge(&self, edge: &oxide_core::CallEdge) -> Result<()> {
        let sql = r#"
            UPSERT type::record('calls', $id) SET
                in = type::record('symbol', $caller_id),
                out = type::record('symbol', $callee_name),
                caller_name = (SELECT VALUE name FROM type::record('symbol', $caller_id))[0] ?? $caller_id,
                callee_name = $callee_name,
                line = $line,
                weight = 1.0,
                valid_from = time::now();
        "#;
        let edge_id = format!("{}_{}", edge.caller_symbol_id.0, edge.callee_name);
        let res = self
            .db
            .query(sql)
            .bind(("id", edge_id))
            .bind(("caller_id", edge.caller_symbol_id.0.clone()))
            .bind(("callee_name", edge.callee_name.clone()))
            .bind(("line", edge.line as i64))
            .await
            .map_err(|e| OxideError::Database(e.to_string()))?;
        res.check()
            .map_err(|e| OxideError::Database(e.to_string()))?;
        Ok(())
    }

    async fn upsert_import_edge(&self, edge: &oxide_core::ImportEdge) -> Result<()> {
        let sql = r#"
            UPSERT type::record('imports', $id) SET
                in = type::record('file', $file_id),
                imported_path = $imported_path,
                imported_symbols = $imported_symbols,
                weight = 1.0;
        "#;
        let edge_id = format!("{}:{}", edge.file_id.0, edge.imported_path);
        self.db
            .query(sql)
            .bind(("id", edge_id))
            .bind(("file_id", edge.file_id.0.clone()))
            .bind(("imported_path", edge.imported_path.clone()))
            .bind(("imported_symbols", edge.imported_symbols.clone()))
            .await
            .map_err(|e| OxideError::Database(e.to_string()))?;
        Ok(())
    }

    async fn upsert_memory(
        &self,
        memory: &MemoryRecord,
        embedding: Option<Vec<f32>>,
    ) -> Result<()> {
        let id_str = memory.id.0.clone();
        let sql = r#"
            UPSERT type::record('memory_record', $id) SET
                project_id = $project_id,
                session_id = $session_id,
                kind = $kind,
                title = $title,
                content = $content,
                tags = $tags,
                symbol_ref = $symbol_ref,
                status = $status,
                superseded_by = $superseded_by,
                author = $author,
                confidence = $confidence,
                source_hash = $source_hash,
                embedding = $embedding,
                created_at = $created_at,
                valid_until = $valid_until;
        "#;
        let res = self
            .db
            .query(sql)
            .bind(("id", id_str))
            .bind(("project_id", memory.project_id.0.to_string()))
            .bind(("session_id", memory.session_id.clone()))
            .bind(("kind", memory.kind.as_str().to_string()))
            .bind(("title", memory.title.clone()))
            .bind(("content", memory.content.clone()))
            .bind(("tags", memory.tags.clone()))
            .bind(("symbol_ref", memory.symbol_ref.clone()))
            .bind(("status", memory.status.to_string()))
            .bind((
                "superseded_by",
                memory.superseded_by.as_ref().map(|s| s.0.clone()),
            ))
            .bind(("author", memory.author.clone()))
            .bind(("confidence", memory.confidence))
            .bind(("source_hash", memory.source_hash.clone()))
            .bind(("embedding", embedding))
            .bind(("created_at", memory.created_at))
            .bind(("valid_until", memory.valid_until))
            .await
            .map_err(|e| OxideError::Database(e.to_string()))?;
        res.check()
            .map_err(|e| OxideError::Database(e.to_string()))?;
        Ok(())
    }

    async fn get_memory(&self, id: &MemoryId) -> Result<Option<MemoryRecord>> {
        let sql =
            "SELECT * FROM memory_record WHERE id = type::record('memory_record', $id) LIMIT 1;";
        let mut res = self
            .db
            .query(sql)
            .bind(("id", id.0.clone()))
            .await
            .map_err(|e| OxideError::Database(e.to_string()))?;
        let rows: Vec<MemoryRow> = take_vec(&mut res, 0)?;
        if let Some(row) = rows.into_iter().next() {
            Ok(Some(row.into_memory_record()?))
        } else {
            Ok(None)
        }
    }

    async fn recall_memories(
        &self,
        query_emb: Option<&[f32]>,
        kind: Option<MemoryKind>,
        tags: &[String],
        as_of: Option<DateTime<Utc>>,
        limit: usize,
    ) -> Result<Vec<MemoryRecord>> {
        let mut conditions = Vec::new();
        if let Some(k) = kind {
            conditions.push(format!("kind = '{}'", k.as_str()));
        }
        if let Some(cutoff) = as_of {
            conditions.push(format!(
                "created_at <= '{}' AND (valid_until IS NONE OR valid_until > '{}')",
                cutoff.to_rfc3339(),
                cutoff.to_rfc3339()
            ));
        } else {
            conditions.push("status = 'active'".to_string());
        }
        if !tags.is_empty() {
            let tag_conds: Vec<String> = tags
                .iter()
                .map(|t| format!("'{}' IN tags", t.replace('\'', "''")))
                .collect();
            conditions.push(format!("({})", tag_conds.join(" OR ")));
        }
        let where_clause = if conditions.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", conditions.join(" AND "))
        };

        let sql = if query_emb.is_some() {
            format!(
                "SELECT *, vector::similarity::cosine(embedding, $emb) AS score FROM memory_record {} ORDER BY score DESC LIMIT $limit;",
                where_clause
            )
        } else {
            format!(
                "SELECT * FROM memory_record {} ORDER BY created_at DESC LIMIT $limit;",
                where_clause
            )
        };

        let mut query = self.db.query(&sql).bind(("limit", limit as i64));
        if let Some(emb) = query_emb {
            query = query.bind(("emb", emb.to_vec()));
        }
        let mut res = query
            .await
            .map_err(|e| OxideError::Database(e.to_string()))?;
        let rows: Vec<MemoryRow> = take_vec(&mut res, 0)?;
        let mut records = Vec::new();
        for r in rows {
            records.push(r.into_memory_record()?);
        }
        Ok(records)
    }

    async fn find_conflicts(
        &self,
        kind: MemoryKind,
        embedding: &[f32],
        threshold: f32,
    ) -> Result<Vec<MemoryRecord>> {
        let sql = "SELECT *, vector::similarity::cosine(embedding, $emb) AS score FROM memory_record WHERE kind = $kind AND status = 'active' AND embedding IS NOT NONE ORDER BY score DESC LIMIT 5;";
        let mut res = self
            .db
            .query(sql)
            .bind(("kind", kind.as_str().to_string()))
            .bind(("emb", embedding.to_vec()))
            .await
            .map_err(|e| OxideError::Database(e.to_string()))?;

        let rows: Vec<ConflictRow> = take_vec(&mut res, 0)?;
        let mut conflicts = Vec::new();
        for r in rows {
            if r.score.unwrap_or(0.0) >= threshold {
                conflicts.push(r.mem.into_memory_record()?);
            }
        }
        Ok(conflicts)
    }

    async fn list_active_rules(&self) -> Result<Vec<MemoryRecord>> {
        let sql = "SELECT * FROM memory_record WHERE status = 'active' AND kind IN ['instruction', 'decision', 'preference', 'fact'] ORDER BY created_at DESC LIMIT 50;";
        let mut res = self
            .db
            .query(sql)
            .await
            .map_err(|e| OxideError::Database(e.to_string()))?;
        let rows: Vec<MemoryRow> = take_vec(&mut res, 0)?;
        rows.into_iter().map(|r| r.into_memory_record()).collect()
    }

    async fn list_all_memories(&self) -> Result<Vec<MemoryRecord>> {
        let sql = "SELECT * FROM memory_record ORDER BY created_at DESC LIMIT 1000;";
        let mut res = self
            .db
            .query(sql)
            .await
            .map_err(|e| OxideError::Database(e.to_string()))?;
        let rows: Vec<MemoryRow> = take_vec(&mut res, 0)?;
        rows.into_iter().map(|r| r.into_memory_record()).collect()
    }

    async fn sync_all_memories(&self, memories: &[MemoryRecord]) -> Result<()> {
        for mem in memories {
            self.upsert_memory(mem, None).await?;
        }
        Ok(())
    }

    async fn link_memory_to_symbol(
        &self,
        mem_id: &MemoryId,
        symbol_id: &SymbolId,
        relation: &str,
    ) -> Result<()> {
        let sql = r#"
            UPSERT type::record('governs', $id) SET
                in = type::record('memory_record', $mem_id),
                out = type::record('symbol', $sym_id),
                relation = $relation,
                created_at = time::now();
        "#;
        let edge_id = format!("{}_{}", mem_id.0, symbol_id.0);
        let res = self
            .db
            .query(sql)
            .bind(("id", edge_id))
            .bind(("mem_id", mem_id.0.clone()))
            .bind(("sym_id", symbol_id.0.clone()))
            .bind(("relation", relation.to_string()))
            .await
            .map_err(|e| OxideError::Database(e.to_string()))?;
        res.check()
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
        let sql =
            "SELECT outline FROM chunk WHERE file_id = $fid AND kind = 'file_outline' LIMIT 1;";
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
        let (sql, bind_emb) = if let Some(ref q_emb) = query.embedding {
            let s = r#"
                SELECT 
                    (SELECT VALUE relative_path FROM file WHERE id = type::record('file', chunk.file_id))[0] AS file_path,
                    (SELECT VALUE name FROM symbol WHERE id = type::record('symbol', chunk.symbol_id))[0] AS symbol_name,
                    start_line,
                    end_line,
                    outline,
                    text,
                    embedding,
                    vector::similarity::cosine(embedding, $emb) AS vec_score
                FROM chunk
                WHERE embedding IS NOT NONE
                ORDER BY vec_score DESC
                LIMIT $limit;
            "#;
            (s, Some(q_emb.clone()))
        } else {
            let s = r#"
                SELECT 
                    (SELECT VALUE relative_path FROM file WHERE id = type::record('file', chunk.file_id))[0] AS file_path,
                    (SELECT VALUE name FROM symbol WHERE id = type::record('symbol', chunk.symbol_id))[0] AS symbol_name,
                    start_line,
                    end_line,
                    outline,
                    text,
                    embedding,
                    0.0f AS vec_score
                FROM chunk
                LIMIT $limit;
            "#;
            (s, None)
        };

        #[derive(Debug, Deserialize)]
        struct ChunkHitRow {
            file_path: Option<String>,
            symbol_name: Option<String>,
            start_line: usize,
            end_line: usize,
            outline: Option<String>,
            text: String,
            vec_score: Option<f32>,
        }

        let limit = query.limit.max(10);
        let mut q = self.db.query(sql).bind(("limit", limit as i64));
        if let Some(emb) = bind_emb {
            q = q.bind(("emb", emb));
        }

        let mut res = q.await.map_err(|e| OxideError::Database(e.to_string()))?;
        let chunks: Vec<ChunkHitRow> = take_vec(&mut res, 0)?;
        let query_lower = query.text.to_lowercase();

        let mut hits: Vec<SearchHit> = chunks
            .into_iter()
            .filter_map(|c| {
                let file_path = c.file_path.unwrap_or_else(|| "unknown".to_string());
                let text_lower = c.text.to_lowercase();
                let sym_lower = c
                    .symbol_name
                    .as_ref()
                    .map(|s| s.to_lowercase())
                    .unwrap_or_default();
                let outline_lower = c
                    .outline
                    .as_ref()
                    .map(|s| s.to_lowercase())
                    .unwrap_or_default();

                let mut score = c.vec_score.unwrap_or(0.0) * 0.7;

                if sym_lower.contains(&query_lower) && !query_lower.is_empty() {
                    score += 0.5;
                }
                if outline_lower.contains(&query_lower) && !query_lower.is_empty() {
                    score += 0.3;
                }
                if text_lower.contains(&query_lower) && !query_lower.is_empty() {
                    score += 0.2;
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

        hits.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        hits.truncate(query.limit);
        Ok(hits)
    }

    async fn stair_search(&self, query: &str, limit: usize) -> Result<Vec<oxide_core::StairHit>> {
        let query_lower = query.to_lowercase();
        let sym_sql = if query_lower.is_empty() {
            r#"
                SELECT 
                    file_id,
                    name,
                    qualified_name,
                    kind,
                    start_line,
                    end_line,
                    signature,
                    doc,
                    is_macro_node,
                    parent_id,
                    breadcrumbs,
                    summary
                FROM symbol
                LIMIT $limit;
            "#
            .to_string()
        } else {
            r#"
                SELECT 
                    file_id,
                    name,
                    qualified_name,
                    kind,
                    start_line,
                    end_line,
                    signature,
                    doc,
                    is_macro_node,
                    parent_id,
                    breadcrumbs,
                    summary
                FROM symbol
                WHERE string::contains(string::lowercase(name), $q)
                   OR (qualified_name IS NOT NONE AND string::contains(string::lowercase(qualified_name), $q))
                   OR (summary IS NOT NONE AND string::contains(string::lowercase(summary), $q))
                LIMIT $limit;
            "#.to_string()
        };

        let sym_limit = (limit * 4).max(50);
        let mut q_stmt = self.db.query(&sym_sql).bind(("limit", sym_limit as i64));
        if !query_lower.is_empty() {
            q_stmt = q_stmt.bind(("q", query_lower.clone()));
        }

        let mut sym_res = q_stmt
            .await
            .map_err(|e| OxideError::Database(e.to_string()))?;

        let symbols: Vec<RawSymbolResult> = take_vec(&mut sym_res, 0)?;

        let mut file_res = self
            .db
            .query("SELECT id, relative_path FROM file;")
            .await
            .map_err(|e| OxideError::Database(e.to_string()))?;

        #[derive(Debug, Deserialize)]
        struct FileMapRow {
            id: serde_json::Value,
            relative_path: String,
        }

        let file_rows: Vec<FileMapRow> = take_vec(&mut file_res, 0)?;
        let mut file_map = std::collections::HashMap::new();
        for r in file_rows {
            let id_str = match r.id {
                serde_json::Value::String(s) => s,
                serde_json::Value::Object(map) => map
                    .get("id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                _ => r.id.to_string(),
            };
            let clean_id = id_str
                .replace("file:", "")
                .replace(['`', '"'], "")
                .trim()
                .to_string();
            file_map.insert(clean_id, r.relative_path);
        }

        let mut hits = Vec::new();
        for sym in symbols {
            let name_lower = sym.name.to_lowercase();
            let qual_lower = sym
                .qualified_name
                .as_ref()
                .map(|q| q.to_lowercase())
                .unwrap_or_default();
            let sum_lower = sym
                .summary
                .as_ref()
                .map(|s| s.to_lowercase())
                .unwrap_or_default();

            let mut confidence = 0.0f32;
            if name_lower == query_lower {
                confidence = 1.0;
            } else if name_lower.contains(&query_lower) {
                confidence = 0.8;
            } else if qual_lower.contains(&query_lower) {
                confidence = 0.7;
            } else if sum_lower.contains(&query_lower) {
                confidence = 0.5;
            } else if sym
                .breadcrumbs
                .iter()
                .any(|b| b.to_lowercase().contains(&query_lower))
            {
                confidence = 0.4;
            }

            if confidence > 0.0 || query.is_empty() {
                let file_path = sym
                    .file_id
                    .as_deref()
                    .and_then(|fid| {
                        let clean_fid = fid
                            .replace("file:", "")
                            .replace(['`', '"'], "")
                            .trim()
                            .to_string();
                        file_map.get(&clean_fid)
                    })
                    .cloned()
                    .or(sym.file_path)
                    .unwrap_or_else(|| "unknown".to_string());
                let macro_parent = sym.breadcrumbs.first().cloned();

                hits.push(oxide_core::StairHit {
                    breadcrumbs: sym.breadcrumbs,
                    leaf_symbol: sym.name,
                    signature: sym.signature,
                    file_path,
                    start_line: sym.start_line,
                    end_line: sym.end_line,
                    code_body: sym.summary.unwrap_or_default(),
                    confidence,
                    macro_parent,
                    symbol_kind: Some(sym.kind),
                });
            }
        }

        hits.sort_by(|a, b| {
            b.confidence
                .partial_cmp(&a.confidence)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        hits.truncate(limit);
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

    async fn upsert_mental_model(&self, model: &oxide_core::MentalModel) -> Result<()> {
        let sql = r#"
            UPSERT type::record('mental_model', $id) SET
                topic = $topic,
                question = $question,
                standing_answer = $standing_answer,
                referenced_symbols = $referenced_symbols,
                referenced_files = $referenced_files,
                source_memory_ids = $source_memory_ids,
                file_hashes_snapshot = $file_hashes_snapshot,
                is_stale = $is_stale,
                last_refreshed_at = $last_refreshed_at,
                embedding = $embedding;
        "#;
        self.db
            .query(sql)
            .bind(("id", model.id.0.clone()))
            .bind(("topic", model.topic.clone()))
            .bind(("question", model.question.clone()))
            .bind(("standing_answer", model.standing_answer.clone()))
            .bind(("referenced_symbols", model.referenced_symbols.clone()))
            .bind(("referenced_files", model.referenced_files.clone()))
            .bind(("source_memory_ids", model.source_memory_ids.clone()))
            .bind((
                "file_hashes_snapshot",
                serde_json::to_value(&model.file_hashes_snapshot).unwrap_or_default(),
            ))
            .bind(("is_stale", model.is_stale))
            .bind(("last_refreshed_at", model.last_refreshed_at.to_rfc3339()))
            .bind(("embedding", model.embedding.clone()))
            .await
            .map_err(|e| OxideError::Database(e.to_string()))?;

        for sym_name in &model.referenced_symbols {
            let edge_sql = r#"
                RELATE (SELECT id FROM type::record('mental_model', $id))->model_references->(SELECT id FROM symbol WHERE name = $sym_name OR qualified_name = $sym_name LIMIT 1);
            "#;
            let _ = self
                .db
                .query(edge_sql)
                .bind(("id", model.id.0.clone()))
                .bind(("sym_name", sym_name.clone()))
                .await;
        }

        Ok(())
    }

    async fn get_mental_model(&self, topic: &str) -> Result<Option<oxide_core::MentalModel>> {
        let sql = "SELECT * FROM mental_model WHERE topic = $topic LIMIT 1;";
        let mut res = self
            .db
            .query(sql)
            .bind(("topic", topic.to_string()))
            .await
            .map_err(|e| OxideError::Database(e.to_string()))?;
        let rows: Vec<MentalModelRow> = take_vec(&mut res, 0)?;
        if let Some(row) = rows.into_iter().next() {
            Ok(Some(row.into_mental_model()?))
        } else {
            Ok(None)
        }
    }

    async fn list_mental_models(&self) -> Result<Vec<oxide_core::MentalModel>> {
        let sql = "SELECT * FROM mental_model ORDER BY last_refreshed_at DESC;";
        let mut res = self
            .db
            .query(sql)
            .await
            .map_err(|e| OxideError::Database(e.to_string()))?;
        let rows: Vec<MentalModelRow> = take_vec(&mut res, 0)?;
        let mut models = Vec::with_capacity(rows.len());
        for row in rows {
            models.push(row.into_mental_model()?);
        }
        Ok(models)
    }

    async fn search_mental_models(
        &self,
        query_emb: &[f32],
        limit: usize,
    ) -> Result<Vec<oxide_core::MentalModel>> {
        let sql = "SELECT *, vector::similarity::cosine(embedding, $emb) AS score FROM mental_model WHERE embedding IS NOT NONE ORDER BY score DESC LIMIT $limit;";
        let mut res = self
            .db
            .query(sql)
            .bind(("emb", query_emb.to_vec()))
            .bind(("limit", limit as i64))
            .await
            .map_err(|e| OxideError::Database(e.to_string()))?;
        let rows: Vec<MentalModelRow> = take_vec(&mut res, 0)?;
        let mut models = Vec::with_capacity(rows.len());
        for row in rows {
            models.push(row.into_mental_model()?);
        }
        Ok(models)
    }

    async fn mark_mental_models_stale_for_files(&self, file_paths: &[String]) -> Result<usize> {
        let mut count = 0;
        for path in file_paths {
            let sql = "UPDATE mental_model SET is_stale = true WHERE $path IN referenced_files AND is_stale = false;";
            if let Ok(mut res) = self.db.query(sql).bind(("path", path.clone())).await {
                let updated: Vec<serde_json::Value> = res.take(0).unwrap_or_default();
                count += updated.len();
            }
        }
        Ok(count)
    }

    async fn export_surql(&self) -> Result<String> {
        let dump_table = |tbl: &'static str| {
            let db = self.db.clone();
            async move {
                let mut res = db
                    .query(format!("SELECT * FROM {};", tbl))
                    .await
                    .map_err(|e| OxideError::Database(e.to_string()))?;
                let rows: Vec<serde_json::Value> = res
                    .take(0)
                    .map_err(|e| OxideError::Database(e.to_string()))?;
                Ok::<_, OxideError>(rows)
            }
        };

        let files = dump_table("file").await?;
        let symbols = dump_table("symbol").await?;
        let chunks = dump_table("chunk").await?;
        let memories = dump_table("memory_record").await?;
        let calls = dump_table("calls").await?;
        let imports = dump_table("imports").await?;
        let doc_sections = dump_table("doc_section").await?;
        let doc_references = dump_table("doc_reference").await?;
        let cerebrum_rules = dump_table("cerebrum_rule").await?;
        let procedural_skills = dump_table("procedural_skill").await?;
        let mental_models = dump_table("mental_model").await?;
        let model_references = dump_table("model_references").await?;

        let dump = serde_json::json!({
            "files": files,
            "symbols": symbols,
            "chunks": chunks,
            "memories": memories,
            "calls": calls,
            "imports": imports,
            "doc_sections": doc_sections,
            "doc_references": doc_references,
            "cerebrum_rules": cerebrum_rules,
            "procedural_skills": procedural_skills,
            "mental_models": mental_models,
            "model_references": model_references,
        });

        serde_json::to_string_pretty(&dump).map_err(|e| OxideError::Database(e.to_string()))
    }

    async fn import_surql(&self, content: &str) -> Result<()> {
        let val: serde_json::Value =
            serde_json::from_str(content).map_err(|e| OxideError::Database(e.to_string()))?;

        let tables = [
            ("files", "file"),
            ("symbols", "symbol"),
            ("chunks", "chunk"),
            ("memories", "memory_record"),
            ("calls", "calls"),
            ("imports", "imports"),
            ("doc_sections", "doc_section"),
            ("doc_references", "doc_reference"),
            ("cerebrum_rules", "cerebrum_rule"),
            ("procedural_skills", "procedural_skill"),
            ("mental_models", "mental_model"),
            ("model_references", "model_references"),
        ];

        for (json_key, table_name) in tables {
            if let Some(items) = val.get(json_key).and_then(|v| v.as_array()) {
                for item in items {
                    let _ = self
                        .db
                        .query(format!("INSERT INTO {} $data;", table_name))
                        .bind(("data", item.clone()))
                        .await;
                }
            }
        }
        Ok(())
    }
}
