use super::LanguageExtractor;
use oxide_core::id::{FileId, SymbolId};
use oxide_core::{CallEdge, ImportEdge, SymbolKind, SymbolRecord};
use tree_sitter::{Node, Parser};

pub struct CExtractor;

impl LanguageExtractor for CExtractor {
    fn extract_symbols(&self, file_id: &FileId, content: &str) -> Vec<SymbolRecord> {
        let mut parser = Parser::new();
        let language = tree_sitter_c::LANGUAGE.into();
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
        let language = tree_sitter_c::LANGUAGE.into();
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
        let language = tree_sitter_c::LANGUAGE.into();
        if parser.set_language(&language).is_err() {
            return Vec::new();
        }

        let tree = match parser.parse(content, None) {
            Some(t) => t,
            None => return Vec::new(),
        };

        let root_node = tree.root_node();
        let mut imports = Vec::new();
        traverse_includes(root_node, content, file_id, &mut imports);
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
        "function_definition" => {
            let decl = node.child_by_field_name("declarator");
            let name_n = find_identifier_in_declarator(decl);
            let kind = if parent_scope.is_some() {
                SymbolKind::Method
            } else {
                SymbolKind::Function
            };
            (Some(kind), name_n)
        }
        "struct_specifier" => {
            let name_n = node.child_by_field_name("name");
            if let Some(n) = name_n
                && let Ok(sname) = n.utf8_text(content.as_bytes())
            {
                current_scope = Some(format!("struct {}", sname));
            }
            (Some(SymbolKind::Struct), name_n)
        }
        "union_specifier" => {
            let name_n = node.child_by_field_name("name");
            if let Some(n) = name_n
                && let Ok(uname) = n.utf8_text(content.as_bytes())
            {
                current_scope = Some(format!("union {}", uname));
            }
            (Some(SymbolKind::Union), name_n)
        }
        "enum_specifier" => {
            let name_n = node.child_by_field_name("name");
            if let Some(n) = name_n
                && let Ok(ename) = n.utf8_text(content.as_bytes())
            {
                current_scope = Some(format!("enum {}", ename));
            }
            (Some(SymbolKind::Enum), name_n)
        }
        "enumerator" => {
            let name_n = node.child_by_field_name("name");
            (Some(SymbolKind::Constant), name_n)
        }
        "type_definition" => {
            let decl = node.child_by_field_name("declarator");
            let name_n = if decl.is_some() {
                find_identifier_in_declarator(decl)
            } else {
                let mut cursor = node.walk();
                let mut found = None;
                for child in node.children(&mut cursor) {
                    if child.kind() == "type_identifier" || child.kind() == "identifier" {
                        found = Some(child);
                    }
                }
                found
            };
            (Some(SymbolKind::TypeAlias), name_n)
        }
        "type_identifier" => {
            if node.parent().map(|p| p.kind()) == Some("type_definition") {
                (Some(SymbolKind::TypeAlias), Some(node))
            } else {
                (None, None)
            }
        }
        _ => (None, None),
    };

    if let (Some(kind), Some(name_n)) = (symbol_kind, name_node)
        && let Ok(name) = name_n.utf8_text(content.as_bytes())
        && !name.is_empty()
    {
        let qualified_name = match &parent_scope {
            Some(scope) => format!("{}::{}", scope, name),
            None => name.to_string(),
        };

        let start_point = node.start_position();
        let end_point = node.end_position();

        let signature = node
            .utf8_text(content.as_bytes())
            .ok()
            .map(|text| text.lines().next().unwrap_or("").trim().to_string());

        let doc = extract_c_doc_comment(content, start_point.row);

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
            SymbolKind::Struct | SymbolKind::Union | SymbolKind::Enum | SymbolKind::Module
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
            doc,
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

fn find_identifier_in_declarator(node: Option<Node>) -> Option<Node> {
    let n = node?;
    match n.kind() {
        "identifier" => Some(n),
        "function_declarator"
        | "pointer_declarator"
        | "array_declarator"
        | "parenthesized_declarator" => {
            let inner = n.child_by_field_name("declarator");
            if inner.is_some() {
                find_identifier_in_declarator(inner)
            } else {
                let mut cursor = n.walk();
                for child in n.children(&mut cursor) {
                    if let Some(id) = find_identifier_in_declarator(Some(child)) {
                        return Some(id);
                    }
                }
                None
            }
        }
        _ => {
            let mut cursor = n.walk();
            for child in n.children(&mut cursor) {
                if let Some(id) = find_identifier_in_declarator(Some(child)) {
                    return Some(id);
                }
            }
            None
        }
    }
}

fn extract_c_doc_comment(content: &str, start_row: usize) -> Option<String> {
    let lines: Vec<&str> = content.lines().collect();
    if start_row == 0 || start_row > lines.len() {
        return None;
    }
    let mut doc_lines = Vec::new();
    let mut cur = start_row;
    while cur > 0 {
        cur -= 1;
        let line = lines[cur].trim();
        if line.starts_with("/**") || line.starts_with("/*") || line.starts_with('*') {
            let cleaned = line
                .trim_start_matches("/**")
                .trim_start_matches("/*")
                .trim_start_matches('*')
                .trim_end_matches("*/")
                .trim();
            if !cleaned.is_empty() {
                doc_lines.push(cleaned.to_string());
            }
        } else if line.starts_with("//") {
            let cleaned = line.trim_start_matches("//").trim();
            if !cleaned.is_empty() {
                doc_lines.push(cleaned.to_string());
            }
        } else if line.is_empty() {
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

fn traverse_calls(node: Node, content: &str, symbols: &[SymbolRecord], calls: &mut Vec<CallEdge>) {
    if node.kind() == "call_expression"
        && let Some(func_node) = node.child_by_field_name("function")
        && let Ok(callee_text) = func_node.utf8_text(content.as_bytes())
    {
        let line = node.start_position().row + 1;
        let callee_name = callee_text
            .rsplit("->")
            .next()
            .and_then(|s| s.rsplit('.').next())
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

fn traverse_includes(node: Node, content: &str, file_id: &FileId, imports: &mut Vec<ImportEdge>) {
    if node.kind() == "preproc_include"
        && let Some(path_node) = node.child_by_field_name("path")
        && let Ok(path_text) = path_node.utf8_text(content.as_bytes())
    {
        let cleaned = path_text
            .trim_matches('<')
            .trim_matches('>')
            .trim_matches('"')
            .trim();

        imports.push(ImportEdge {
            file_id: file_id.clone(),
            imported_path: cleaned.to_string(),
            imported_symbols: Vec::new(),
        });
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        traverse_includes(child, content, file_id, imports);
    }
}
