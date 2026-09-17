use super::LanguageExtractor;
use crate::language::Language;
use oxide_core::id::{FileId, SymbolId};
use oxide_core::{CallEdge, ImportEdge, SymbolKind, SymbolRecord};

pub struct GenericConfigExtractor {
    pub language: Language,
}

impl GenericConfigExtractor {
    pub fn new(language: Language) -> Self {
        Self { language }
    }
}

impl LanguageExtractor for GenericConfigExtractor {
    fn extract_symbols(&self, file_id: &FileId, content: &str) -> Vec<SymbolRecord> {
        let mut symbols = Vec::new();
        let lines: Vec<&str> = content.lines().collect();

        for (idx, line) in lines.iter().enumerate() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            // C / C++ Structs, Classes, Functions, and Macros
            if self.language == Language::C || self.language == Language::Cpp {
                let line_no = idx + 1;
                if trimmed.starts_with("struct ") && trimmed.contains('{') {
                    let name = trimmed
                        .trim_start_matches("struct ")
                        .split_whitespace()
                        .next()
                        .unwrap_or("Struct")
                        .trim_end_matches('{')
                        .trim();
                    symbols.push(SymbolRecord {
                        id: SymbolId::new(file_id, name),
                        file_id: file_id.clone(),
                        kind: SymbolKind::Struct,
                        name: name.to_string(),
                        qualified_name: Some(name.to_string()),
                        start_line: line_no,
                        end_line: line_no,
                        signature: Some(trimmed.to_string()),
                        doc: None,
                        fingerprint: format!("c_struct:{}:{}", name, line_no),
                    });
                } else if trimmed.starts_with("class ") && trimmed.contains('{') {
                    let name = trimmed
                        .trim_start_matches("class ")
                        .split_whitespace()
                        .next()
                        .unwrap_or("Class")
                        .trim_end_matches('{')
                        .trim_end_matches(':')
                        .trim();
                    symbols.push(SymbolRecord {
                        id: SymbolId::new(file_id, name),
                        file_id: file_id.clone(),
                        kind: SymbolKind::Class,
                        name: name.to_string(),
                        qualified_name: Some(name.to_string()),
                        start_line: line_no,
                        end_line: line_no,
                        signature: Some(trimmed.to_string()),
                        doc: None,
                        fingerprint: format!("cpp_class:{}:{}", name, line_no),
                    });
                } else if trimmed.starts_with("enum ") && trimmed.contains('{') {
                    let name = trimmed
                        .trim_start_matches("enum ")
                        .split_whitespace()
                        .next()
                        .unwrap_or("Enum")
                        .trim_end_matches('{')
                        .trim();
                    symbols.push(SymbolRecord {
                        id: SymbolId::new(file_id, name),
                        file_id: file_id.clone(),
                        kind: SymbolKind::Enum,
                        name: name.to_string(),
                        qualified_name: Some(name.to_string()),
                        start_line: line_no,
                        end_line: line_no,
                        signature: Some(trimmed.to_string()),
                        doc: None,
                        fingerprint: format!("c_enum:{}:{}", name, line_no),
                    });
                } else if trimmed.starts_with("#define ") {
                    let parts: Vec<&str> = trimmed.split_whitespace().collect();
                    if parts.len() >= 2 {
                        let name = parts[1].split('(').next().unwrap_or(parts[1]);
                        symbols.push(SymbolRecord {
                            id: SymbolId::new(file_id, name),
                            file_id: file_id.clone(),
                            kind: SymbolKind::Macro,
                            name: name.to_string(),
                            qualified_name: Some(name.to_string()),
                            start_line: line_no,
                            end_line: line_no,
                            signature: Some(trimmed.to_string()),
                            doc: None,
                            fingerprint: format!("c_macro:{}:{}", name, line_no),
                        });
                    }
                } else if (trimmed.contains('(') && trimmed.ends_with(')'))
                    || (trimmed.contains('(') && trimmed.ends_with('{'))
                {
                    // Likely function definition: e.g. "void init_hardware() {"
                    let before_paren = trimmed.split('(').next().unwrap_or("").trim();
                    if let Some(func_name) = before_paren.split_whitespace().last() {
                        let clean_name = func_name.trim_start_matches('*');
                        if !clean_name.is_empty()
                            && clean_name != "if"
                            && clean_name != "while"
                            && clean_name != "for"
                            && clean_name != "switch"
                        {
                            symbols.push(SymbolRecord {
                                id: SymbolId::new(file_id, clean_name),
                                file_id: file_id.clone(),
                                kind: SymbolKind::Function,
                                name: clean_name.to_string(),
                                qualified_name: Some(clean_name.to_string()),
                                start_line: line_no,
                                end_line: line_no,
                                signature: Some(trimmed.to_string()),
                                doc: None,
                                fingerprint: format!("c_func:{}:{}", clean_name, line_no),
                            });
                        }
                    }
                }
            }

            // Slint component or struct
            if self.language == Language::Slint {
                if trimmed.starts_with("export component ") || trimmed.starts_with("component ") {
                    let parts: Vec<&str> = trimmed.split_whitespace().collect();
                    let name = parts
                        .iter()
                        .skip_while(|&&p| p != "component")
                        .nth(1)
                        .unwrap_or(&"Component");
                    let name_clean = name.trim_end_matches('{').trim_end_matches(':').trim();
                    let line_no = idx + 1;

                    symbols.push(SymbolRecord {
                        id: SymbolId::new(file_id, name_clean),
                        file_id: file_id.clone(),
                        kind: SymbolKind::Class,
                        name: name_clean.to_string(),
                        qualified_name: Some(name_clean.to_string()),
                        start_line: line_no,
                        end_line: line_no,
                        signature: Some(trimmed.to_string()),
                        doc: None,
                        fingerprint: format!("slint:{}:{}", name_clean, line_no),
                    });
                } else if trimmed.starts_with("export struct ") || trimmed.starts_with("struct ") {
                    let parts: Vec<&str> = trimmed.split_whitespace().collect();
                    let name = parts
                        .iter()
                        .skip_while(|&&p| p != "struct")
                        .nth(1)
                        .unwrap_or(&"Struct");
                    let name_clean = name.trim_end_matches('{').trim_end_matches(':').trim();
                    let line_no = idx + 1;

                    symbols.push(SymbolRecord {
                        id: SymbolId::new(file_id, name_clean),
                        file_id: file_id.clone(),
                        kind: SymbolKind::Struct,
                        name: name_clean.to_string(),
                        qualified_name: Some(name_clean.to_string()),
                        start_line: line_no,
                        end_line: line_no,
                        signature: Some(trimmed.to_string()),
                        doc: None,
                        fingerprint: format!("slint_struct:{}:{}", name_clean, line_no),
                    });
                }
            }

            // Markdown Headings
            if self.language == Language::Markdown && trimmed.starts_with('#') {
                let heading_level = trimmed.chars().take_while(|&c| c == '#').count();
                let heading_text = trimmed.trim_start_matches('#').trim();
                let line_no = idx + 1;

                symbols.push(SymbolRecord {
                    id: SymbolId::new(file_id, heading_text),
                    file_id: file_id.clone(),
                    kind: SymbolKind::Module,
                    name: heading_text.to_string(),
                    qualified_name: Some(format!("H{}: {}", heading_level, heading_text)),
                    start_line: line_no,
                    end_line: line_no,
                    signature: Some(trimmed.to_string()),
                    doc: None,
                    fingerprint: format!("md_h:{}:{}", heading_text, line_no),
                });
            }

            // Dockerfile instructions (FROM, RUN, CMD, EXPOSE, etc.)
            if self.language == Language::Docker {
                let upper = trimmed.to_uppercase();
                if upper.starts_with("FROM ") || upper.starts_with("STAGE ") {
                    let line_no = idx + 1;
                    symbols.push(SymbolRecord {
                        id: SymbolId::new(file_id, &format!("stage_{}", line_no)),
                        file_id: file_id.clone(),
                        kind: SymbolKind::Module,
                        name: trimmed.to_string(),
                        qualified_name: Some(trimmed.to_string()),
                        start_line: line_no,
                        end_line: line_no,
                        signature: Some(trimmed.to_string()),
                        doc: None,
                        fingerprint: format!("docker:{}:{}", trimmed, line_no),
                    });
                }
            }

            // Gradle tasks
            if self.language == Language::Gradle
                && (trimmed.starts_with("task ") || trimmed.starts_with("tasks.register"))
            {
                let line_no = idx + 1;
                symbols.push(SymbolRecord {
                    id: SymbolId::new(file_id, &format!("task_{}", line_no)),
                    file_id: file_id.clone(),
                    kind: SymbolKind::Function,
                    name: trimmed.to_string(),
                    qualified_name: Some(trimmed.to_string()),
                    start_line: line_no,
                    end_line: line_no,
                    signature: Some(trimmed.to_string()),
                    doc: None,
                    fingerprint: format!("gradle:{}:{}", trimmed, line_no),
                });
            }

            // YAML / TOML / CFG / INI Top-Level Sections
            if (self.language == Language::Toml
                || self.language == Language::Cfg
                || self.language == Language::Qemu)
                && trimmed.starts_with('[')
                && trimmed.ends_with(']')
            {
                let section_name = trimmed.trim_matches('[').trim_matches(']').trim();
                let line_no = idx + 1;
                symbols.push(SymbolRecord {
                    id: SymbolId::new(file_id, section_name),
                    file_id: file_id.clone(),
                    kind: SymbolKind::Module,
                    name: section_name.to_string(),
                    qualified_name: Some(format!("[{}]", section_name)),
                    start_line: line_no,
                    end_line: line_no,
                    signature: Some(trimmed.to_string()),
                    doc: None,
                    fingerprint: format!("sec:{}:{}", section_name, line_no),
                });
            }
        }

        symbols
    }

    fn extract_import_edges(&self, file_id: &FileId, content: &str) -> Vec<ImportEdge> {
        let mut imports = Vec::new();
        if self.language == Language::C || self.language == Language::Cpp {
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("#include ") {
                    let path = trimmed
                        .trim_start_matches("#include ")
                        .trim_matches('<')
                        .trim_matches('>')
                        .trim_matches('"')
                        .trim();
                    imports.push(ImportEdge {
                        file_id: file_id.clone(),
                        imported_path: path.to_string(),
                        imported_symbols: Vec::new(),
                    });
                }
            }
        }
        imports
    }

    fn extract_call_edges(
        &self,
        _file_id: &FileId,
        _content: &str,
        _symbols: &[SymbolRecord],
    ) -> Vec<CallEdge> {
        Vec::new()
    }
}
