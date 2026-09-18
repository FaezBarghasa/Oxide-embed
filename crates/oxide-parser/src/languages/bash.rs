use super::LanguageExtractor;
use oxide_core::id::{FileId, SymbolId};
use oxide_core::{SymbolKind, SymbolRecord};
use tree_sitter::{Node, Parser};

pub struct BashExtractor;

impl LanguageExtractor for BashExtractor {
    fn extract_symbols(&self, file_id: &FileId, content: &str) -> Vec<SymbolRecord> {
        let mut parser = Parser::new();
        let language = tree_sitter_bash::LANGUAGE.into();
        if parser.set_language(&language).is_err() {
            return Vec::new();
        }

        let tree = match parser.parse(content, None) {
            Some(t) => t,
            None => return Vec::new(),
        };

        let root_node = tree.root_node();
        let mut symbols = Vec::new();
        traverse_node(root_node, content, file_id, &mut symbols);
        symbols
    }
}

fn traverse_node(node: Node, content: &str, file_id: &FileId, symbols: &mut Vec<SymbolRecord>) {
    if node.kind() == "function_definition"
        && let Some(name_n) = node.child_by_field_name("name")
        && let Ok(name) = name_n.utf8_text(content.as_bytes())
    {
        let start_point = node.start_position();
        let end_point = node.end_position();

        let signature = node
            .utf8_text(content.as_bytes())
            .ok()
            .map(|text| text.lines().next().unwrap_or("").trim().to_string());

        let fingerprint = format!("fn:{}:{}-{}", name, start_point.row + 1, end_point.row + 1);
        let symbol_id = SymbolId::new(file_id, name);

        symbols.push(SymbolRecord {
            id: symbol_id,
            file_id: file_id.clone(),
            kind: SymbolKind::Function,
            name: name.to_string(),
            qualified_name: Some(name.to_string()),
            start_line: start_point.row + 1,
            end_line: end_point.row + 1,
            signature,
            doc: None,
            fingerprint,
            is_macro_node: false,
            parent_id: None,
            breadcrumbs: vec![name.to_string()],
            summary: None,
        });
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        traverse_node(child, content, file_id, symbols);
    }
}
