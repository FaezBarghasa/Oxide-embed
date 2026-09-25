pub mod bash;
pub mod c;
pub mod cpp;
pub mod embedded_meta;
pub mod generic;
pub mod go;
pub mod java;
pub mod python;
pub mod rust;
pub mod typescript;

use crate::language::Language;
use oxide_core::id::FileId;
use oxide_core::{CallEdge, ImportEdge, SymbolRecord};

pub trait LanguageExtractor {
    fn extract_symbols(&self, file_id: &FileId, content: &str) -> Vec<SymbolRecord>;

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

pub fn get_extractor(lang: Language) -> Option<Box<dyn LanguageExtractor + Send + Sync>> {
    match lang {
        Language::Rust => Some(Box::new(rust::RustExtractor)),
        Language::C => Some(Box::new(c::CExtractor)),
        Language::Cpp => Some(Box::new(cpp::CppExtractor)),
        Language::Python | Language::Mojo => Some(Box::new(python::PythonExtractor)),
        Language::JavaScript | Language::TypeScript => {
            Some(Box::new(typescript::TypeScriptExtractor))
        }
        Language::Go => Some(Box::new(go::GoExtractor)),
        Language::Java | Language::Kotlin => Some(Box::new(java::JavaExtractor)),
        Language::Bash => Some(Box::new(bash::BashExtractor)),
        Language::Svd | Language::LinkerScript | Language::Assembly => {
            Some(Box::new(embedded_meta::EmbeddedMetaExtractor::new(lang)))
        }
        Language::Slint
        | Language::Markdown
        | Language::Xml
        | Language::Html
        | Language::Json
        | Language::Yaml
        | Language::Docker
        | Language::Qemu
        | Language::Cfg
        | Language::Svg
        | Language::Csv
        | Language::Gradle
        | Language::Css
        | Language::Toml => Some(Box::new(generic::GenericConfigExtractor::new(lang))),
        _ => None,
    }
}

#[cfg(test)]
mod test_syntax {
    #[test]
    fn test_lang_symbols() {
        let _ = tree_sitter_json::LANGUAGE;
        let _ = tree_sitter_yaml::LANGUAGE;
        let _ = tree_sitter_html::LANGUAGE;
        let _ = tree_sitter_css::LANGUAGE;
        let _ = tree_sitter_md::LANGUAGE;
    }
}

