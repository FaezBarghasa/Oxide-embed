use super::LanguageExtractor;
use oxide_core::id::{FileId, SymbolId};
use oxide_core::{SymbolKind, SymbolRecord};
use tree_sitter::{Node, Parser};

pub struct JsonExtractor;

impl LanguageExtractor for JsonExtractor {
    fn extract_symbols(&self, file_id: &FileId, content: &str) -> Vec<SymbolRecord> {
        let mut parser = Parser::new();
        let language = tree_sitter_json::LANGUAGE.into();
        if parser.set_language(&language).is_err() {
            return Vec::new();
        }

        let tree = match parser.parse(content, None) {
            Some(t) => t,
            None => return Vec::new(),
        };

        let root_node = tree.root_node();
        let mut symbols = Vec::new();
        traverse_node(root_node, content, file_id, None, &mut symbols);
        symbols
    }
}

fn traverse_node(
    node: Node,
    content: &str,
    file_id: &FileId,
    parent_scope: Option<&str>,
    symbols: &mut Vec<SymbolRecord>,
) {
    if node.kind() == "pair"
        && let Some(key_n) = node.child_by_field_name("key")
        && let Ok(raw_key) = key_n.utf8_text(content.as_bytes())
    {
        let key = raw_key.trim_matches('"');
        let qualified_name = match parent_scope {
            Some(scope) => format!("{}.{}", scope, key),
            None => key.to_string(),
        };

        let start_point = node.start_position();
        let end_point = node.end_position();

        let val_n = node.child_by_field_name("value");
        let is_container = val_n.is_some_and(|v| v.kind() == "object" || v.kind() == "array");
        let kind = if is_container {
            SymbolKind::Struct
        } else {
            SymbolKind::Field
        };

        let signature = if is_container {
            Some(format!("{}: {{...}}", key))
        } else {
            val_n
                .and_then(|v| v.utf8_text(content.as_bytes()).ok())
                .map(|val| format!("{}: {}", key, val))
        };

        let fingerprint = format!("json:{}:{}-{}", qualified_name, start_point.row + 1, end_point.row + 1);
        let symbol_id = SymbolId::new(file_id, &qualified_name);

        symbols.push(SymbolRecord {
            id: symbol_id,
            file_id: file_id.clone(),
            kind,
            name: key.to_string(),
            qualified_name: Some(qualified_name.clone()),
            start_line: start_point.row + 1,
            end_line: end_point.row + 1,
            signature,
            doc: None,
            fingerprint,
            is_macro_node: false,
            parent_id: None,
            breadcrumbs: vec![qualified_name.clone()],
            summary: Some(format!("JSON property {}", qualified_name)),
        });

        if let Some(val_node) = val_n {
            let mut cursor = val_node.walk();
            for child in val_node.children(&mut cursor) {
                traverse_node(child, content, file_id, Some(&qualified_name), symbols);
            }
        }
        return;
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        traverse_node(child, content, file_id, parent_scope, symbols);
    }
}
