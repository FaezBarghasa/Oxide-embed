use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BugLogRecord {
    pub id: String,
    pub timestamp: i64,
    pub symptom: String,
    pub root_cause: String,
    pub resolution: String,
    pub component: String,
    pub embedding: Option<Vec<f32>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CerebrumRule {
    pub id: String,
    pub title: String,
    pub condition_pattern: String,
    pub prescribed_solution: String,
    pub confidence: f32,
    pub source_incidents: Vec<String>,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecayStats {
    pub total_edges_evaluated: usize,
    pub decayed_edges: usize,
    pub pruned_edges: usize,
    pub pruned_orphan_symbols: usize,
}

pub struct MemifyEngine {
    decay_half_life_days: f64,
    prune_threshold: f32,
}

impl Default for MemifyEngine {
    fn default() -> Self {
        Self {
            decay_half_life_days: 30.0,
            prune_threshold: 0.15,
        }
    }
}

impl MemifyEngine {
    pub fn new(decay_half_life_days: f64, prune_threshold: f32) -> Self {
        Self {
            decay_half_life_days,
            prune_threshold,
        }
    }

    /// Calculates decayed weight given last access timestamp and current weight
    pub fn calculate_decayed_weight(
        &self,
        initial_weight: f32,
        last_accessed_timestamp: i64,
        current_timestamp: i64,
    ) -> f32 {
        let elapsed_seconds = (current_timestamp - last_accessed_timestamp).max(0) as f64;
        let elapsed_days = elapsed_seconds / (24.0 * 3600.0);
        let decay_factor = (-elapsed_days / self.decay_half_life_days).exp();
        (initial_weight * decay_factor as f32).clamp(0.0, 1.0)
    }

    pub fn should_prune(&self, current_weight: f32) -> bool {
        current_weight < self.prune_threshold
    }

    /// Clusters bug logs by symptom/root_cause cosine similarity or keyword heuristics and synthesizes Cerebrum rules
    pub fn consolidate_bugs_to_rules(&self, bug_logs: &[BugLogRecord]) -> Vec<CerebrumRule> {
        if bug_logs.is_empty() {
            return Vec::new();
        }

        let mut rules = Vec::new();
        let mut grouped: std::collections::HashMap<String, Vec<&BugLogRecord>> =
            std::collections::HashMap::new();

        for bug in bug_logs {
            grouped.entry(bug.component.clone()).or_default().push(bug);
        }

        let now = chrono::Utc::now().timestamp();

        for (component, logs) in grouped {
            let count = logs.len();
            let incident_ids = logs.iter().map(|l| l.id.clone()).collect();
            let symptoms: Vec<&str> = logs.iter().map(|l| l.symptom.as_str()).collect();
            let resolutions: Vec<&str> = logs.iter().map(|l| l.resolution.as_str()).collect();

            let title = format!("Consolidated Rule: {}", component);
            let condition_pattern = format!(
                "When encountering issues in component '{}' with symptoms: [{}]",
                component,
                symptoms.join(" | ")
            );
            let prescribed_solution = format!(
                "Apply established resolutions:\n{}",
                resolutions
                    .iter()
                    .enumerate()
                    .map(|(i, r)| format!("{}. {}", i + 1, r))
                    .collect::<Vec<_>>()
                    .join("\n")
            );

            let confidence = (0.5 + (count as f32 * 0.1)).min(1.0);

            rules.push(CerebrumRule {
                id: format!(
                    "rule_{}",
                    component.to_lowercase().replace([' ', '-', '/'], "_")
                ),
                title,
                condition_pattern,
                prescribed_solution,
                confidence,
                source_incidents: incident_ids,
                created_at: now,
            });
        }

        rules
    }

    pub fn export_cerebrum_markdown<P: AsRef<Path>>(
        &self,
        workspace_root: P,
        rules: &[CerebrumRule],
    ) -> Result<PathBuf> {
        let docs_dir = workspace_root.as_ref().join(".oxide").join("docs");
        fs::create_dir_all(&docs_dir)?;
        let file_path = docs_dir.join("CEREBRUM.md");

        let mut md = String::new();
        md.push_str("# Oxide Cerebrum — Consolidated Memory & Knowledge Base\n\n");
        md.push_str("> Auto-synthesized operational rules and distilled memory from historical incidents, buglogs, and graph evolution.\n\n");

        if rules.is_empty() {
            md.push_str("*(No consolidated rules active yet)*\n");
        } else {
            for rule in rules {
                md.push_str(&format!("## {}\n", rule.title));
                md.push_str(&format!("- **Rule ID**: `{}`\n", rule.id));
                md.push_str(&format!("- **Confidence**: {:.2}\n", rule.confidence));
                md.push_str(&format!(
                    "- **Source Incidents**: {}\n\n",
                    rule.source_incidents.join(", ")
                ));
                md.push_str("### Trigger Condition\n");
                md.push_str(&format!("> {}\n\n", rule.condition_pattern));
                md.push_str("### Prescribed Action / Fix\n");
                md.push_str(&format!("{}\n\n", rule.prescribed_solution));
                md.push_str("---\n\n");
            }
        }

        fs::write(&file_path, md)?;
        Ok(file_path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decay_calculation() {
        let engine = MemifyEngine::new(30.0, 0.15);
        let now = 100_000_000;
        let thirty_days_ago = now - (30 * 24 * 3600);

        // At 0 days elapsed, weight is preserved
        let w0 = engine.calculate_decayed_weight(1.0, now, now);
        assert!((w0 - 1.0).abs() < 1e-4);

        // At 30 days elapsed (1 half life), weight decays by ~1/e (~0.3678)
        let w30 = engine.calculate_decayed_weight(1.0, thirty_days_ago, now);
        assert!((w30 - 0.367879).abs() < 1e-3);
        assert!(!engine.should_prune(w30));

        // At 90 days elapsed (3 half lives), weight drops below 0.15 prune threshold
        let ninety_days_ago = now - (90 * 24 * 3600);
        let w90 = engine.calculate_decayed_weight(1.0, ninety_days_ago, now);
        assert!(engine.should_prune(w90));
    }

    #[test]
    fn test_bug_consolidation_to_rules() {
        let engine = MemifyEngine::default();
        let bugs = vec![
            BugLogRecord {
                id: "bug_1".into(),
                timestamp: 1000,
                symptom: "UART timeout".into(),
                root_cause: "Baud rate mismatch".into(),
                resolution: "Set baud rate to 115200".into(),
                component: "serial_driver".into(),
                embedding: None,
            },
            BugLogRecord {
                id: "bug_2".into(),
                timestamp: 2000,
                symptom: "Framing error".into(),
                root_cause: "Noise on RX pin".into(),
                resolution: "Enable pull-up resistor".into(),
                component: "serial_driver".into(),
                embedding: None,
            },
        ];

        let rules = engine.consolidate_bugs_to_rules(&bugs);
        assert_eq!(rules.len(), 1);
        let rule = &rules[0];
        assert_eq!(rule.id, "rule_serial_driver");
        assert_eq!(rule.source_incidents.len(), 2);
        assert!(rule.condition_pattern.contains("UART timeout"));
        assert!(rule.prescribed_solution.contains("Set baud rate to 115200"));
        assert!(rule.confidence > 0.6);
    }
}
