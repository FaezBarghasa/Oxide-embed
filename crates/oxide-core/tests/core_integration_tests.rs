use oxide_core::condenser::TerminalCondenser;
use oxide_core::config::OxideConfig;
use oxide_core::handoff::HandoffCheckpoint;
use oxide_core::id::{FileId, ProjectId, SymbolId};
use oxide_core::ledger::{TokenLedger, TokenUsageRecord};
use oxide_core::manifest::OxideManifest;
use oxide_core::memify::{BugLogRecord, MemifyEngine};
use oxide_core::read_guard::{ReadGuardDecision, SessionReadGuard};

#[test]
fn test_project_and_file_ids() {
    let p1 = ProjectId::new_v7();
    let p2 = ProjectId::new_v7();
    assert_ne!(p1, p2);

    let fid1 = FileId::from_relative_path("src/lib.rs");
    let fid2 = FileId::from_relative_path("src/lib.rs");
    let fid3 = FileId::from_relative_path("src/main.rs");
    assert_eq!(fid1, fid2);
    assert_ne!(fid1, fid3);

    let sym_id1 = SymbolId::new(&fid1, "calculate_sum");
    let sym_id2 = SymbolId::new(&fid1, "calculate_sum");
    let sym_id3 = SymbolId::new(&fid1, "other_fn");
    assert_eq!(sym_id1, sym_id2);
    assert_ne!(sym_id1, sym_id3);
    assert_eq!(sym_id1.0.len(), 64);
}

#[test]
fn test_manifest_and_config_serialization() {
    let manifest = OxideManifest::new("test_project");
    let toml_str = toml::to_string_pretty(&manifest).expect("serialize manifest toml");
    let parsed: OxideManifest = toml::from_str(&toml_str).expect("deserialize manifest toml");
    assert_eq!(parsed.project_name, manifest.project_name);
    assert_eq!(parsed.schema_version, 1);

    let config = OxideConfig::default_for_project("test_project");
    let config_toml = toml::to_string_pretty(&config).expect("serialize config toml");
    let parsed_config: OxideConfig = toml::from_str(&config_toml).expect("deserialize config toml");
    assert_eq!(parsed_config.index.max_file_kb, config.index.max_file_kb);
}

#[test]
fn test_read_guard_multi_cycle_suppression() {
    let mut guard = SessionReadGuard::new();
    let path = "crates/oxide-db/src/surreal.rs";
    let text = "pub struct SurrealProjectStore { db: Surreal<Db> }";

    let r1 = guard.process_read(path, text, Some("Outline 1"), false);
    match r1 {
        ReadGuardDecision::FullRead { is_first_read, .. } => assert!(is_first_read),
        _ => panic!("Expected FullRead"),
    }

    let r2 = guard.process_read(path, text, Some("Outline 1"), false);
    match r2 {
        ReadGuardDecision::DuplicateSuppressed { read_count, .. } => assert_eq!(read_count, 2),
        _ => panic!("Expected DuplicateSuppressed"),
    }

    let r3 = guard.process_read(path, text, Some("Outline 1"), true);
    match r3 {
        ReadGuardDecision::FullRead { is_first_read, .. } => assert!(!is_first_read),
        _ => panic!("Expected FullRead on force"),
    }
}

#[test]
fn test_condenser_error_caching() {
    let condenser = TerminalCondenser::new(64);
    let tmp = std::env::temp_dir().join("oxide_condenser_test");
    let stderr = "error[E0308]: mismatched types\n  expected i32, found &str\n".repeat(10);

    let res = condenser
        .condense(&tmp, "cargo test", "", &stderr, 101)
        .unwrap();
    assert_eq!(res.exit_code, 101);
    assert!(res.cached_log_path.is_some());
    assert!(res.condensed_text.contains("mismatched types"));
}

#[test]
fn test_handoff_and_ledger() {
    let mut cp = HandoffCheckpoint::new("session_alpha", "Build knowledge graph", "Run clippy");
    cp.completed_tasks.push("Added SurrealDB 3 store".into());
    cp.pending_tasks.push("Write benchmarks".into());
    cp.key_decisions.push("100% offline-first".into());

    let md = cp.to_markdown();
    assert!(md.contains("session_alpha"));
    assert!(md.contains("Build knowledge graph"));
    assert!(md.contains("100% offline-first"));

    let mut ledger = TokenLedger::new();
    ledger.record_usage(TokenUsageRecord {
        timestamp: 1000,
        session_id: "session_alpha".into(),
        agent_type: "antigravity".into(),
        model_name: "gemini-3.7".into(),
        input_tokens: 3000,
        output_tokens: 800,
        cached_tokens: 1200,
        estimated_cost_usd: 0.0027,
    });
    ledger.record_saving("session_alpha", "read_guard", 15000, 14000);

    assert_eq!(ledger.total_tokens_saved(), 3500);
    let rep = ledger.generate_summary_report();
    assert!(rep.contains("read_guard"));
}

#[test]
fn test_memify_decay_and_consolidation() {
    let engine = MemifyEngine::new(10.0, 0.2);
    let now = 1000000;
    let old = now - (20 * 24 * 3600); // 2 half-lives elapsed -> weight ~ 1/4 = 0.25

    let w = engine.calculate_decayed_weight(1.0, old, now);
    assert!(w < 0.3 && w > 0.1);

    let bug1 = BugLogRecord {
        id: "b1".into(),
        timestamp: now,
        symptom: "SurrealKV lock error".into(),
        root_cause: "File lock contention".into(),
        resolution: "Use single connection pool".into(),
        component: "storage_kv".into(),
        embedding: None,
    };
    let rules = engine.consolidate_bugs_to_rules(&[bug1]);
    assert_eq!(rules.len(), 1);
    assert_eq!(rules[0].id, "rule_storage_kv");
}
