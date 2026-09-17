use crate::surreal::SurrealProjectStore;
use oxide_core::error::{OxideError, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubgraphContext {
    pub target_symbol: String,
    pub symbol_kind: String,
    pub file_path: String,
    pub signature: Option<String>,
    pub callers: Vec<String>,
    pub callees: Vec<String>,
    pub doc_references: Vec<String>,
}

impl SubgraphContext {
    pub fn to_compact_string(&self) -> String {
        let mut lines = Vec::new();
        lines.push(format!(
            "[Target] {} ({}) in {}",
            self.target_symbol, self.symbol_kind, self.file_path
        ));
        if let Some(sig) = &self.signature {
            lines.push(format!("[Signature] {}", sig));
        }
        if !self.callers.is_empty() {
            lines.push(format!("[Callers] {}", self.callers.join(", ")));
        }
        if !self.callees.is_empty() {
            lines.push(format!("[Callees] {}", self.callees.join(", ")));
        }
        if !self.doc_references.is_empty() {
            lines.push(format!(
                "[Documentation] {}",
                self.doc_references.join(" | ")
            ));
        }
        lines.join("\n")
    }
}

pub struct GraphTraversalService;

impl GraphTraversalService {
    pub async fn get_subgraph(
        store: &SurrealProjectStore,
        symbol_name: &str,
        _hops: usize,
    ) -> Result<Option<SubgraphContext>> {
        let sql = r#"
            SELECT 
                name,
                kind,
                signature,
                file_id,
                (SELECT VALUE relative_path FROM file WHERE id = type::record('file', $parent.file_id))[0] AS file_path
            FROM symbol
            WHERE name = $name OR qualified_name = $name
            LIMIT 1;
        "#;

        let mut res = store
            .db()
            .query(sql)
            .bind(("name", symbol_name.to_string()))
            .await
            .map_err(|e| OxideError::Database(e.to_string()))?;

        #[derive(Deserialize)]
        struct SymRow {
            name: String,
            kind: String,
            signature: Option<String>,
            file_id: String,
            file_path: Option<String>,
        }

        let raw: Vec<serde_json::Value> = res
            .take(0)
            .map_err(|e| OxideError::Database(e.to_string()))?;
        let row: Option<SymRow> = raw
            .into_iter()
            .next()
            .and_then(|v| serde_json::from_value(v).ok());

        if let Some(sym) = row {
            // Fetch callers and callees from calls table
            let call_sql = r#"
                SELECT callee_name FROM calls WHERE caller_name = $name;
            "#;
            let mut callee_res = store
                .db()
                .query(call_sql)
                .bind(("name", symbol_name.to_string()))
                .await
                .map_err(|e| OxideError::Database(e.to_string()))?;

            #[derive(Deserialize)]
            struct CalleeRow {
                callee_name: String,
            }
            let raw_callees: Vec<serde_json::Value> = callee_res.take(0).unwrap_or_default();
            let callees: Vec<String> = raw_callees
                .into_iter()
                .filter_map(|v| serde_json::from_value::<CalleeRow>(v).ok())
                .map(|r| r.callee_name)
                .collect();

            let caller_sql = r#"
                SELECT caller_name FROM calls WHERE callee_name = $name;
            "#;
            let mut caller_res = store
                .db()
                .query(caller_sql)
                .bind(("name", symbol_name.to_string()))
                .await
                .map_err(|e| OxideError::Database(e.to_string()))?;

            #[derive(Deserialize)]
            struct CallerRow {
                caller_name: String,
            }
            let raw_callers: Vec<serde_json::Value> = caller_res.take(0).unwrap_or_default();
            let callers: Vec<String> = raw_callers
                .into_iter()
                .filter_map(|v| serde_json::from_value::<CallerRow>(v).ok())
                .map(|r| r.caller_name)
                .collect();

            // Fetch doc references
            let doc_sql = r#"
                SELECT context FROM doc_reference WHERE symbol_name = $name;
            "#;
            let mut doc_res = store
                .db()
                .query(doc_sql)
                .bind(("name", symbol_name.to_string()))
                .await
                .map_err(|e| OxideError::Database(e.to_string()))?;

            #[derive(Deserialize)]
            struct DocRow {
                context: String,
            }
            let raw_docs: Vec<serde_json::Value> = doc_res.take(0).unwrap_or_default();
            let docs: Vec<String> = raw_docs
                .into_iter()
                .filter_map(|v| serde_json::from_value::<DocRow>(v).ok())
                .map(|r| r.context)
                .collect();

            Ok(Some(SubgraphContext {
                target_symbol: sym.name,
                symbol_kind: sym.kind,
                file_path: sym.file_path.unwrap_or(sym.file_id),
                signature: sym.signature,
                callers,
                callees,
                doc_references: docs,
            }))
        } else {
            Ok(None)
        }
    }
}
