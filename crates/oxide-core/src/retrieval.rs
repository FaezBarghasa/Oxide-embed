use std::collections::HashMap;

use crate::budget::BudgetCandidate;

/// A candidate retrieved from one of the search strategies.
#[derive(Debug, Clone)]
pub struct RrfCandidate {
    pub key: String,
    pub title: String,
    pub content: String,
    pub source_type: String,
    pub raw_score: f32,
    pub file_path: Option<String>,
    pub start_line: Option<usize>,
    pub end_line: Option<usize>,
}

/// A fused candidate with individual strategy ranks and unified RRF score.
#[derive(Debug, Clone)]
pub struct FusedItem {
    pub candidate: RrfCandidate,
    pub rrf_score: f32,
    pub vector_rank: Option<usize>,
    pub lexical_rank: Option<usize>,
    pub graph_rank: Option<usize>,
    pub temporal_rank: Option<usize>,
}

impl FusedItem {
    pub fn to_budget_candidate(&self) -> BudgetCandidate {
        BudgetCandidate::new(
            &self.candidate.key,
            &self.candidate.title,
            &self.candidate.content,
            self.rrf_score,
        )
    }
}

/// 4-Way Reciprocal Rank Fusion (RRF) Scorer combining vector, lexical, graph, and temporal retrieval.
#[derive(Debug, Clone)]
pub struct RrfScorer {
    pub k: f32,
    pub vector_weight: f32,
    pub lexical_weight: f32,
    pub graph_weight: f32,
    pub temporal_weight: f32,
}

impl Default for RrfScorer {
    fn default() -> Self {
        Self {
            k: 60.0,
            vector_weight: 0.35,
            lexical_weight: 0.35,
            graph_weight: 0.20,
            temporal_weight: 0.10,
        }
    }
}

impl RrfScorer {
    pub fn new(k: f32, w_vec: f32, w_lex: f32, w_graph: f32, w_temp: f32) -> Self {
        Self {
            k,
            vector_weight: w_vec,
            lexical_weight: w_lex,
            graph_weight: w_graph,
            temporal_weight: w_temp,
        }
    }

    /// Merges multiple parallel candidate ranked streams into a single RRF-ranked list.
    pub fn fuse(
        &self,
        vector_ranked: &[RrfCandidate],
        lexical_ranked: &[RrfCandidate],
        graph_ranked: &[RrfCandidate],
        temporal_ranked: &[RrfCandidate],
    ) -> Vec<FusedItem> {
        let mut map: HashMap<
            String,
            (
                RrfCandidate,
                Option<usize>,
                Option<usize>,
                Option<usize>,
                Option<usize>,
            ),
        > = HashMap::new();

        for (rank, item) in vector_ranked.iter().enumerate() {
            let entry = map
                .entry(item.key.clone())
                .or_insert_with(|| (item.clone(), None, None, None, None));
            entry.1 = Some(rank + 1);
        }

        for (rank, item) in lexical_ranked.iter().enumerate() {
            let entry = map
                .entry(item.key.clone())
                .or_insert_with(|| (item.clone(), None, None, None, None));
            entry.2 = Some(rank + 1);
        }

        for (rank, item) in graph_ranked.iter().enumerate() {
            let entry = map
                .entry(item.key.clone())
                .or_insert_with(|| (item.clone(), None, None, None, None));
            entry.3 = Some(rank + 1);
        }

        for (rank, item) in temporal_ranked.iter().enumerate() {
            let entry = map
                .entry(item.key.clone())
                .or_insert_with(|| (item.clone(), None, None, None, None));
            entry.4 = Some(rank + 1);
        }

        let mut fused = Vec::with_capacity(map.len());
        for (_key, (candidate, v_r, l_r, g_r, t_r)) in map {
            let mut score = 0.0;
            if let Some(r) = v_r {
                score += self.vector_weight / (self.k + r as f32);
            }
            if let Some(r) = l_r {
                score += self.lexical_weight / (self.k + r as f32);
            }
            if let Some(r) = g_r {
                score += self.graph_weight / (self.k + r as f32);
            }
            if let Some(r) = t_r {
                score += self.temporal_weight / (self.k + r as f32);
            }

            fused.push(FusedItem {
                candidate,
                rrf_score: score,
                vector_rank: v_r,
                lexical_rank: l_r,
                graph_rank: g_r,
                temporal_rank: t_r,
            });
        }

        fused.sort_by(|a, b| {
            b.rrf_score
                .partial_cmp(&a.rrf_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        fused
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_candidate(key: &str) -> RrfCandidate {
        RrfCandidate {
            key: key.to_string(),
            title: key.to_string(),
            content: format!("Content for {}", key),
            source_type: "symbol".to_string(),
            raw_score: 1.0,
            file_path: Some("src/lib.rs".to_string()),
            start_line: Some(1),
            end_line: Some(10),
        }
    }

    #[test]
    fn test_rrf_fusion_priority() {
        let scorer = RrfScorer::default();

        let v1 = dummy_candidate("item_a");
        let v2 = dummy_candidate("item_b");

        let l1 = dummy_candidate("item_a");
        let l2 = dummy_candidate("item_c");

        let fused = scorer.fuse(&[v1, v2], &[l1, l2], &[], &[]);

        assert!(!fused.is_empty());
        // item_a is in both vector and lexical at rank 1, so it should have the highest score
        assert_eq!(fused[0].candidate.key, "item_a");
        assert!(fused[0].vector_rank.is_some());
        assert!(fused[0].lexical_rank.is_some());
        assert!(fused[0].graph_rank.is_none());
    }

    #[test]
    fn test_rrf_empty_streams() {
        let scorer = RrfScorer::default();
        let fused = scorer.fuse(&[], &[], &[], &[]);
        assert!(fused.is_empty());
    }
}
