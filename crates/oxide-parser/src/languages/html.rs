use super::LanguageExtractor;
use oxide_core::id::{FileId, SymbolId};
use oxide_core::{SymbolKind, SymbolRecord};
use tree_sitter::{Node, Parser};

pub struct HtmlExtractor;

impl LanguageExtractor for HtmlExtractor {
    fn extract_symbols(&self, file_id: &FileId, content: &str) -> Vec<SymbolRecord> {
        let mut parser = Parser::new();
        let language = tree_sitter_html::LANGUAGE.into();
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
    if node.kind() == "element" || node.kind() == "script_element" || node.kind() == "style_element" {
        let start_tag = node.child_by_field_name("start_tag").or_else(|| {
            let mut cursor = node.walk();
            node.children(&mut cursor).find(|c| c.kind() == "start_tag")
        });

        let tag_name = start_tag
            .and_then(|st| st.child_by_field_name("tag_name").or_else(|| {
                let mut cursor = st.walk();
                st.children(&mut cursor).find(|c| c.kind() == "tag_name")
            }))
            .and_then(|tn| tn.utf8_text(content.as_bytes()).ok());

        let id_val = start_tag.and_then(|st| {
            let mut cursor = st.walk();
            for attr in st.children(&mut cursor) {
                if attr.kind() == "attribute" {
                    let mut attr_cursor = attr.walk();
                    let mut is_id = false;
                    for part in attr.children(&mut attr_cursor) {
                        if part.kind() == "attribute_name" && part.utf8_text(content.as_bytes()).ok() == Some("id") {
                            is_id = true;
                        } else if is_id && (part.kind() == "quoted_attribute_value" || part.kind() == "attribute_value") {
                            return part.utf8_text(content.as_bytes()).ok().map(|s| s.trim_matches('"').trim_matches('\''));
                        }
                    }
                }
            }
            None
        });

        if let Some(tag) = tag_name {
            let is_notable = id_val.is_some()
                || tag.contains('-')
                || matches!(tag, "main" | "nav" | "header" | "footer" | "form" | "section" | "template" | "dialog");

            if is_notable {
                let display_name = match id_val {
                    Some(id) => format!("{}#{}", tag, id),
                    None => tag.to_string(),
                };

                let start_point = node.start_position();
                let end_point = node.end_position();
                let fingerprint = format!("html:{}:{}-{}", display_name, start_point.row + 1, end_point.row + 1);
                let symbol_id = SymbolId::new(file_id, &display_name);

                let signature = start_tag
                    .and_then(|st| st.utf8_text(content.as_bytes()).ok())
                    .map(|s| s.lines().next().unwrap_or("").trim().to_string());

                symbols.push(SymbolRecord {
                    id: symbol_id,
                    file_id: file_id.clone(),
                    kind: SymbolKind::Class,
                    name: display_name.clone(),
                    qualified_name: Some(display_name.clone()),
                    start_line: start_point.row + 1,
                    end_line: end_point.row + 1,
                    signature,
                    doc: None,
                    fingerprint,
                    is_macro_node: false,
                    parent_id: None,
                    breadcrumbs: vec![display_name],
                    summary: None,
                });
            }
        }
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        traverse_node(child, content, file_id, symbols);
    }
}
