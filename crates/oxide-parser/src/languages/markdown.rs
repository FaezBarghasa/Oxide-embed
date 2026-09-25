use super::LanguageExtractor;
use oxide_core::id::{FileId, SymbolId};
use oxide_core::{SymbolKind, SymbolRecord};
use tree_sitter::{Node, Parser};

pub struct MarkdownExtractor;

impl LanguageExtractor for MarkdownExtractor {
    fn extract_symbols(&self, file_id: &FileId, content: &str) -> Vec<SymbolRecord> {
        let mut parser = Parser::new();
        let language = tree_sitter_md::LANGUAGE.into();
        if parser.set_language(&language).is_err() {
            return fallback_markdown_symbols(file_id, content);
        }

        let tree = match parser.parse(content, None) {
            Some(t) => t,
            None => return fallback_markdown_symbols(file_id, content),
        };

        let root_node = tree.root_node();
        let mut symbols = Vec::new();
        traverse_node(root_node, content, file_id, &mut symbols);

        if symbols.is_empty() {
            fallback_markdown_symbols(file_id, content)
        } else {
            symbols
        }
    }
}

fn traverse_node(node: Node, content: &str, file_id: &FileId, symbols: &mut Vec<SymbolRecord>) {
    let kind = node.kind();
    if kind.contains("heading") || kind == "atx_heading" || kind == "setext_heading" {
        if let Ok(text) = node.utf8_text(content.as_bytes()) {
            let line = text.lines().next().unwrap_or("").trim();
            let heading_text = line.trim_start_matches('#').trim();
            if !heading_text.is_empty() {
                let start_point = node.start_position();
                let end_point = node.end_position();
                let fingerprint = format!("md:{}:{}-{}", heading_text, start_point.row + 1, end_point.row + 1);
                let symbol_id = SymbolId::new(file_id, heading_text);

                symbols.push(SymbolRecord {
                    id: symbol_id,
                    file_id: file_id.clone(),
                    kind: SymbolKind::Module,
                    name: heading_text.to_string(),
                    qualified_name: Some(heading_text.to_string()),
                    start_line: start_point.row + 1,
                    end_line: end_point.row + 1,
                    signature: Some(line.to_string()),
                    doc: None,
                    fingerprint,
                    is_macro_node: false,
                    parent_id: None,
                    breadcrumbs: vec![heading_text.to_string()],
                    summary: Some(format!("Heading: {}", heading_text)),
                });
            }
        }
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        traverse_node(child, content, file_id, symbols);
    }
}

fn fallback_markdown_symbols(file_id: &FileId, content: &str) -> Vec<SymbolRecord> {
    let mut symbols = Vec::new();
    for (idx, line) in content.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with('#') {
            let heading_text = trimmed.trim_start_matches('#').trim();
            if !heading_text.is_empty() {
                let line_no = idx + 1;
                let fingerprint = format!("md:{}:{}", heading_text, line_no);
                let symbol_id = SymbolId::new(file_id, heading_text);

                symbols.push(SymbolRecord {
                    id: symbol_id,
                    file_id: file_id.clone(),
                    kind: SymbolKind::Module,
                    name: heading_text.to_string(),
                    qualified_name: Some(heading_text.to_string()),
                    start_line: line_no,
                    end_line: line_no,
                    signature: Some(trimmed.to_string()),
                    doc: None,
                    fingerprint,
                    is_macro_node: false,
                    parent_id: None,
                    breadcrumbs: vec![heading_text.to_string()],
                    summary: Some(format!("Heading: {}", heading_text)),
                });
            }
        }
    }
    symbols
}
