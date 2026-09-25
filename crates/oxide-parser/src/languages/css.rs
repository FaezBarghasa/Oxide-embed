use super::LanguageExtractor;
use oxide_core::id::{FileId, SymbolId};
use oxide_core::{SymbolKind, SymbolRecord};
use tree_sitter::{Node, Parser};

pub struct CssExtractor;

impl LanguageExtractor for CssExtractor {
    fn extract_symbols(&self, file_id: &FileId, content: &str) -> Vec<SymbolRecord> {
        let mut parser = Parser::new();
        let language = tree_sitter_css::LANGUAGE.into();
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
    let kind_str = node.kind();
    match kind_str {
        "rule_set" => {
            let selectors_n = node.child_by_field_name("selectors").or_else(|| {
                let mut cursor = node.walk();
                node.children(&mut cursor).find(|c| c.kind() == "selectors")
            });

            if let Some(sel_node) = selectors_n
                && let Ok(selector_text) = sel_node.utf8_text(content.as_bytes())
            {
                let clean_selector = selector_text.lines().next().unwrap_or("").trim().trim_end_matches('{').trim();
                if !clean_selector.is_empty() {
                    let start_point = node.start_position();
                    let end_point = node.end_position();
                    let fingerprint = format!("css:{}:{}-{}", clean_selector, start_point.row + 1, end_point.row + 1);
                    let symbol_id = SymbolId::new(file_id, clean_selector);

                    symbols.push(SymbolRecord {
                        id: symbol_id,
                        file_id: file_id.clone(),
                        kind: SymbolKind::Class,
                        name: clean_selector.to_string(),
                        qualified_name: Some(clean_selector.to_string()),
                        start_line: start_point.row + 1,
                        end_line: end_point.row + 1,
                        signature: Some(format!("{} {{...}}", clean_selector)),
                        doc: None,
                        fingerprint,
                        is_macro_node: false,
                        parent_id: None,
                        breadcrumbs: vec![clean_selector.to_string()],
                        summary: None,
                    });
                }
            }
        }
        "keyframes_statement" => {
            let name_n = node.child_by_field_name("name").or_else(|| {
                let mut cursor = node.walk();
                node.children(&mut cursor).find(|c| c.kind() == "keyframes_name")
            });

            if let Some(n) = name_n
                && let Ok(name) = n.utf8_text(content.as_bytes())
            {
                let clean_name = name.trim();
                let start_point = node.start_position();
                let end_point = node.end_position();
                let fingerprint = format!("keyframes:{}:{}-{}", clean_name, start_point.row + 1, end_point.row + 1);
                let symbol_id = SymbolId::new(file_id, clean_name);

                symbols.push(SymbolRecord {
                    id: symbol_id,
                    file_id: file_id.clone(),
                    kind: SymbolKind::Macro,
                    name: format!("@keyframes {}", clean_name),
                    qualified_name: Some(format!("@keyframes {}", clean_name)),
                    start_line: start_point.row + 1,
                    end_line: end_point.row + 1,
                    signature: Some(format!("@keyframes {} {{...}}", clean_name)),
                    doc: None,
                    fingerprint,
                    is_macro_node: false,
                    parent_id: None,
                    breadcrumbs: vec![format!("@keyframes {}", clean_name)],
                    summary: None,
                });
            }
        }
        _ => {}
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        traverse_node(child, content, file_id, symbols);
    }
}
