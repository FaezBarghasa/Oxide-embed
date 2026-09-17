use oxide_core::SymbolRecord;
use oxide_core::cognify::{DocReferenceEdge, DocSection};
use std::collections::HashSet;

pub struct DocLinker;

impl DocLinker {
    pub fn extract_doc_sections(file_path: &str, content: &str) -> Vec<DocSection> {
        let mut sections = Vec::new();
        let mut current_heading = "Overview".to_string();
        let mut current_lines = Vec::new();
        let mut section_index = 0;

        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with('#') {
                if !current_lines.is_empty() {
                    let section_id = format!("{}:{}", file_path, section_index);
                    sections.push(DocSection {
                        id: section_id,
                        file_path: file_path.to_string(),
                        heading: current_heading.clone(),
                        content: current_lines.join("\n"),
                        embedding: None,
                    });
                    section_index += 1;
                    current_lines.clear();
                }
                current_heading = trimmed.trim_start_matches('#').trim().to_string();
            } else {
                current_lines.push(line);
            }
        }

        if !current_lines.is_empty() {
            let section_id = format!("{}:{}", file_path, section_index);
            sections.push(DocSection {
                id: section_id,
                file_path: file_path.to_string(),
                heading: current_heading,
                content: current_lines.join("\n"),
                embedding: None,
            });
        }

        sections
    }

    pub fn link_sections_to_symbols(
        sections: &[DocSection],
        symbols: &[SymbolRecord],
    ) -> Vec<DocReferenceEdge> {
        let mut edges = Vec::new();
        let mut seen = HashSet::new();

        for section in sections {
            let content_lower = section.content.to_lowercase();
            let heading_lower = section.heading.to_lowercase();

            for sym in symbols {
                let name_lower = sym.name.to_lowercase();
                // Avoid matching trivial common 1-2 char words
                if name_lower.len() < 3 {
                    continue;
                }

                if content_lower.contains(&name_lower) || heading_lower.contains(&name_lower) {
                    let key = (section.id.clone(), sym.id.clone());
                    if seen.insert(key) {
                        // Extract sentence context
                        let context = section
                            .content
                            .lines()
                            .find(|l| l.to_lowercase().contains(&name_lower))
                            .unwrap_or(&section.heading)
                            .trim()
                            .to_string();

                        edges.push(DocReferenceEdge {
                            doc_section_id: section.id.clone(),
                            symbol_id: sym.id.clone(),
                            context,
                        });
                    }
                }
            }
        }

        edges
    }
}
