use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

use crate::error::{OxideError, Result};
use crate::id::{MemoryId, ProjectId};

/// The 13 typed semantic memory categories (inspired by Memanto and synthesized for systems/embedded codebases).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryKind {
    /// Developer instructions, prompt rules, system constraints
    Instruction,
    /// Domain or hardware facts (e.g. "STM32F401 operates at max 84MHz")
    Fact,
    /// Architectural and design choices (e.g. "Use RTIC v2 with embedded-hal 1.0")
    Decision,
    /// Active or planned objectives (e.g. "Implement USB-MIDI packet buffer")
    Goal,
    /// Cross-session commitments or contracts
    Commitment,
    /// Developer coding preferences (e.g. "Strict no_std, zero unwrap")
    Preference,
    /// Conceptual or architectural relationships between modules
    Relationship,
    /// High-level session background and operational environment
    Context,
    /// Milestones, releases, refactors, benchmarks
    Event,
    /// Lessons learned from debugging (e.g. "BERT position embeddings max 512 tokens")
    Learning,
    /// Agent code observations and telemetry
    Observation,
    /// Key links, binary outputs, generated assets
    Artifact,
    /// Resolved bugs, compiler diagnostics, and panics
    Error,
}

impl MemoryKind {
    pub fn all() -> &'static [MemoryKind] {
        &[
            MemoryKind::Instruction,
            MemoryKind::Fact,
            MemoryKind::Decision,
            MemoryKind::Goal,
            MemoryKind::Commitment,
            MemoryKind::Preference,
            MemoryKind::Relationship,
            MemoryKind::Context,
            MemoryKind::Event,
            MemoryKind::Learning,
            MemoryKind::Observation,
            MemoryKind::Artifact,
            MemoryKind::Error,
        ]
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Instruction => "instruction",
            Self::Fact => "fact",
            Self::Decision => "decision",
            Self::Goal => "goal",
            Self::Commitment => "commitment",
            Self::Preference => "preference",
            Self::Relationship => "relationship",
            Self::Context => "context",
            Self::Event => "event",
            Self::Learning => "learning",
            Self::Observation => "observation",
            Self::Artifact => "artifact",
            Self::Error => "error",
        }
    }

    /// Whether this kind represents a governing rule, constraint, or persistent fact
    pub fn is_governing_rule(&self) -> bool {
        matches!(
            self,
            Self::Instruction | Self::Decision | Self::Preference | Self::Fact
        )
    }
}

impl fmt::Display for MemoryKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for MemoryKind {
    type Err = OxideError;

    fn from_str(s: &str) -> Result<Self> {
        match s.trim().to_lowercase().as_str() {
            "instruction" => Ok(Self::Instruction),
            "fact" => Ok(Self::Fact),
            "decision" => Ok(Self::Decision),
            "goal" => Ok(Self::Goal),
            "commitment" => Ok(Self::Commitment),
            "preference" => Ok(Self::Preference),
            "relationship" => Ok(Self::Relationship),
            "context" => Ok(Self::Context),
            "event" => Ok(Self::Event),
            "learning" => Ok(Self::Learning),
            "observation" => Ok(Self::Observation),
            "artifact" => Ok(Self::Artifact),
            "error" => Ok(Self::Error),
            other => Err(OxideError::Config(format!(
                "Unknown memory kind '{other}'. Valid kinds: {}",
                Self::all()
                    .iter()
                    .map(|k| k.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ))),
        }
    }
}

/// Lifecycle status of a memory entry
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryStatus {
    /// Active and current
    Active,
    /// Superseded by a newer decision or fact
    Superseded,
    /// Contradicted by an active conflicting memory
    Contradicted,
}

impl fmt::Display for MemoryStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Active => write!(f, "active"),
            Self::Superseded => write!(f, "superseded"),
            Self::Contradicted => write!(f, "contradicted"),
        }
    }
}

/// A strongly typed semantic memory record in Oxide-Embed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryRecord {
    pub id: MemoryId,
    pub project_id: ProjectId,
    pub session_id: Option<String>,
    pub kind: MemoryKind,
    pub title: String,
    pub content: String,
    pub tags: Vec<String>,
    pub symbol_ref: Option<String>,
    pub status: MemoryStatus,
    pub superseded_by: Option<MemoryId>,
    pub created_at: DateTime<Utc>,
    pub valid_until: Option<DateTime<Utc>>,
}

impl MemoryRecord {
    pub fn new(
        project_id: ProjectId,
        kind: MemoryKind,
        title: impl Into<String>,
        content: impl Into<String>,
    ) -> Self {
        Self {
            id: MemoryId::new_v7(),
            project_id,
            session_id: None,
            kind,
            title: title.into(),
            content: content.into(),
            tags: Vec::new(),
            symbol_ref: None,
            status: MemoryStatus::Active,
            superseded_by: None,
            created_at: Utc::now(),
            valid_until: None,
        }
    }

    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    pub fn with_symbol_ref(mut self, symbol: impl Into<String>) -> Self {
        self.symbol_ref = Some(symbol.into());
        self
    }

    pub fn with_session_id(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    /// Mark this memory as superseded by a newer memory ID
    pub fn supersede(&mut self, newer_id: &MemoryId) {
        self.status = MemoryStatus::Superseded;
        self.superseded_by = Some(newer_id.clone());
        self.valid_until = Some(Utc::now());
    }

    /// Mark this memory as contradicted
    pub fn mark_contradicted(&mut self) {
        self.status = MemoryStatus::Contradicted;
    }
}

/// Automated conflict and contradiction detection helpers
pub struct ConflictDetector;

impl ConflictDetector {
    /// Heuristic contradiction check between an existing memory and incoming content.
    /// Returns true if polarity or conflicting assertions are detected for the same subject.
    pub fn is_potential_conflict(
        existing: &MemoryRecord,
        incoming_content: &str,
        incoming_kind: MemoryKind,
    ) -> bool {
        if existing.kind != incoming_kind || existing.status != MemoryStatus::Active {
            return false;
        }

        let ex_lower = existing.content.to_lowercase();
        let in_lower = incoming_content.to_lowercase();

        // Contradiction patterns: (use X vs avoid X / don't use X / use Y instead of X)
        let has_opposite_polarity = (ex_lower.contains("never ") && !in_lower.contains("never "))
            || (ex_lower.contains("always ") && in_lower.contains("never "))
            || (ex_lower.contains("prefer ") && in_lower.contains("avoid "))
            || (ex_lower.contains("use ") && in_lower.contains("do not use "))
            || (ex_lower.contains("no_std") && in_lower.contains("with std"));

        // If they share significant common words/tokens and opposite assertions
        let ex_words: std::collections::HashSet<&str> = ex_lower
            .split_whitespace()
            .filter(|w| w.len() > 3)
            .collect();
        let in_words: std::collections::HashSet<&str> = in_lower
            .split_whitespace()
            .filter(|w| w.len() > 3)
            .collect();

        let overlap = ex_words.intersection(&in_words).count();
        overlap >= 2 && has_opposite_polarity
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_kind_roundtrip() {
        for kind in MemoryKind::all() {
            let s = kind.as_str();
            let parsed: MemoryKind = s.parse().expect("parse memory kind");
            assert_eq!(*kind, parsed);
        }
    }

    #[test]
    fn test_memory_record_lifecycle() {
        let proj = ProjectId::new_v7();
        let mut rec = MemoryRecord::new(
            proj,
            MemoryKind::Decision,
            "RTIC Architecture",
            "Use RTIC v2 and pure no_std",
        );
        assert_eq!(rec.status, MemoryStatus::Active);

        let new_id = MemoryId::new_v7();
        rec.supersede(&new_id);
        assert_eq!(rec.status, MemoryStatus::Superseded);
        assert_eq!(rec.superseded_by, Some(new_id));
        assert!(rec.valid_until.is_some());
    }

    #[test]
    fn test_conflict_detector() {
        let proj = ProjectId::new_v7();
        let rec = MemoryRecord::new(
            proj,
            MemoryKind::Decision,
            "Std Library Policy",
            "Always use no_std for embedded drivers",
        );

        let conflicting = "Never use no_std for embedded drivers, always use with std";
        let is_conflict =
            ConflictDetector::is_potential_conflict(&rec, conflicting, MemoryKind::Decision);
        assert!(is_conflict);

        let harmonious = "Also use embedded-hal 1.0 for driver traits";
        let is_conflict_harmonious =
            ConflictDetector::is_potential_conflict(&rec, harmonious, MemoryKind::Decision);
        assert!(!is_conflict_harmonious);
    }
}
