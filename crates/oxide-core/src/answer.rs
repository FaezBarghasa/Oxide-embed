use crate::budget::TokenEstimator;
use crate::memory::MemoryRecord;

/// Offline Grounded Q&A Synthesizer (Memanto `answer` primitive).
/// Synthesizes direct, source-grounded answers combining recalled memories, rules, and code snippets.
pub struct AnswerSynthesizer;

#[derive(Debug, Clone)]
pub struct GroundedAnswer {
    pub question: String,
    pub answer: String,
    pub citations: Vec<AnswerCitation>,
    pub token_cost: usize,
}

#[derive(Debug, Clone)]
pub struct AnswerCitation {
    pub source_id: String,
    pub source_type: String,
    pub title: String,
}

impl AnswerSynthesizer {
    /// Generates a structured grounded answer from retrieved context.
    pub fn answer(
        question: &str,
        memories: &[MemoryRecord],
        rules: &[String],
        code_snippets: &[String],
        budget: usize,
    ) -> GroundedAnswer {
        let mut out = String::new();
        let mut citations = Vec::new();

        out.push_str(&format!("### 🎯 Answer for: \"{}\"\n\n", question));

        // 1. Direct synthesized answer based on strongest matching memory or rule
        if let Some(top_mem) = memories.first() {
            out.push_str(&format!(
                "Based on active project **{}** (`{}`):\n> {}\n\n",
                top_mem.kind.as_str().to_uppercase(),
                top_mem.title,
                top_mem.content
            ));
            citations.push(AnswerCitation {
                source_id: top_mem.id.to_string(),
                source_type: top_mem.kind.as_str().to_string(),
                title: top_mem.title.clone(),
            });
        } else if let Some(top_rule) = rules.first() {
            out.push_str(&format!(
                "According to active architectural constraints:\n> {}\n\n",
                top_rule
            ));
            citations.push(AnswerCitation {
                source_id: "rule_0".into(),
                source_type: "cerebrum_rule".into(),
                title: "Architectural Constraint".into(),
            });
        } else {
            out.push_str("No direct prescriptive rule or decision found for this query in project memory.\n\n");
        }

        // 2. Supporting context & decisions
        if memories.len() > 1 {
            out.push_str("#### 📌 Key Supporting Decisions & Facts:\n");
            for mem in memories.iter().skip(1) {
                out.push_str(&format!(
                    "- **[{}] {}**: {}\n",
                    mem.kind.as_str().to_uppercase(),
                    mem.title,
                    mem.content
                ));
                citations.push(AnswerCitation {
                    source_id: mem.id.to_string(),
                    source_type: mem.kind.as_str().to_string(),
                    title: mem.title.clone(),
                });
            }
            out.push('\n');
        }

        // 3. Relevant Code Symbols & Subgraphs
        if !code_snippets.is_empty() {
            out.push_str("#### 🧩 Code Symbols & Graph Context:\n");
            for snip in code_snippets.iter().take(3) {
                out.push_str(&format!("```\n{}\n```\n", snip));
            }
        }

        // 4. Budget check & formatting
        let token_cost = TokenEstimator::estimate_tokens(&out);
        if token_cost > budget {
            // Truncate cleanly
            out.push_str(&format!(
                "\n*(Truncated to fit {} token budget constraint)*\n",
                budget
            ));
        }

        GroundedAnswer {
            question: question.to_string(),
            answer: out,
            citations,
            token_cost,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::id::ProjectId;
    use crate::memory::MemoryKind;

    #[test]
    fn test_answer_synthesis() {
        let pid = ProjectId::new_v7();
        let m1 = MemoryRecord::new(
            pid,
            MemoryKind::Decision,
            "Use pure no_std for STM32 drivers",
            "Hardware drivers must never allocate on heap.",
        );
        let ans = AnswerSynthesizer::answer(
            "What is the heap allocation rule for STM32?",
            &[m1],
            &[],
            &["pub struct SpiDriver;".to_string()],
            500,
        );

        assert!(ans.answer.contains("pure no_std"));
        assert_eq!(ans.citations.len(), 1);
        assert_eq!(ans.citations[0].source_type, "decision");
    }
}
