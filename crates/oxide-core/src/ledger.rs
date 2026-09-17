use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsageRecord {
    pub timestamp: i64,
    pub session_id: String,
    pub agent_type: String,
    pub model_name: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cached_tokens: u64,
    pub estimated_cost_usd: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenSavingRecord {
    pub timestamp: i64,
    pub session_id: String,
    pub category: String, // "condenser", "read_guard", "graph_outline"
    pub original_bytes: usize,
    pub saved_bytes: usize,
    pub estimated_tokens_saved: u64,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct TokenLedger {
    pub usage_records: Vec<TokenUsageRecord>,
    pub savings_records: Vec<TokenSavingRecord>,
}

impl TokenLedger {
    pub fn new() -> Self {
        Self {
            usage_records: Vec::new(),
            savings_records: Vec::new(),
        }
    }

    pub fn record_usage(&mut self, record: TokenUsageRecord) {
        self.usage_records.push(record);
    }

    pub fn record_saving(
        &mut self,
        session_id: &str,
        category: &str,
        original_bytes: usize,
        saved_bytes: usize,
    ) {
        // Approximate ~4 chars per token
        let estimated_tokens_saved = (saved_bytes / 4) as u64;
        self.savings_records.push(TokenSavingRecord {
            timestamp: chrono::Utc::now().timestamp(),
            session_id: session_id.to_string(),
            category: category.to_string(),
            original_bytes,
            saved_bytes,
            estimated_tokens_saved,
        });
    }

    pub fn total_tokens_used(&self) -> (u64, u64, u64) {
        let (mut inp, mut out, mut cached) = (0, 0, 0);
        for r in &self.usage_records {
            inp += r.input_tokens;
            out += r.output_tokens;
            cached += r.cached_tokens;
        }
        (inp, out, cached)
    }

    pub fn total_tokens_saved(&self) -> u64 {
        self.savings_records
            .iter()
            .map(|r| r.estimated_tokens_saved)
            .sum()
    }

    pub fn total_cost_usd(&self) -> f64 {
        self.usage_records
            .iter()
            .map(|r| r.estimated_cost_usd)
            .sum()
    }

    pub fn generate_summary_report(&self) -> String {
        let (inp, out, cached) = self.total_tokens_used();
        let saved = self.total_tokens_saved();
        let cost = self.total_cost_usd();

        let mut report = String::new();
        report.push_str("================ TOKEN EFFICIENCY & USAGE REPORT ================\n");
        report.push_str(&format!("  * Total Input Tokens:      {:>12}\n", inp));
        report.push_str(&format!("  * Total Output Tokens:     {:>12}\n", out));
        report.push_str(&format!("  * Total Cached Tokens:     {:>12}\n", cached));
        report.push_str(&format!("  * Total Tokens Used:       {:>12}\n", inp + out));
        report.push_str(&format!(
            "  * Total Tokens Saved:      {:>12} (approx ~4 bytes/tok)\n",
            saved
        ));
        report.push_str(&format!("  * Estimated Cost:         ${:>11.4}\n", cost));
        report.push_str("-----------------------------------------------------------------\n");
        report.push_str("  Savings Breakdown by Component:\n");

        let mut cat_map = std::collections::HashMap::new();
        for r in &self.savings_records {
            let entry = cat_map.entry(&r.category).or_insert((0usize, 0u64));
            entry.0 += r.saved_bytes;
            entry.1 += r.estimated_tokens_saved;
        }

        for (cat, (bytes, tokens)) in cat_map {
            report.push_str(&format!(
                "    - {:<18} {:>10} bytes saved (~{:>8} tokens)\n",
                cat, bytes, tokens
            ));
        }
        report.push_str("=================================================================\n");

        report
    }

    pub fn parse_transcript_file<P: AsRef<Path>>(
        transcript_path: P,
        session_id: &str,
    ) -> Result<TokenUsageRecord> {
        let content = fs::read_to_string(transcript_path)?;
        let mut input_tokens = 0u64;
        let mut output_tokens = 0u64;
        let mut cached_tokens = 0u64;

        for line in content.lines() {
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(line)
                && let Some(usage) = val.get("usage").or_else(|| val.get("token_usage"))
            {
                input_tokens += usage
                    .get("input_tokens")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                output_tokens += usage
                    .get("output_tokens")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                cached_tokens += usage
                    .get("cached_tokens")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
            }
        }

        // Fallback calculation if no usage field (estimate from text length)
        if input_tokens == 0 && output_tokens == 0 {
            input_tokens = (content.len() / 4) as u64;
        }

        // Approximate pricing (standard $0.50/M input, $1.50/M output)
        let cost = (input_tokens as f64 * 0.0000005) + (output_tokens as f64 * 0.0000015);

        Ok(TokenUsageRecord {
            timestamp: chrono::Utc::now().timestamp(),
            session_id: session_id.to_string(),
            agent_type: "antigravity".to_string(),
            model_name: "gemini-3.7".to_string(),
            input_tokens,
            output_tokens,
            cached_tokens,
            estimated_cost_usd: cost,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_ledger_calculations() {
        let mut ledger = TokenLedger::new();
        ledger.record_usage(TokenUsageRecord {
            timestamp: 1000,
            session_id: "s1".into(),
            agent_type: "antigravity".into(),
            model_name: "gemini".into(),
            input_tokens: 1000,
            output_tokens: 500,
            cached_tokens: 200,
            estimated_cost_usd: 0.00125,
        });

        ledger.record_saving("s1", "condenser", 8000, 7000);
        ledger.record_saving("s1", "read_guard", 12000, 11000);

        let (inp, out, cached) = ledger.total_tokens_used();
        assert_eq!(inp, 1000);
        assert_eq!(out, 500);
        assert_eq!(cached, 200);

        let saved = ledger.total_tokens_saved();
        assert_eq!(saved, (7000 / 4) + (11000 / 4));

        let report = ledger.generate_summary_report();
        assert!(report.contains("TOKEN EFFICIENCY & USAGE REPORT"));
        assert!(report.contains("condenser"));
        assert!(report.contains("read_guard"));
    }
}
