# Oxide-embed Test-Driven Development (TDD) & Quality Assurance

This document details the test strategy, verification matrix, regression suites, and quality gates for **Oxide-embed**.

---

## 1. Testing Philosophy & Standards

Oxide-embed enforces strict systems engineering standards:
- **Zero `.unwrap()` in Production Code**: Error paths must propagate typed errors via `Result<T, OxideError>`.
- **Pure Local Verification**: All tests execute offline against in-memory or temporary local SurrealKV database instances without external network services.
- **Deterministic AST & Fusion Validation**: Synthesizers, packers, and RRF algorithms must maintain deterministic outputs.
- **Zero Clippy Warnings**: All code must pass `cargo clippy --workspace --all-targets -- -D warnings`.

---

## 2. Test Suite Matrix (50 Passed Tests)

```
Test Execution Summary:
├── oxide-core:    26 tests passed (20 unit, 6 integration)
├── oxide-db:       2 tests passed (SurrealKV CRUD, GraphRAG BFS)
├── oxide-ml:       7 tests passed (MMR reranking, batched embeddings, Gemma/Qwen)
└── oxide-parser:  15 tests passed (Tree-sitter AST, languages, slice extraction)
Total:             50 passed, 0 failed, 0 ignored
```

### A. `oxide-core` Test Suite (26 Tests)
- **Token Budget & Knapsack Packing**: `test_token_estimator`, `test_budget_packer_greedy_fitting` validate greedy knapsack token packing within strict boundaries.
- **Answer Synthesis & Citations**: `test_answer_synthesis` validates grounded citation generation.
- **Terminal Condenser**: `test_condenser_small_output`, `test_condenser_large_error_output` test ANSI stripping and log caching.
- **Git Commit Distillation**: `test_distill_conventional_commits` validates automatic parsing of conventional commits into `decision` and `learning` records.
- **Cognitive Reflection**: `test_reflect_engine_standing_model`, `test_reflect_engine_stale_model_rejected` test standing mental model hit/miss and stale invalidation.
- **Reciprocal Rank Fusion (RRF)**: `test_rrf_empty_streams`, `test_rrf_fusion_priority` ensure multi-stream rank combination adheres to $RRF = \sum \frac{1}{60 + rank}$.
- **Temporal Decay & Polarity**: `test_decay_calculation` tests Ebbinghaus exponential forgetting; `test_conflict_detector` verifies polarity opposition detection (`always` vs `never`).
- **Pre-Read Guard**: `test_read_guard_suppression`, `test_read_guard_multi_cycle_suppression` test LRU content hash suppression.

### B. `oxide-db` Test Suite (2 Tests)
- **Typed Semantic Memory CRUD**: Tests storing, recalling, updating, and querying 13 semantic memory kinds in SurrealDB.
- **SurrealDB Traversal & GraphRAG**: Verifies multi-hop 2-hop BFS symbol graph queries using set-based `IN $targets` batch queries.

### C. `oxide-ml` Test Suite (7 Tests)
- **Maximal Marginal Relevance (MMR)**: Tests diversity reranking balancing query relevance against redundant candidates.
- **Batched Embedding Parity**: Validates that batched embeddings produce identical numerical vectors to single-item forward passes.
- **Local Model Embedders**: Verifies Candle BERT (384d), Candle Qwen (1024d), and ONNX Gemma (768d) engines.

### D. `oxide-parser` Test Suite (15 Tests)
- **Multi-Language AST Extractors**: Validates Tree-sitter parsers for Rust, TypeScript, Python, C, C++, OpenSCAD, JSON, SystemVerilog, Markdown, CSS, HTML, and YAML.
- **Surgical AST Slicing**: Validates exact line-range slicing of targeted functions and structs.
- **DocLinker (ECL Pipeline)**: Verifies bidirectional mapping of markdown doc sections to code symbols.

---

## 3. Verification Commands

```bash
# Run all workspace unit and integration tests
cargo test --workspace

# Run Criterion microbenchmarks
cargo bench --workspace

# Strict Clippy validation
cargo clippy --workspace --all-targets -- -D warnings

# Code formatting verification
cargo fmt --check
```
