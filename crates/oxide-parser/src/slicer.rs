use oxide_core::error::{OxideError, Result};
use oxide_core::id::FileId;
use oxide_core::symbol::{SymbolKind, SymbolRecord};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlicedSymbol {
    pub name: String,
    pub kind: SymbolKind,
    pub signature: Option<String>,
    pub start_line: usize,
    pub end_line: usize,
    pub doc: Option<String>,
    pub code: String,
}

pub struct SymbolSlicer;

impl SymbolSlicer {
    /// Slice specific line ranges directly from a file stream without loading the full file.
    pub fn slice_file_lines<P: AsRef<Path>>(
        file_path: P,
        start_line: usize,
        end_line: usize,
    ) -> Result<String> {
        let file = File::open(file_path.as_ref())?;
        let reader = BufReader::new(file);

        let mut extracted_lines = Vec::new();
        for (idx, line_res) in reader.lines().enumerate() {
            let line_num = idx + 1;
            if line_num >= start_line && line_num <= end_line {
                let line = line_res?;
                extracted_lines.push(line);
            }
            if line_num > end_line {
                break;
            }
        }

        Ok(extracted_lines.join("\n"))
    }

    /// Slice code directly from an in-memory string by line numbers.
    pub fn slice_string_lines(content: &str, start_line: usize, end_line: usize) -> String {
        content
            .lines()
            .enumerate()
            .filter_map(|(idx, line)| {
                let line_num = idx + 1;
                if line_num >= start_line && line_num <= end_line {
                    Some(line)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Extract a target symbol from code using Tree-sitter and slice its source lines.
    pub fn extract_and_slice(
        file_id: &FileId,
        _file_path: &Path,
        content: &str,
        target_symbol: &str,
        language: crate::Language,
    ) -> Result<Option<SlicedSymbol>> {
        let extractor = crate::languages::get_extractor(language)
            .ok_or_else(|| OxideError::Parser(format!("Unsupported language: {:?}", language)))?;
        let symbols = extractor.extract_symbols(file_id, content);

        if let Some(sym) = symbols
            .into_iter()
            .find(|s| s.name == target_symbol || s.qualified_name.as_deref() == Some(target_symbol))
        {
            let code = Self::slice_string_lines(content, sym.start_line, sym.end_line);
            Ok(Some(SlicedSymbol {
                name: sym.name,
                kind: sym.kind,
                signature: sym.signature,
                start_line: sym.start_line,
                end_line: sym.end_line,
                doc: sym.doc,
                code,
            }))
        } else {
            Ok(None)
        }
    }

    /// Slice directly from a SymbolRecord and source file.
    pub fn slice_from_record<P: AsRef<Path>>(
        file_path: P,
        sym: &SymbolRecord,
    ) -> Result<SlicedSymbol> {
        let code = Self::slice_file_lines(&file_path, sym.start_line, sym.end_line)?;
        Ok(SlicedSymbol {
            name: sym.name.clone(),
            kind: sym.kind,
            signature: sym.signature.clone(),
            start_line: sym.start_line,
            end_line: sym.end_line,
            doc: sym.doc.clone(),
            code,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slice_string_lines() {
        let sample = "line1\nline2\nline3\nline4\nline5";
        let sliced = SymbolSlicer::slice_string_lines(sample, 2, 4);
        assert_eq!(sliced, "line2\nline3\nline4");
    }

    #[test]
    fn test_extract_and_slice_rust() {
        let rust_code = r#"
pub struct Motor {
    speed: u32,
}

impl Motor {
    pub fn set_speed(&mut self, val: u32) {
        self.speed = val;
    }
}
"#;
        let fid = FileId::from_relative_path("motor.rs");
        let path = Path::new("motor.rs");
        let res = SymbolSlicer::extract_and_slice(
            &fid,
            path,
            rust_code,
            "set_speed",
            crate::Language::Rust,
        )
        .expect("slice");

        assert!(res.is_some());
        let sym = res.unwrap();
        assert_eq!(sym.name, "set_speed");
        assert!(sym.code.contains("pub fn set_speed"));
        assert!(sym.code.contains("self.speed = val;"));
    }
}
