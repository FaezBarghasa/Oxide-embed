use crate::budget::{BudgetCandidate, TokenBudgetPacker, TokenEstimator};
use crate::handoff::HandoffCheckpoint;
use crate::memify::CerebrumRule;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SynthesizedContext {
    pub task_query: String,
    pub total_budget: usize,
    pub used_tokens: usize,
    pub active_handoff: Option<HandoffCheckpoint>,
    pub cerebrum_rules: Vec<String>,
    pub packed_snippets: Vec<BudgetCandidate>,
    pub formatted_markdown: String,
}

pub struct ContextSynthesizer;

impl ContextSynthesizer {
    /// Assemble multi-layer cognitive context tailored to a task and fitted to an exact token budget.
    pub fn build(
        task_query: &str,
        handoff: Option<HandoffCheckpoint>,
        rules: Vec<CerebrumRule>,
        graph_candidates: Vec<BudgetCandidate>,
        token_budget: usize,
    ) -> SynthesizedContext {
        let mut candidates = Vec::new();

        // Layer 1: Cerebrum Rules (High Priority)
        for (idx, r) in rules.into_iter().enumerate() {
            let item = BudgetCandidate::new(
                format!("rule_{}", idx),
                "Cerebrum Architectural Rule",
                format!("- [Rule] {}", r.rule),
                90.0, // High score
            );
            candidates.push(item);
        }

        // Layer 2: Graph Context Candidates (Symbols, Call Chains, Linked Docs)
        for gc in graph_candidates {
            candidates.push(gc);
        }

        // Reserve budget for Header & Active Handoff summary (if any)
        let mut handoff_tokens = 0;
        let mut handoff_summary = String::new();
        if let Some(ref h) = handoff {
            handoff_summary = format!(
                "## 🎯 Active Project Objective\n**Objective**: {}\n**Branch**: {}\n**Next Step**: {}\n\n",
                h.objective, h.active_branch, h.next_step
            );
            handoff_tokens = TokenEstimator::estimate_tokens(&handoff_summary);
        }

        let effective_budget = token_budget.saturating_sub(handoff_tokens + 50);
        let packed = TokenBudgetPacker::pack(candidates, effective_budget);

        let mut md = String::new();
        md.push_str(&format!("# 🧠 Oxide-Embed Cognitive Context (Budget: {} tokens)\n\n", token_budget));
        md.push_str(&format!("**Task**: `{}`\n\n", task_query));

        if !handoff_summary.is_empty() {
            md.push_str(&handoff_summary);
        }

        let mut applied_rules = Vec::new();
        let mut code_snippets = Vec::new();

        for item in &packed.included {
            if item.id.starts_with("rule_") {
                applied_rules.push(item.content.clone());
            } else {
                code_snippets.push(item.clone());
            }
        }

        if !applied_rules.is_empty() {
            md.push_str("## 📜 Active Architectural Constraints (Cerebrum)\n");
            for r in &applied_rules {
                md.push_str(&format!("{}\n", r));
            }
            md.push('\n');
        }

        if !code_snippets.is_empty() {
            md.push_str("## 🧩 Relevant Symbols & Code Subgraphs\n");
            for snip in &code_snippets {
                md.push_str(&format!("### {}\n```\n{}\n```\n\n", snip.title, snip.content));
            }
        }

        let total_used = TokenEstimator::estimate_tokens(&md);

        SynthesizedContext {
            task_query: task_query.to_string(),
            total_budget: token_budget,
            used_tokens: total_used,
            active_handoff: handoff,
            cerebrum_rules: applied_rules,
            packed_snippets: code_snippets,
            formatted_markdown: md,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_synthesizer_building() {
        let handoff = HandoffCheckpoint {
            objective: "Implement SPI Flash driver".into(),
            active_branch: "feat/spi".into(),
            files_modified: vec!["crates/spi.rs".into()],
            pending_errors: vec![],
            next_step: "Run probe-rs test".into(),
            created_at: chrono::Utc::now(),
        };

        let rules = vec![CerebrumRule {
            rule: "Never use unwrap in bare-metal SPI handlers".into(),
            source_cluster: Some("spi_faults".into()),
            created_at: chrono::Utc::now(),
        }];

        let candidates = vec![
            BudgetCandidate::new("sym_1", "fn init_spi", "pub fn init_spi() -> Result<()> { Ok(()) }", 50.0),
        ];

        let ctx = ContextSynthesizer::build(
            "configure SPI",
            Some(handoff),
            rules,
            candidates,
            1000,
        );

        assert!(ctx.formatted_markdown.contains("Active Project Objective"));
        assert!(ctx.formatted_markdown.contains("Implement SPI Flash driver"));
        assert!(ctx.formatted_markdown.contains("Never use unwrap"));
        assert!(ctx.formatted_markdown.contains("fn init_spi"));
        assert!(ctx.used_tokens <= 1000);
    }
}
