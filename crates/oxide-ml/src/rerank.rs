use crate::cosine_similarity;

/// Maximal Marginal Relevance (MMR) diversity reranker.
/// Balances query relevance with diversity to eliminate redundant token consumption.
#[derive(Debug, Clone)]
pub struct MmrReranker {
    pub lambda: f32,
}

impl Default for MmrReranker {
    fn default() -> Self {
        Self { lambda: 0.65 }
    }
}

impl MmrReranker {
    pub fn new(lambda: f32) -> Self {
        Self {
            lambda: lambda.clamp(0.0, 1.0),
        }
    }

    /// Rerank a set of candidates according to MMR.
    ///
    /// - `query_embedding`: The vector representation of the search query.
    /// - `candidates`: A slice of (ID, embedding) pairs.
    /// - `limit`: Maximum number of reranked items to select.
    ///
    /// Returns the indices of selected candidates in ranked order.
    pub fn rerank<T: Clone>(
        &self,
        query_embedding: &[f32],
        candidates: &[(T, Vec<f32>)],
        limit: usize,
    ) -> Vec<(T, f32)> {
        if candidates.is_empty() || limit == 0 {
            return Vec::new();
        }

        let n = candidates.len();
        let target_len = limit.min(n);

        // Precompute relevance scores (cosine sim to query)
        let query_sims: Vec<f32> = candidates
            .iter()
            .map(|(_, emb)| cosine_similarity(query_embedding, emb))
            .collect();

        let mut selected_indices: Vec<usize> = Vec::with_capacity(target_len);
        let mut unselected: Vec<usize> = (0..n).collect();

        // 1. Pick the single most relevant candidate first
        let mut best_first_idx = 0;
        let mut max_first_sim = f32::NEG_INFINITY;
        for (i, &sim) in query_sims.iter().enumerate() {
            if sim > max_first_sim {
                max_first_sim = sim;
                best_first_idx = i;
            }
        }

        selected_indices.push(best_first_idx);
        unselected.retain(|&idx| idx != best_first_idx);

        // 2. Iteratively select candidates maximizing MMR score
        while selected_indices.len() < target_len && !unselected.is_empty() {
            let mut best_score = f32::NEG_INFINITY;
            let mut best_candidate_pos = 0;

            for (pos, &u_idx) in unselected.iter().enumerate() {
                let sim_to_query = query_sims[u_idx];

                // Compute maximum similarity to any already selected candidate
                let mut max_sim_to_selected = f32::NEG_INFINITY;
                for &s_idx in &selected_indices {
                    let sim = cosine_similarity(&candidates[u_idx].1, &candidates[s_idx].1);
                    if sim > max_sim_to_selected {
                        max_sim_to_selected = sim;
                    }
                }

                let mmr_score =
                    self.lambda * sim_to_query - (1.0 - self.lambda) * max_sim_to_selected;

                if mmr_score > best_score {
                    best_score = mmr_score;
                    best_candidate_pos = pos;
                }
            }

            let chosen_idx = unselected.remove(best_candidate_pos);
            selected_indices.push(chosen_idx);
        }

        selected_indices
            .into_iter()
            .map(|idx| (candidates[idx].0.clone(), query_sims[idx]))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mmr_diversity() {
        let reranker = MmrReranker::new(0.5);
        let query = vec![1.0, 0.0, 0.0];

        // Candidate 0: Perfect match
        let c0 = (0, vec![1.0, 0.0, 0.0]);
        // Candidate 1: Identical to candidate 0 (redundant)
        let c1 = (1, vec![0.99, 0.01, 0.0]);
        // Candidate 2: Diverse, still somewhat relevant
        let c2 = (2, vec![0.7, 0.7, 0.0]);

        let candidates = vec![c0, c1, c2];
        let results = reranker.rerank(&query, &candidates, 2);

        assert_eq!(results.len(), 2);
        // First must be c0
        assert_eq!(results[0].0, 0);
        // Second should be c2 because c1 is penalized for high similarity to c0
        assert_eq!(results[1].0, 2);
    }
}
