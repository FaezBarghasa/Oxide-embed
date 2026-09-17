use super::LanguageExtractor;
use oxide_core::id::{FileId, SymbolId};
use oxide_core::{SymbolKind, SymbolRecord};
use tree_sitter::{Node, Parser};

pub struct RustExtractor;

impl LanguageExtractor for RustExtractor {
    fn extract_symbols(&self, file_id: &FileId, content: &str) -> Vec<SymbolRecord> {
        let mut parser = Parser::new();
        let language = tree_sitter_rust::LANGUAGE.into();
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
    let mut current_scope = parent_scope.map(|s| s.to_string());

    let (symbol_kind, name_node) = match kind_str {
        "function_item" => {
            let kind = if parent_scope.is_some() {
                SymbolKind::Method
            } else {
                SymbolKind::Function
            };
            (Some(kind), node.child_by_field_name("name"))
        }
        "struct_item" => (Some(SymbolKind::Struct), node.child_by_field_name("name")),
        "enum_item" => (Some(SymbolKind::Enum), node.child_by_field_name("name")),
        "trait_item" => (Some(SymbolKind::Trait), node.child_by_field_name("name")),
        "impl_item" => {
            let trait_name = node
                .child_by_field_name("trait")
                .and_then(|n| n.utf8_text(content.as_bytes()).ok());
            let type_name = node
                .child_by_field_name("type")
                .and_then(|n| n.utf8_text(content.as_bytes()).ok());
            let impl_name = match (trait_name, type_name) {
                (Some(tr), Some(ty)) => format!("impl {} for {}", tr, ty),
                (None, Some(ty)) => format!("impl {}", ty),
                _ => "impl".to_string(),
            };
            current_scope = Some(impl_name);
            (None, None)
        }
        "const_item" => (Some(SymbolKind::Constant), node.child_by_field_name("name")),
        "static_item" => (Some(SymbolKind::Static), node.child_by_field_name("name")),
        "mod_item" => (Some(SymbolKind::Module), node.child_by_field_name("name")),
        "type_item" => (
            Some(SymbolKind::TypeAlias),
            node.child_by_field_name("name"),
        ),
        _ => (None, None),
    };

    if let (Some(kind), Some(name_n)) = (symbol_kind, name_node)
        && let Ok(name) = name_n.utf8_text(content.as_bytes())
    {
        let qualified_name = match &current_scope {
            Some(scope) => format!("{}::{}", scope, name),
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
        });
    }

    // Recurse into children
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        traverse_node(child, content, file_id, current_scope.as_deref(), symbols);
    }
}
