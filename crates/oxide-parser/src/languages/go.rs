use super::LanguageExtractor;
use oxide_core::id::{FileId, SymbolId};
use oxide_core::{SymbolKind, SymbolRecord};
use tree_sitter::{Node, Parser};

pub struct GoExtractor;

impl LanguageExtractor for GoExtractor {
    fn extract_symbols(&self, file_id: &FileId, content: &str) -> Vec<SymbolRecord> {
        let mut parser = Parser::new();
        let language = tree_sitter_go::LANGUAGE.into();
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
    let kind_str = node.kind();
    let current_scope = parent_scope.map(|s| s.to_string());

    let (symbol_kind, name_node) = match kind_str {
        "function_declaration" => (Some(SymbolKind::Function), node.child_by_field_name("name")),
        "method_declaration" => (Some(SymbolKind::Method), node.child_by_field_name("name")),
        "type_spec" => (Some(SymbolKind::Struct), node.child_by_field_name("name")),
        "const_spec" => (Some(SymbolKind::Constant), node.child_by_field_name("name")),
        "package_clause" => (Some(SymbolKind::Module), node.child_by_field_name("name")),
        _ => (None, None),
    };

    if let (Some(kind), Some(name_n)) = (symbol_kind, name_node)
        && let Ok(name) = name_n.utf8_text(content.as_bytes())
    {
        let qualified_name = match &current_scope {
            Some(scope) => format!("{}.{}", scope, name),
            None => name.to_string(),
        };

        let start_point = node.start_position();
        let end_point = node.end_position();

        let signature = node
            .utf8_text(content.as_bytes())
            .ok()
            .map(|text| text.lines().next().unwrap_or("").trim().to_string());

        let fingerprint = format!(
            "{}:{}:{}-{}",
            kind.as_str(),
            qualified_name,
            start_point.row + 1,
            end_point.row + 1
        );
        let symbol_id = SymbolId::new(file_id, &qualified_name);

        let is_macro = matches!(
            kind,
            SymbolKind::Struct | SymbolKind::Module | SymbolKind::Interface | SymbolKind::Class
        );

        let mut breadcrumbs = Vec::new();
        if let Some(scope) = &parent_scope {
            breadcrumbs.push(scope.to_string());
        }
        breadcrumbs.push(name.to_string());

        let parent_id = parent_scope.map(|scope| SymbolId::new(file_id, scope));
        let summary = if is_macro {
            Some(format!(
                "{} {} defined at lines {}-{}",
                kind.as_str(),
                qualified_name,
                start_point.row + 1,
                end_point.row + 1
            ))
        } else {
            None
        };

        symbols.push(SymbolRecord {
            id: symbol_id,
            file_id: file_id.clone(),
            kind,
            name: name.to_string(),
            qualified_name: Some(qualified_name),
            start_line: start_point.row + 1,
            end_line: end_point.row + 1,
            signature,
            doc: None,
            fingerprint,
            is_macro_node: is_macro,
            parent_id,
            breadcrumbs,
            summary,
        });
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        traverse_node(child, content, file_id, current_scope.as_deref(), symbols);
    }
}
