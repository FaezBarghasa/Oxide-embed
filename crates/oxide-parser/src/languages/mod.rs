pub mod rust;
pub mod python;
pub mod typescript;

use oxide_core::SymbolRecord;
use crate::language::Language;
use oxide_core::id::FileId;

pub trait LanguageExtractor {
    fn extract_symbols(
        &self,
        file_id: &FileId,
        content: &str,
    ) -> Vec<SymbolRecord>;
}

pub fn get_extractor(lang: Language) -> Option<Box<dyn LanguageExtractor + Send + Sync>> {
    match lang {
        Language::Rust => Some(Box::new(rust::RustExtractor)),
        Language::Python => Some(Box::new(python::PythonExtractor)),
        Language::JavaScript | Language::TypeScript => Some(Box::new(typescript::TypeScriptExtractor)),
        _ => None,
    }
}
