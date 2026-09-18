use super::LanguageExtractor;
use oxide_core::id::{FileId, SymbolId};
use oxide_core::{CallEdge, ImportEdge, SymbolKind, SymbolRecord};
use tree_sitter::{Node, Parser};

pub struct TypeScriptExtractor;

impl LanguageExtractor for TypeScriptExtractor {
    fn extract_symbols(&self, file_id: &FileId, content: &str) -> Vec<SymbolRecord> {
        let mut parser = Parser::new();
        let language = tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into();
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

    fn extract_call_edges(
        &self,
        _file_id: &FileId,
        content: &str,
        symbols: &[SymbolRecord],
    ) -> Vec<CallEdge> {
        let mut parser = Parser::new();
        let language = tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into();
        if parser.set_language(&language).is_err() {
            return Vec::new();
        }

        let tree = match parser.parse(content, None) {
            Some(t) => t,
            None => return Vec::new(),
        };

        let root_node = tree.root_node();
        let mut calls = Vec::new();
        traverse_calls(root_node, content, symbols, &mut calls);
        calls
    }

    fn extract_import_edges(&self, file_id: &FileId, content: &str) -> Vec<ImportEdge> {
        let mut parser = Parser::new();
        let language = tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into();
        if parser.set_language(&language).is_err() {
            return Vec::new();
        }

        let tree = match parser.parse(content, None) {
            Some(t) => t,
            None => return Vec::new(),
        };

        let root_node = tree.root_node();
        let mut imports = Vec::new();
        traverse_imports(root_node, content, file_id, &mut imports);
        imports
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
        "function_declaration" => (Some(SymbolKind::Function), node.child_by_field_name("name")),
        "method_definition" => (Some(SymbolKind::Method), node.child_by_field_name("name")),
        "class_declaration" => {
            let name_n = node.child_by_field_name("name");
            if let Some(n) = name_n
                && let Ok(class_name) = n.utf8_text(content.as_bytes())
            {
                current_scope = Some(class_name.to_string());
            }
            (Some(SymbolKind::Class), name_n)
        }
        "interface_declaration" => (
            Some(SymbolKind::Interface),
            node.child_by_field_name("name"),
        ),
        "type_alias_declaration" => (
            Some(SymbolKind::TypeAlias),
            node.child_by_field_name("name"),
        ),
        _ => (None, None),
    };

    if let (Some(kind), Some(name_n)) = (symbol_kind, name_node)
        && let Ok(name) = name_n.utf8_text(content.as_bytes())
    {
        let qualified_name = match &parent_scope {
            Some(scope) if kind == SymbolKind::Method => format!("{}.{}", scope, name),
            _ => name.to_string(),
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
            SymbolKind::Struct
                | SymbolKind::Enum
                | SymbolKind::Interface
                | SymbolKind::Class
                | SymbolKind::Module
        );

        let mut breadcrumbs = Vec::new();
        if let Some(scope) = &parent_scope {
            breadcrumbs.push(scope.to_string());
        }
        breadcrumbs.push(name.to_string());

        let parent_id = parent_scope.as_ref().map(|scope| SymbolId::new(file_id, scope));
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

fn traverse_calls(node: Node, content: &str, symbols: &[SymbolRecord], calls: &mut Vec<CallEdge>) {
    if node.kind() == "call_expression"
        && let Some(func_node) = node.child_by_field_name("function")
        && let Ok(callee_text) = func_node.utf8_text(content.as_bytes())
    {
        let line = node.start_position().row + 1;
        let callee_name = callee_text
            .split('.')
            .next_back()
            .unwrap_or(callee_text)
            .trim()
            .to_string();

        if let Some(caller) = symbols.iter().find(|s| {
            s.start_line <= line
                && line <= s.end_line
                && matches!(s.kind, SymbolKind::Function | SymbolKind::Method)
        }) {
            calls.push(CallEdge {
                caller_symbol_id: caller.id.clone(),
                callee_name,
                line,
            });
        }
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        traverse_calls(child, content, symbols, calls);
    }
}

fn traverse_imports(node: Node, content: &str, file_id: &FileId, imports: &mut Vec<ImportEdge>) {
    if node.kind() == "import_statement"
        && let Ok(import_text) = node.utf8_text(content.as_bytes())
    {
        let path = if let Some(source_node) = node.child_by_field_name("source") {
            source_node
                .utf8_text(content.as_bytes())
                .unwrap_or("")
                .trim_matches('\'')
                .trim_matches('"')
                .to_string()
        } else {
            import_text.to_string()
        };

        imports.push(ImportEdge {
            file_id: file_id.clone(),
            imported_path: path,
            imported_symbols: Vec::new(),
        });
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        traverse_imports(child, content, file_id, imports);
    }
}
