pub mod bash;
pub mod generic;
pub mod go;
pub mod java;
pub mod python;
pub mod rust;
pub mod typescript;

use oxide_core::id::FileId;
use oxide_core::SymbolRecord;
use crate::language::Language;

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
        Language::Python | Language::Mojo => Some(Box::new(python::PythonExtractor)),
        Language::JavaScript | Language::TypeScript => Some(Box::new(typescript::TypeScriptExtractor)),
        Language::Go => Some(Box::new(go::GoExtractor)),
        Language::Java | Language::Kotlin => Some(Box::new(java::JavaExtractor)),
        Language::Bash => Some(Box::new(bash::BashExtractor)),
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
