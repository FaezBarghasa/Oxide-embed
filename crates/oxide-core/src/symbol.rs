use crate::id::{FileId, SymbolId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SymbolKind {
    Function,
    Method,
    Struct,
    Enum,
    Trait,
    Class,
    Interface,
    TypeAlias,
    Constant,
    Static,
    Module,
    Import,
    Macro,
    Other,
}

impl SymbolKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Function => "function",
            Self::Method => "method",
            Self::Struct => "struct",
            Self::Enum => "enum",
            Self::Trait => "trait",
            Self::Class => "class",
            Self::Interface => "interface",
            Self::TypeAlias => "type",
            Self::Constant => "constant",
            Self::Static => "static",
            Self::Module => "module",
            Self::Import => "import",
            Self::Macro => "macro",
            Self::Other => "other",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SymbolRecord {
    pub id: SymbolId,
    pub file_id: FileId,
    pub kind: SymbolKind,
    pub name: String,
    pub qualified_name: Option<String>,
    pub start_line: usize,
    pub end_line: usize,
    pub signature: Option<String>,
    pub doc: Option<String>,
    pub fingerprint: String,
    // STAIR (Structure-Aware Information Retriever) Hierarchy Fields
    #[serde(default, deserialize_with = "deserialize_null_as_false")]
    pub is_macro_node: bool,
    #[serde(default)]
    pub parent_id: Option<SymbolId>,
    #[serde(default, deserialize_with = "deserialize_null_as_empty_vec")]
    pub breadcrumbs: Vec<String>,
    #[serde(default)]
    pub summary: Option<String>,
}

pub fn deserialize_null_as_false<'de, D>(deserializer: D) -> std::result::Result<bool, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let opt = Option::<bool>::deserialize(deserializer)?;
    Ok(opt.unwrap_or(false))
}

pub fn deserialize_null_as_empty_vec<'de, D>(deserializer: D) -> std::result::Result<Vec<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let opt = Option::<Vec<String>>::deserialize(deserializer)?;
    Ok(opt.unwrap_or_default())
}

/// Structural search hit returned by STAIR hierarchical retrieval
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StairHit {
    pub breadcrumbs: Vec<String>,
    pub leaf_symbol: String,
    pub signature: Option<String>,
    pub file_path: String,
    pub start_line: usize,
    pub end_line: usize,
    pub code_body: String,
    pub confidence: f32,
    pub macro_parent: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CallEdge {
    pub caller_symbol_id: SymbolId,
    pub callee_name: String,
    pub line: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImportEdge {
    pub file_id: FileId,
    pub imported_path: String,
    pub imported_symbols: Vec<String>,
}
