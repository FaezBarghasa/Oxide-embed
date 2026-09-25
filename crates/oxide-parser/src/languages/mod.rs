pub mod bash;
pub mod c;
pub mod cpp;
pub mod css;
pub mod embedded_meta;
pub mod generic;
pub mod go;
pub mod html;
pub mod java;
pub mod json;
pub mod markdown;
pub mod python;
pub mod rust;
pub mod typescript;
pub mod yaml;

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
        Language::Html => Some(Box::new(html::HtmlExtractor)),
        Language::Css => Some(Box::new(css::CssExtractor)),
        Language::Json => Some(Box::new(json::JsonExtractor)),
        Language::Yaml => Some(Box::new(yaml::YamlExtractor)),
        Language::Markdown => Some(Box::new(markdown::MarkdownExtractor)),
        Language::Svd | Language::LinkerScript | Language::Assembly => {
            Some(Box::new(embedded_meta::EmbeddedMetaExtractor::new(lang)))
        }
        Language::Slint
        | Language::Xml
        | Language::Docker
        | Language::Qemu
        | Language::Cfg
        | Language::Svg
        | Language::Csv
        | Language::Gradle
        | Language::Toml
        | Language::SystemVerilog
        | Language::OpenScad => Some(Box::new(generic::GenericConfigExtractor::new(lang))),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_extractor() {
        let file_id = FileId::from_relative_path("config.json");
        let content = r#"{
            "server": {
                "port": 8080,
                "host": "127.0.0.1"
            },
            "enabled": true
        }"#;
        let extractor = get_extractor(Language::Json).expect("JSON extractor");
        let symbols = extractor.extract_symbols(&file_id, content);
        assert!(symbols.iter().any(|s| s.name == "server"));
        assert!(symbols.iter().any(|s| s.name == "port"));
        assert!(symbols.iter().any(|s| s.name == "enabled"));
    }

    #[test]
    fn test_yaml_extractor() {
        let file_id = FileId::from_relative_path("pipeline.yml");
        let content = r#"
version: "3"
services:
  web:
    image: nginx
"#;
        let extractor = get_extractor(Language::Yaml).expect("YAML extractor");
        let symbols = extractor.extract_symbols(&file_id, content);
        assert!(symbols.iter().any(|s| s.name == "version"));
        assert!(symbols.iter().any(|s| s.name == "services"));
        assert!(symbols.iter().any(|s| s.name == "web"));
    }

    #[test]
    fn test_html_extractor() {
        let file_id = FileId::from_relative_path("index.html");
        let content = r#"
<!DOCTYPE html>
<html>
  <body>
    <main id="app-root">
      <form id="login-form">
      </form>
    </main>
  </body>
</html>
"#;
        let extractor = get_extractor(Language::Html).expect("HTML extractor");
        let symbols = extractor.extract_symbols(&file_id, content);
        assert!(symbols.iter().any(|s| s.name.contains("main#app-root")));
        assert!(symbols.iter().any(|s| s.name.contains("form#login-form")));
    }

    #[test]
    fn test_css_extractor() {
        let file_id = FileId::from_relative_path("styles.css");
        let content = r#"
.dashboard-card {
    background-color: #1a1a1a;
}

@keyframes pulse {
    0% { opacity: 0.5; }
    100% { opacity: 1.0; }
}
"#;
        let extractor = get_extractor(Language::Css).expect("CSS extractor");
        let symbols = extractor.extract_symbols(&file_id, content);
        assert!(symbols.iter().any(|s| s.name == ".dashboard-card"));
        assert!(symbols.iter().any(|s| s.name == "@keyframes pulse"));
    }

    #[test]
    fn test_markdown_extractor() {
        let file_id = FileId::from_relative_path("README.md");
        let content = r#"
# Architecture Overview

Detailed system notes.

## Subsystems

More details.
"#;
        let extractor = get_extractor(Language::Markdown).expect("Markdown extractor");
        let symbols = extractor.extract_symbols(&file_id, content);
        assert!(symbols.iter().any(|s| s.name == "Architecture Overview"));
        assert!(symbols.iter().any(|s| s.name == "Subsystems"));
    }

    #[test]
    fn test_systemverilog_extractor() {
        let file_id = FileId::from_relative_path("core.sv");
        let content = r#"
module alu_core #(parameter WIDTH = 32)(
    input logic clk
);
endmodule
"#;
        let extractor = get_extractor(Language::SystemVerilog).expect("SV extractor");
        let symbols = extractor.extract_symbols(&file_id, content);
        assert!(symbols.iter().any(|s| s.name == "alu_core"));
    }

    #[test]
    fn test_openscad_extractor() {
        let file_id = FileId::from_relative_path("gear.scad");
        let content = r#"
module gear_mount(teeth = 12) {
    cylinder(r = teeth, h = 10);
}
"#;
        let extractor = get_extractor(Language::OpenScad).expect("SCAD extractor");
        let symbols = extractor.extract_symbols(&file_id, content);
        assert!(symbols.iter().any(|s| s.name == "gear_mount"));
    }
}

