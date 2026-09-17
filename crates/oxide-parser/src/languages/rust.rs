use super::LanguageExtractor;
use oxide_core::id::{FileId, SymbolId};
use oxide_core::{CallEdge, ImportEdge, SymbolKind, SymbolRecord};
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

    fn extract_call_edges(
        &self,
        _file_id: &FileId,
        content: &str,
        symbols: &[SymbolRecord],
    ) -> Vec<CallEdge> {
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
        let mut calls = Vec::new();
        traverse_calls(root_node, content, symbols, &mut calls);
        calls
    }

    fn extract_import_edges(&self, file_id: &FileId, content: &str) -> Vec<ImportEdge> {
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

        let doc = extract_doc_comment(content, start_point.row);

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
            doc,
            fingerprint,
        });
    }

    // Recurse into children
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        traverse_node(child, content, file_id, current_scope.as_deref(), symbols);
    }
}

fn extract_doc_comment(content: &str, start_row: usize) -> Option<String> {
    let lines: Vec<&str> = content.lines().collect();
    if start_row == 0 || start_row > lines.len() {
        return None;
    }
    let mut doc_lines = Vec::new();
    let mut cur = start_row;
    while cur > 0 {
        cur -= 1;
        let line = lines[cur].trim();
        if line.starts_with("///") {
            doc_lines.push(line.trim_start_matches("///").trim().to_string());
        } else if line.starts_with("#[") || line.is_empty() {
            continue;
        } else {
            break;
        }
    }
    if doc_lines.is_empty() {
        None
    } else {
        doc_lines.reverse();
        Some(doc_lines.join("\n"))
    }
}

fn traverse_calls(
    node: Node,
    content: &str,
    symbols: &[SymbolRecord],
    calls: &mut Vec<CallEdge>,
) {
    let kind = node.kind();
    if kind == "call_expression" {
        if let Some(func_node) = node.child_by_field_name("function")
            && let Ok(callee_text) = func_node.utf8_text(content.as_bytes())
        {
            let line = node.start_position().row + 1;
            let callee_name = callee_text.split("::").last().unwrap_or(callee_text).trim().to_string();

            // Find enclosing function/method symbol
            if let Some(caller) = symbols
                .iter()
                .find(|s| s.start_line <= line && line <= s.end_line && matches!(s.kind, SymbolKind::Function | SymbolKind::Method))
            {
                calls.push(CallEdge {
                    caller_symbol_id: caller.id.clone(),
                    callee_name,
                    line,
                });
            }
        }
    } else if kind == "field_expression" || kind == "method_call_expression" {
        if let Some(method_node) = node.child_by_field_name("name")
            && let Ok(callee_text) = method_node.utf8_text(content.as_bytes())
        {
            let line = node.start_position().row + 1;
            let callee_name = callee_text.trim().to_string();

            if let Some(caller) = symbols
                .iter()
                .find(|s| s.start_line <= line && line <= s.end_line && matches!(s.kind, SymbolKind::Function | SymbolKind::Method))
            {
                calls.push(CallEdge {
                    caller_symbol_id: caller.id.clone(),
                    callee_name,
                    line,
                });
            }
        }
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        traverse_calls(child, content, symbols, calls);
    }
}

fn traverse_imports(
    node: Node,
    content: &str,
    file_id: &FileId,
    imports: &mut Vec<ImportEdge>,
) {
    if node.kind() == "use_declaration" {
        if let Ok(use_text) = node.utf8_text(content.as_bytes()) {
            let cleaned = use_text
                .trim_start_matches("use ")
                .trim_end_matches(';')
                .trim();

            let mut imported_symbols = Vec::new();
            if let Some(last_part) = cleaned.split("::").last() {
                if last_part.starts_with('{') && last_part.ends_with('}') {
                    let inner = &last_part[1..last_part.len() - 1];
                    for s in inner.split(',') {
                        let sym = s.trim();
                        if !sym.is_empty() {
                            imported_symbols.push(sym.to_string());
                        }
                    }
                } else {
                    imported_symbols.push(last_part.to_string());
                }
            }

            imports.push(ImportEdge {
                file_id: file_id.clone(),
                imported_path: cleaned.to_string(),
                imported_symbols,
            });
        }
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        traverse_imports(child, content, file_id, imports);
    }
}
