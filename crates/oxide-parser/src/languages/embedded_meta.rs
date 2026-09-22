use super::LanguageExtractor;
use crate::language::Language;
use oxide_core::id::{FileId, SymbolId};
use oxide_core::{CallEdge, ImportEdge, SymbolKind, SymbolRecord};

pub struct EmbeddedMetaExtractor {
    pub language: Language,
}

impl EmbeddedMetaExtractor {
    pub fn new(language: Language) -> Self {
        Self { language }
    }
}

impl LanguageExtractor for EmbeddedMetaExtractor {
    fn extract_symbols(&self, file_id: &FileId, content: &str) -> Vec<SymbolRecord> {
        let mut symbols = Vec::new();

        match self.language {
            Language::Svd => {
                let mut current_peripheral: Option<(String, usize)> = None;
                for (idx, line) in content.lines().enumerate() {
                    let line_no = idx + 1;
                    let trimmed = line.trim();

                    if trimmed.starts_with("<peripheral>") || trimmed.contains("<peripheral ") {
                        current_peripheral = None;
                    } else if let Some(p) = extract_xml_tag(trimmed, "name") {
                        if current_peripheral.is_none() && (trimmed.contains("<name>") || trimmed.starts_with("<name>")) {
                            current_peripheral = Some((p.clone(), line_no));
                            symbols.push(SymbolRecord {
                                id: SymbolId::new(file_id, &p),
                                file_id: file_id.clone(),
                                kind: SymbolKind::Module,
                                name: p.clone(),
                                qualified_name: Some(format!("peripheral::{}", p)),
                                start_line: line_no,
                                end_line: line_no,
                                signature: Some(format!("Peripheral {}", p)),
                                doc: None,
                                fingerprint: format!("svd_periph:{}:{}", p, line_no),
                                is_macro_node: true,
                                parent_id: None,
                                breadcrumbs: vec![p],
                                summary: Some(format!("CMSIS SVD Peripheral at line {}", line_no)),
                            });
                        } else if let Some((ref periph_name, _)) = current_peripheral {
                            let reg_name = p;
                            let qual = format!("{}::{}", periph_name, reg_name);
                            symbols.push(SymbolRecord {
                                id: SymbolId::new(file_id, &qual),
                                file_id: file_id.clone(),
                                kind: SymbolKind::Field,
                                name: qual.clone(),
                                qualified_name: Some(qual),
                                start_line: line_no,
                                end_line: line_no,
                                signature: Some(format!("Register {} of {}", reg_name, periph_name)),
                                doc: None,
                                fingerprint: format!("svd_reg:{}:{}:{}", periph_name, reg_name, line_no),
                                is_macro_node: false,
                                parent_id: Some(SymbolId::new(file_id, periph_name)),
                                breadcrumbs: vec![periph_name.clone(), reg_name.clone()],
                                summary: Some(format!("Register {} of {}", reg_name, periph_name)),
                            });
                        }
                    }
                }
            }
            Language::LinkerScript => {
                let mut in_memory_block = false;
                for (idx, line) in content.lines().enumerate() {
                    let line_no = idx + 1;
                    let trimmed = line.trim();

                    if trimmed.starts_with("MEMORY") {
                        in_memory_block = true;
                    } else if in_memory_block {
                        if trimmed.starts_with('}') {
                            in_memory_block = false;
                        } else if !trimmed.is_empty() && !trimmed.starts_with('{') && !trimmed.starts_with("/*") && !trimmed.starts_with("//") {
                            let region_name = trimmed
                                .split(['(', ':', ' ', '='])
                                .next()
                                .unwrap_or("")
                                .trim();

                            if !region_name.is_empty() {
                                symbols.push(SymbolRecord {
                                    id: SymbolId::new(file_id, region_name),
                                    file_id: file_id.clone(),
                                    kind: SymbolKind::Constant,
                                    name: region_name.to_string(),
                                    qualified_name: Some(format!("memory_region::{}", region_name)),
                                    start_line: line_no,
                                    end_line: line_no,
                                    signature: Some(trimmed.to_string()),
                                    doc: None,
                                    fingerprint: format!("ld_region:{}:{}", region_name, line_no),
                                    is_macro_node: false,
                                    parent_id: None,
                                    breadcrumbs: vec!["MEMORY".into(), region_name.to_string()],
                                    summary: Some(format!("Memory region {} in linker script", region_name)),
                                });
                            }
                        }
                    } else if trimmed.starts_with('.') && trimmed.contains(':') {
                        let sec_name = trimmed.split(':').next().unwrap_or("").trim();
                        if !sec_name.is_empty() {
                            symbols.push(SymbolRecord {
                                id: SymbolId::new(file_id, sec_name),
                                file_id: file_id.clone(),
                                kind: SymbolKind::Module,
                                name: sec_name.to_string(),
                                qualified_name: Some(format!("section::{}", sec_name)),
                                start_line: line_no,
                                end_line: line_no,
                                signature: Some(trimmed.to_string()),
                                doc: None,
                                fingerprint: format!("ld_section:{}:{}", sec_name, line_no),
                                is_macro_node: true,
                                parent_id: None,
                                breadcrumbs: vec!["SECTIONS".into(), sec_name.to_string()],
                                summary: Some(format!("Section {} in linker script", sec_name)),
                            });
                        }
                    }
                }
            }
            Language::Assembly => {
                for (idx, line) in content.lines().enumerate() {
                    let line_no = idx + 1;
                    let trimmed = line.trim();

                    if (trimmed.ends_with(':') || trimmed.contains(": ")) && !trimmed.starts_with("/*") && !trimmed.starts_with("//") {
                        let label = trimmed.split(':').next().unwrap_or("").trim();
                        if !label.is_empty() && !label.starts_with('.') {
                            symbols.push(SymbolRecord {
                                id: SymbolId::new(file_id, label),
                                file_id: file_id.clone(),
                                kind: SymbolKind::Function,
                                name: label.to_string(),
                                qualified_name: Some(label.to_string()),
                                start_line: line_no,
                                end_line: line_no,
                                signature: Some(format!("{}:", label)),
                                doc: None,
                                fingerprint: format!("asm_label:{}:{}", label, line_no),
                                is_macro_node: false,
                                parent_id: None,
                                breadcrumbs: vec![label.to_string()],
                                summary: Some(format!("Assembly routine {} at line {}", label, line_no)),
                            });
                        }
                    }
                }
            }
            _ => {}
        }

        symbols
    }

    fn extract_call_edges(
        &self,
        _file_id: &FileId,
        _content: &str,
        _symbols: &[SymbolRecord],
    ) -> Vec<CallEdge> {
        Vec::new()
    }

    fn extract_import_edges(&self, _file_id: &FileId, _content: &str) -> Vec<ImportEdge> {
        Vec::new()
    }
}

fn extract_xml_tag(line: &str, tag: &str) -> Option<String> {
    let open_tag = format!("<{}>", tag);
    let close_tag = format!("</{}>", tag);
    if let Some(start) = line.find(&open_tag)
        && let Some(end) = line.find(&close_tag)
    {
        let content = &line[start + open_tag.len()..end];
        return Some(content.trim().to_string());
    }
    None
}
