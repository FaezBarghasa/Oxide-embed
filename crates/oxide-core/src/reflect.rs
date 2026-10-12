use serde::{Deserialize, Serialize};

use crate::budget::{BudgetItem, TokenBudgetPacker, TokenEstimator};
use crate::model::MentalModel;
use crate::retrieval::{FusedItem, RrfCandidate, RrfScorer};

/// The result of a reflect operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReflectResult {
    pub query: String,
    pub standing_model: Option<MentalModel>,
    pub answer: String,
    pub citations: Vec<String>,
    pub token_count: usize,
    pub is_from_standing_model: bool,
}

/// The cognitive reflection engine managing tiered retrieval (Mental Model -> 4-Way RRF -> Grounded Synthesis).
pub struct ReflectEngine {
    pub rrf_scorer: RrfScorer,
    pub token_budget: usize,
}

impl Default for ReflectEngine {
    fn default() -> Self {
        Self {
            rrf_scorer: RrfScorer::default(),
            token_budget: 1500,
        }
    }
}

impl ReflectEngine {
    pub fn new(token_budget: usize) -> Self {
        Self {
            rrf_scorer: RrfScorer::default(),
            token_budget,
        }
    }

    /// Evaluates if a matching fresh Mental Model answers the query.
    pub fn resolve_standing_model(
        &self,
        candidate_models: &[MentalModel],
    ) -> Option<MentalModel> {
        for model in candidate_models {
            if !model.is_stale {
                return Some(model.clone());
            }
        }
        None
    }

    /// Synthesizes an evidence-grounded reflection from 4-way fused candidates.
    pub fn synthesize_from_fused(
        &self,
        query: &str,
        fused_items: &[FusedItem],
    ) -> ReflectResult {
        let budget_candidates: Vec<_> = fused_items
            .iter()
            .map(|item| item.to_budget_candidate())
            .collect();

        let packer = TokenBudgetPacker::new(self.token_budget);
        let packed = packer.pack(&budget_candidates);

        let mut citations = Vec::new();
        let mut evidence_md = String::new();

        evidence_md.push_str(&format!("# Reflection: {}\n\n", query));
        evidence_md.push_str("## Grounded Evidence\n\n");

        for item in &packed.selected {
            citations.push(item.id.clone());
            evidence_md.push_str(&format!("### {}\n", item.title));
            evidence_md.push_str(&item.content);
            evidence_md.push_str("\n\n");
        }

        let total_tokens = TokenEstimator::estimate_tokens(&evidence_md);

        ReflectResult {
            query: query.to_string(),
            standing_model: None,
            answer: evidence_md,
            citations,
            token_count: total_tokens,
            is_from_standing_model: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reflect_engine_standing_model() {
        let engine = ReflectEngine::default();
        let mut model = MentalModel::new(
            "auth",
            "How does auth work?",
            "Auth uses PASETO v4 asymmetric keys.",
        );
        model.is_stale = false;

        let res = engine.resolve_standing_model(&[model.clone()]);
        assert!(res.is_some());
        assert_eq!(res.unwrap().topic, "auth");
    }

    #[test]
    fn test_reflect_engine_stale_model_rejected() {
        let engine = ReflectEngine::default();
        let mut model = MentalModel::new(
            "auth",
            "How does auth work?",
            "Auth uses old tokens.",
        );
        model.is_stale = true;

        let res = engine.resolve_standing_model(&[model]);
        assert!(res.is_none());
    }
}
