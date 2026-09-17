use criterion::{Criterion, criterion_group, criterion_main};
use oxide_core::condenser::TerminalCondenser;
use oxide_core::id::{FileId, ProjectId, bytes_to_hex};
use oxide_core::ledger::{TokenLedger, TokenUsageRecord};
use oxide_core::memify::MemifyEngine;
use oxide_core::read_guard::SessionReadGuard;
use sha2::{Digest, Sha256};
use std::hint::black_box;

fn bench_id_and_hashing(c: &mut Criterion) {
    let mut group = c.benchmark_group("core_id_and_hashing");

    group.bench_function("project_id_uuidv7", |b| {
        b.iter(ProjectId::new_v7);
    });

    group.bench_function("file_id_derivation", |b| {
        let path = "crates/oxide-db/src/surreal.rs";
        b.iter(|| FileId::from_relative_path(black_box(path)));
    });

    let sample_payload = "fn calculate_total(a: i32, b: i32) -> i32 { a + b }".repeat(50);
    group.bench_function("sha256_hashing_throughput", |b| {
        b.iter(|| {
            let mut hasher = Sha256::new();
            hasher.update(black_box(sample_payload.as_bytes()));
            bytes_to_hex(&hasher.finalize())
        });
    });

    group.finish();
}

fn bench_context_hygiene(c: &mut Criterion) {
    let mut group = c.benchmark_group("core_context_hygiene");

    let mut guard = SessionReadGuard::new();
    let file_path = "crates/oxide-core/src/lib.rs";
    let file_content = "pub mod chunk;\npub mod cognify;\npub mod condenser;\n".repeat(100);
    let outline = "  - Module chunk (L1-L1)\n  - Module cognify (L2-L2)\n";

    // Warm up
    guard.process_read(file_path, &file_content, Some(outline), false);

    group.bench_function("session_read_guard_cached_lookup", |b| {
        b.iter(|| {
            guard.process_read(
                black_box(file_path),
                black_box(&file_content),
                black_box(Some(outline)),
                black_box(false),
            )
        });
    });

    let condenser = TerminalCondenser::new(512);
    let tmp_dir = std::env::temp_dir().join("oxide_bench_condenser");
    let massive_stderr =
        "warning: unused variable `x`\nerror[E0425]: cannot find value `foo` in this scope\n"
            .repeat(100);

    group.bench_function("terminal_condenser_error_processing", |b| {
        b.iter(|| {
            condenser.condense(
                black_box(&tmp_dir),
                black_box("cargo build"),
                black_box(""),
                black_box(&massive_stderr),
                black_box(101),
            )
        });
    });

    group.finish();
}

fn bench_ledger_and_memify(c: &mut Criterion) {
    let mut group = c.benchmark_group("core_ledger_and_memify");

    let mut ledger = TokenLedger::new();
    for i in 0..100 {
        ledger.record_usage(TokenUsageRecord {
            timestamp: 1700000000 + i,
            session_id: format!("session_{}", i),
            agent_type: "antigravity".into(),
            model_name: "gemini-3.7".into(),
            input_tokens: 1500,
            output_tokens: 400,
            cached_tokens: 600,
            estimated_cost_usd: 0.0015,
        });
        ledger.record_saving("session_1", "condenser", 8000, 7200);
    }

    group.bench_function("token_ledger_report_generation", |b| {
        b.iter(|| black_box(&ledger).generate_summary_report());
    });

    let memify = MemifyEngine::new(30.0, 0.15);
    let now = 1700000000i64;
    let access = 1690000000i64;

    group.bench_function("memify_decay_calculation", |b| {
        b.iter(|| {
            let w =
                memify.calculate_decayed_weight(black_box(1.0), black_box(access), black_box(now));
            memify.should_prune(w)
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_id_and_hashing,
    bench_context_hygiene,
    bench_ledger_and_memify
);
criterion_main!(benches);
