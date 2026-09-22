use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Language {
    // Code
    Rust,
    Python,
    JavaScript,
    TypeScript,
    Go,
    Java,
    Kotlin,
    C,
    Cpp,
    Mojo,
    Slint,
    Markdown,
    Xml,
    Bash,
    Css,
    Html,
    Json,
    Yaml,
    Toml,
    Gradle,
    Docker,
    Qemu,
    Cfg,
    Csv,
    Svg,
    Svd,
    LinkerScript,
    Assembly,

    // Media & Assets (Metadata indexing)
    Image(ImageFormat),
    Audio(AudioFormat),
    Video(VideoFormat),
    Document(DocFormat),

    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ImageFormat {
    Png,
    Jpeg,
    Svg,
    Webp,
    Gif,
    Ico,
    Bmp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AudioFormat {
    Mp3,
    Wav,
    Flac,
    Ogg,
    Aac,
    M4a,
    Midi,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VideoFormat {
    Mp4,
    Mkv,
    Webm,
    Avi,
    Mov,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DocFormat {
    Pdf,
    Docx,
    Epub,
}

impl Language {
    pub fn from_path<P: AsRef<Path>>(path: P) -> Self {
        let p = path.as_ref();
        let file_name = p
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_lowercase();

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
            // Code
            "rs" => Self::Rust,
            "c" | "h" => Self::C,
            "cpp" | "hpp" | "cc" | "cxx" | "hh" => Self::Cpp,
            "py" | "pyi" => Self::Python,
            "js" | "mjs" | "cjs" | "jsx" => Self::JavaScript,
            "ts" | "mts" | "cts" | "tsx" => Self::TypeScript,
            "go" => Self::Go,
            "java" => Self::Java,
            "kt" | "kts" => Self::Kotlin,
            "mojo" | "🔥" => Self::Mojo,
            "slint" => Self::Slint,
            "sh" | "bash" | "zsh" => Self::Bash,
            "css" | "scss" | "sass" | "less" => Self::Css,
            "html" | "htm" => Self::Html,
            "xml" => Self::Xml,
            "gradle" => Self::Gradle,
            "dockerfile" => Self::Docker,
            "qemu" => Self::Qemu,
            "svd" => Self::Svd,
            "ld" => Self::LinkerScript,
            "s" | "asm" => Self::Assembly,

            // Data & Config
            "json" | "jsonc" | "json5" => Self::Json,
            "yaml" | "yml" => Self::Yaml,
            "toml" => Self::Toml,
            "cfg" | "ini" | "conf" => Self::Cfg,
            "csv" | "tsv" => Self::Csv,
            "md" | "markdown" => Self::Markdown,

            // Images
            "png" => Self::Image(ImageFormat::Png),
            "jpg" | "jpeg" => Self::Image(ImageFormat::Jpeg),
            "svg" => Self::Svg,
            "webp" => Self::Image(ImageFormat::Webp),
            "gif" => Self::Image(ImageFormat::Gif),
            "ico" => Self::Image(ImageFormat::Ico),
            "bmp" => Self::Image(ImageFormat::Bmp),

            // Audio / Sound
            "mp3" => Self::Audio(AudioFormat::Mp3),
            "wav" => Self::Audio(AudioFormat::Wav),
            "flac" => Self::Audio(AudioFormat::Flac),
            "ogg" | "oga" => Self::Audio(AudioFormat::Ogg),
            "aac" => Self::Audio(AudioFormat::Aac),
            "m4a" => Self::Audio(AudioFormat::M4a),
            "mid" | "midi" => Self::Audio(AudioFormat::Midi),

            // Video
            "mp4" | "m4v" => Self::Video(VideoFormat::Mp4),
            "mkv" => Self::Video(VideoFormat::Mkv),
            "webm" => Self::Video(VideoFormat::Webm),
            "avi" => Self::Video(VideoFormat::Avi),
            "mov" => Self::Video(VideoFormat::Mov),

            // Documents
            "pdf" => Self::Document(DocFormat::Pdf),
            "docx" => Self::Document(DocFormat::Docx),
            "epub" => Self::Document(DocFormat::Epub),

            _ => Self::Unknown,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Rust => "rust",
            Self::C => "c",
            Self::Cpp => "cpp",
            Self::Python => "python",
            Self::JavaScript => "javascript",
            Self::TypeScript => "typescript",
            Self::Go => "go",
            Self::Java => "java",
            Self::Kotlin => "kotlin",
            Self::Mojo => "mojo",
            Self::Slint => "slint",
            Self::Bash => "bash",
            Self::Css => "css",
            Self::Html => "html",
            Self::Xml => "xml",
            Self::Gradle => "gradle",
            Self::Docker => "docker",
            Self::Qemu => "qemu",
            Self::Svd => "svd",
            Self::LinkerScript => "linkerscript",
            Self::Assembly => "assembly",
            Self::Json => "json",
            Self::Yaml => "yaml",
            Self::Toml => "toml",
            Self::Cfg => "cfg",
            Self::Csv => "csv",
            Self::Markdown => "markdown",
            Self::Svg => "svg",
            Self::Image(_) => "image",
            Self::Audio(_) => "audio",
            Self::Video(_) => "video",
            Self::Document(_) => "document",
            Self::Unknown => "unknown",
        }
    }

    pub fn is_binary(&self) -> bool {
        match self {
            Self::Svg => false,
            Self::Image(_) | Self::Audio(_) | Self::Video(_) | Self::Document(_) => true,
            _ => false,
        }
    }
}
