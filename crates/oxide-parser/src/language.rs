use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Language {
    Rust,
    Python,
    JavaScript,
    TypeScript,
    Go,
    Java,
    Kotlin,
    Mojo,
    Slint,
    Markdown,
    Xml,
    Html,
    Json,
    Yaml,
    Docker,
    Qemu,
    Cfg,
    Svg,
    Pdf,
    Docx,
    Csv,
    Gradle,
    Bash,
    Css,
    Toml,
    Unknown,
}

impl Language {
    pub fn from_path<P: AsRef<Path>>(path: P) -> Self {
        let p = path.as_ref();
        let file_name = p
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_lowercase();

        // Special filename checks
        if file_name == "dockerfile" || file_name.starts_with("dockerfile.") {
            return Self::Docker;
        }
        if file_name.ends_with(".gradle") || file_name.ends_with(".gradle.kts") {
            return Self::Gradle;
        }

        let ext = p
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_lowercase();

        match ext.as_str() {
            "rs" => Self::Rust,
            "py" | "pyi" => Self::Python,
            "js" | "mjs" | "cjs" | "jsx" => Self::JavaScript,
            "ts" | "mts" | "cts" | "tsx" => Self::TypeScript,
            "go" => Self::Go,
            "java" => Self::Java,
            "kt" | "kts" => Self::Kotlin,
            "mojo" | "🔥" => Self::Mojo,
            "slint" => Self::Slint,
            "md" | "markdown" => Self::Markdown,
            "xml" => Self::Xml,
            "html" | "htm" => Self::Html,
            "json" | "jsonc" | "json5" => Self::Json,
            "yaml" | "yml" => Self::Yaml,
            "dockerfile" => Self::Docker,
            "qemu" => Self::Qemu,
            "cfg" | "ini" | "conf" => Self::Cfg,
            "svg" => Self::Svg,
            "pdf" => Self::Pdf,
            "docx" => Self::Docx,
            "csv" | "tsv" => Self::Csv,
            "gradle" => Self::Gradle,
            "sh" | "bash" | "zsh" => Self::Bash,
            "css" | "scss" | "sass" | "less" => Self::Css,
            "toml" => Self::Toml,
            _ => Self::Unknown,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Rust => "rust",
            Self::Python => "python",
            Self::JavaScript => "javascript",
            Self::TypeScript => "typescript",
            Self::Go => "go",
            Self::Java => "java",
            Self::Kotlin => "kotlin",
            Self::Mojo => "mojo",
            Self::Slint => "slint",
            Self::Markdown => "markdown",
            Self::Xml => "xml",
            Self::Html => "html",
            Self::Json => "json",
            Self::Yaml => "yaml",
            Self::Docker => "docker",
            Self::Qemu => "qemu",
            Self::Cfg => "cfg",
            Self::Svg => "svg",
            Self::Pdf => "pdf",
            Self::Docx => "docx",
            Self::Csv => "csv",
            Self::Gradle => "gradle",
            Self::Bash => "bash",
            Self::Css => "css",
            Self::Toml => "toml",
            Self::Unknown => "unknown",
        }
    }

    pub fn is_binary(&self) -> bool {
        matches!(self, Self::Pdf | Self::Docx)
    }

    pub fn is_tree_sitter_supported(&self) -> bool {
        matches!(
            self,
            Self::Rust
                | Self::Python
                | Self::JavaScript
                | Self::TypeScript
                | Self::Go
                | Self::Java
                | Self::Bash
                | Self::Html
                | Self::Css
                | Self::Json
                | Self::Yaml
                | Self::Markdown
        )
    }
}
