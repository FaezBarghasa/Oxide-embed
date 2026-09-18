pub mod answer;
pub mod budget;
pub mod chunk;
pub mod cognify;
pub mod condenser;
pub mod config;
pub mod context_builder;
pub mod distiller;
pub mod embedding;
pub mod error;
pub mod file;
pub mod handoff;
pub mod id;
pub mod ledger;
pub mod manifest;
pub mod markdown_sync;
pub mod memify;
pub mod memory;
pub mod read_guard;
pub mod symbol;

pub use answer::{AnswerCitation, AnswerSynthesizer, GroundedAnswer};
pub use budget::{
    BudgetCandidate, BudgetItem, BudgetPackResult, TokenBudgetPacker, TokenEstimator,
};
pub use chunk::{ChunkKind, ChunkRecord};
pub use cognify::{CognifyResult, DocReferenceEdge, DocSection};
pub use condenser::{CondensedOutput, TerminalCondenser};
pub use config::OxideConfig;
pub use context_builder::{ContextSynthesizer, SynthesizedContext};
pub use distiller::MemoryDistiller;
pub use embedding::{EmbeddingMetadata, PoolingMethod};
pub use error::{OxideError, Result};
pub use file::FileRecord;
pub use handoff::HandoffCheckpoint;
pub use id::{ChunkId, FileId, MemoryId, ProjectId, SymbolId, WorkspaceId};
pub use ledger::{TokenLedger, TokenSavingRecord, TokenUsageRecord};
pub use manifest::{OxideManifest, resolve_db_path};
pub use markdown_sync::MarkdownMemorySync;
pub use memify::{BugLogRecord, CerebrumRule, DecayStats, MemifyEngine};
pub use memory::{ConflictDetector, MemoryKind, MemoryRecord, MemoryStatus};
pub use read_guard::{ReadGuardDecision, SessionReadGuard, SessionReadRecord};
pub use symbol::{
    CallEdge, ImportEdge, StairHit, SymbolKind, SymbolRecord, deserialize_null_as_empty_vec,
    deserialize_null_as_false,
};
