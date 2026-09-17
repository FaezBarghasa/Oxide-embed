use serde::{Deserialize, Serialize};

/// Lightweight, deterministic token estimation for code and markdown.
/// Uses conservative heuristic of ~3.8 characters per token for English text & code identifiers.
pub struct TokenEstimator;

impl TokenEstimator {
    #[inline]
    pub fn estimate_tokens(text: &str) -> usize {
        if text.is_empty() {
            return 0;
        }
        // Count whitespace-delimited words and structural punctuation
        let char_count = text.chars().count();
        let word_count = text.split_whitespace().count();
        // Heuristic blend: max(word_count, char_count / 4) + structural overhead
        let base = (char_count as f64 / 3.8).ceil() as usize;
        base.max(word_count).max(1)
    }
}

/// Trait for items that can be packed into a token budget.
pub trait BudgetItem {
    fn id(&self) -> &str;
    fn score(&self) -> f32;
    fn content(&self) -> &str;
    fn estimated_tokens(&self) -> usize {
        TokenEstimator::estimate_tokens(self.content())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetCandidate {
    pub id: String,
    pub title: String,
    pub content: String,
    pub score: f32,
    pub estimated_tokens: usize,
}

impl BudgetCandidate {
    pub fn new(id: impl Into<String>, title: impl Into<String>, content: impl Into<String>, score: f32) -> Self {
        let content_str = content.into();
        let tokens = TokenEstimator::estimate_tokens(&content_str);
        Self {
            id: id.into(),
            title: title.into(),
            content: content_str,
            score,
            estimated_tokens: tokens,
        }
    }
}

impl BudgetItem for BudgetCandidate {
    fn id(&self) -> &str {
        &self.id
    }

    fn score(&self) -> f32 {
        self.score
    }

    fn content(&self) -> &str {
        &self.content
    }

    fn estimated_tokens(&self) -> usize {
        self.estimated_tokens
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetPackResult<T> {
    pub total_budget: usize,
    pub used_tokens: usize,
    pub included: Vec<T>,
    pub dropped_count: usize,
}

/// Greedy Knapsack Token Budget Packer.
/// Maximizes information density within strict token ceilings.
pub struct TokenBudgetPacker;

impl TokenBudgetPacker {
    pub fn pack<T: BudgetItem + Clone>(
        items: Vec<T>,
        budget_tokens: usize,
    ) -> BudgetPackResult<T> {
        let mut sorted = items;
        // Sort descending by score. Tie-break with smaller token footprint (higher density).
        sorted.sort_by(|a, b| {
            b.score()
                .partial_cmp(&a.score())
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.estimated_tokens().cmp(&b.estimated_tokens()))
        });

        let mut included = Vec::new();
        let mut used_tokens = 0;
        let mut dropped_count = 0;

        for item in sorted {
            let item_tokens = item.estimated_tokens();
            if used_tokens + item_tokens <= budget_tokens {
                used_tokens += item_tokens;
                included.push(item);
            } else {
                dropped_count += 1;
            }
        }

        BudgetPackResult {
            total_budget: budget_tokens,
            used_tokens,
            included,
            dropped_count,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_estimator() {
        let text = "pub fn calculate_total(items: &[Item]) -> u64 { items.iter().map(|i| i.price).sum() }";
        let est = TokenEstimator::estimate_tokens(text);
        assert!(est > 10 && est < 40);
        assert_eq!(TokenEstimator::estimate_tokens(""), 0);
    }

    #[test]
    fn test_budget_packer_greedy_fitting() {
        let c1 = BudgetCandidate::new("c1", "High Priority", "short code", 10.0);
        let c2 = BudgetCandidate::new("c2", "Medium Priority", "medium length code block with some explanation", 5.0);
        let c3 = BudgetCandidate::new("c3", "Low Priority Huge", "a very long code block ".repeat(50), 1.0);

        let items = vec![c2.clone(), c3.clone(), c1.clone()];
        let packed = TokenBudgetPacker::pack(items, 30);

        // c1 and c2 should be included, huge c3 dropped
        assert_eq!(packed.included.len(), 2);
        assert_eq!(packed.included[0].id, "c1");
        assert_eq!(packed.included[1].id, "c2");
        assert_eq!(packed.dropped_count, 1);
        assert!(packed.used_tokens <= 30);
    }
}
