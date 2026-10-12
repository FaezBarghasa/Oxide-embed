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
        hops: usize,
    ) -> Result<Option<SubgraphContext>> {
        let sql = r#"
            SELECT 
                id,
                name,
                kind,
                signature,
                file_id,
                (SELECT VALUE relative_path FROM file WHERE id = type::record('file', $parent.file_id))[0] AS file_path
            FROM symbol
            WHERE name = $name OR qualified_name = $name OR id = type::record('symbol', $name)
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
            #[serde(default)]
            id: serde_json::Value,
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
            let sym_id_str = match &sym.id {
                serde_json::Value::String(s) => s.strip_prefix("symbol:").unwrap_or(s).to_string(),
                serde_json::Value::Object(obj) => {
                    if let Some(serde_json::Value::String(s)) = obj.get("id") {
                        s.clone()
                    } else if let Some(serde_json::Value::Object(inner)) = obj.get("id") {
                        if let Some(serde_json::Value::String(s)) = inner.get("String") {
                            s.clone()
                        } else {
                            sym.name.clone()
                        }
                    } else {
                        sym.name.clone()
                    }
                }
                _ => sym.name.clone(),
            };

            let max_hops = hops.clamp(1, 5);
            let mut visited_callees = std::collections::HashSet::new();
            let mut visited_callers = std::collections::HashSet::new();
            let mut current_callee_targets = vec![sym.name.clone(), sym_id_str.clone()];
            let mut current_caller_targets = vec![sym.name.clone(), sym_id_str.clone()];

            #[derive(Deserialize)]
            struct CalleeRow {
                callee_name: String,
            }
            #[derive(Deserialize)]
            struct CallerRow {
                caller_name: String,
                #[serde(default)]
                caller_id: Option<String>,
            }

            for _ in 0..max_hops {
                if current_callee_targets.is_empty() {
                    break;
                }
                let mut next_targets = Vec::new();
                for target in current_callee_targets {
                    let call_sql = r#"
                        SELECT callee_name FROM calls WHERE caller_name = $target OR in = type::record('symbol', $target) OR caller_id = $target;
                    "#;
                    if let Ok(mut callee_res) =
                        store.db().query(call_sql).bind(("target", target)).await
                    {
                        let raw_callees: Vec<serde_json::Value> =
                            callee_res.take(0).unwrap_or_default();
                        for v in raw_callees {
                            if let Ok(row) = serde_json::from_value::<CalleeRow>(v)
                                && !row.callee_name.is_empty()
                                && visited_callees.insert(row.callee_name.clone())
                            {
                                next_targets.push(row.callee_name);
                            }
                        }
                    }
                }
                current_callee_targets = next_targets;
            }

            for _ in 0..max_hops {
                if current_caller_targets.is_empty() {
                    break;
                }
                let mut next_targets = Vec::new();
                for target in current_caller_targets {
                    let caller_sql = r#"
                        SELECT caller_name, caller_id FROM calls WHERE callee_name = $target OR out = type::record('symbol', $target);
                    "#;
                    if let Ok(mut caller_res) =
                        store.db().query(caller_sql).bind(("target", target)).await
                    {
                        let raw_callers: Vec<serde_json::Value> =
                            caller_res.take(0).unwrap_or_default();
                        for v in raw_callers {
                            if let Ok(row) = serde_json::from_value::<CallerRow>(v) {
                                if !row.caller_name.is_empty()
                                    && visited_callers.insert(row.caller_name.clone())
                                {
                                    next_targets.push(row.caller_name);
                                }
                                if let Some(cid) = row.caller_id
                                    && !cid.is_empty()
                                    && visited_callers.insert(cid.clone())
                                {
                                    next_targets.push(cid);
                                }
                            }
                        }
                    }
                }
                current_caller_targets = next_targets;
            }

            let mut callees: Vec<String> = visited_callees.into_iter().collect();
            callees.sort();
            let mut callers: Vec<String> = visited_callers.into_iter().collect();
            callers.sort();

            // Fetch doc references
            let doc_sql = r#"
                SELECT context FROM doc_reference WHERE symbol_name = $name OR symbol_name = $sym_id OR out = type::record('symbol', $sym_id);
            "#;
            let mut doc_res = store
                .db()
                .query(doc_sql)
                .bind(("name", symbol_name.to_string()))
                .bind(("sym_id", sym_id_str))
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
