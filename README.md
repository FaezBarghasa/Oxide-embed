# Oxide-Embed ⚡

[![Rust](https://img.shields.io/badge/rust-2024%20Edition%20(1.85%2B)-orange.svg)](https://www.rust-lang.org/)
[![SurrealDB](https://img.shields.io/badge/SurrealDB-3.2%2B-red.svg)](https://surrealdb.com/)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)
[![Status](https://img.shields.io/badge/build-passing-brightgreen.svg)]()

**Oxide-Embed** is a high-performance, local-first code graph indexing, cognitive memory, and context hygiene engine engineered in pure Rust.

It synthesizes the architectural paradigms of **`topoteretes/cognee`** (hierarchical code-to-doc cognitive graphs, GraphRAG, temporal decay, and Cerebrum rule consolidation) and **`cytostack/openwolf`** (sub-millisecond AST pre-read guards, terminal error log condensing, and session handover checkpoints), executing **100% offline with zero cloud API dependencies and zero token waste**.

---

## 🏛️ Architectural Overview

```
                                    +-----------------------------------------+
                                    |         Oxide-Embed CLI Engine         |
                                    +-----------------------------------------+
                                           /             |             \
                                          /              |              \
                    +--------------------+   +-----------+-----------+   +--------------------+
                    |   oxide-parser     |   |       oxide-ml        |   |    oxide-core      |
                    | Tree-sitter AST &  |   | Candle Embeddings &   |   | Context Hygiene &  |
                    | DocLinker Engine   |   | Vector Clustering     |   | Memify Evolution   |
                    +--------------------+   +-----------------------+   +--------------------+
                                          \              |              /
                                           \             |             /
                                    +-----------------------------------------+
                                    |                oxide-db                 |
                                    |    SurrealDB 3.x Embedded KV Store     |
                                    |    (Vectors + SCHEMAFULL Graph RAG)     |
                                    +-----------------------------------------+
```

### Key Modules

- **`oxide-core`**: Core domain primitives, deterministic ID hashing (UUIDv7, SHA-256), `SessionReadGuard` duplicate read suppression, `TerminalCondenser` log caching, `HandoffCheckpoint` markdown generation, `TokenLedger` cost tracking, and `MemifyEngine` exponential graph decay & bug consolidation.
- **`oxide-parser`**: Multi-language Tree-sitter extractors (Rust, TypeScript, Python, Go, Java, Bash, etc.), `AnatomyScanner` sub-millisecond AST maps, `OutlineGenerator`, windowed `Chunker`, and `DocLinker` markdown documentation-to-symbol linkers.
- **`oxide-ml`**: Local vector embeddings powered by Hugging Face Candle (`bge-small-en-v1.5` / deterministic 384-d normalized projection fallback), cosine similarity metrics, and K-Means vector clustering.
- **`oxide-db`**: Embedded SurrealDB 3.x (`SurrealKV` local engine) with SCHEMAFULL relational tables (`symbol`, `file`, `chunk`, `calls`, `contains`, `imports`, `doc_section`, `doc_reference`, `cerebrum_rule`, `buglog`), vector cosine search, and multi-hop `GraphTraversalService`.
- **`oxide-cli`**: Unified CLI binary exposing indexers, search, GraphRAG explanation, terminal runners, and memory consolidation.

---

## 🚀 Key Features

### 1. Cognitive Graph & GraphRAG (`Cognee` Parity)
- **Deterministic AST Extraction**: Extracts symbols, signatures, call graphs, import dependencies, and parent-child hierarchies across polyglot workspaces.
- **DocLinker (ECL Pipeline)**: Hierarchically links markdown documentation sections to concrete code symbols without external LLM calls.
- **Multi-Hop Subgraph Traversal**: Queries symbols, caller/callee chains, and associated architectural docs in a single bounded graph traversal.
- **Active Forgetting & Temporal Decay**: Exponentially decays unreferenced graph edges and auto-prunes orphan nodes.
- **Cerebrum Consolidation**: Clusters resolved bug logs and synthesizes actionable project rules into `.oxide/docs/CEREBRUM.md`.

### 2. Context Hygiene & Token Reduction (`OpenWolf` Parity)
- **Pre-Read Guard**: Intercepts file reads across agent sessions. If content hash is unchanged, returns a lightweight AST symbol outline stub instead of dumping thousands of tokens.
- **Terminal Condenser**: Intercepts CLI output. Caches full raw output into `.oxide/cache/bash/` and presents only essential compiler error blocks to the agent.
- **Session Handover Checkpoints**: Generates atomic `.oxide/STATUS.md` state checkpoints (active branch, files modified, pending errors, next steps) for seamless multi-agent workflows.
- **Local Token Ledger**: Measures input, output, cached, and reasoning tokens with estimated cost breakdowns and savings scoreboards.

---

## 📦 Installation & Build

### Prerequisites
- **Rust**: 1.85+ (Rust 2024 Edition)
- **Cargo**: Resolver v3

### Building from Source
```bash
git clone https://github.com/FaezBarghasa/Oxide-embed.git
cd Oxide-embed

# Build debug binary
cargo build

# Build optimized release binary
cargo build --release
```

---

## 🛠️ CLI Usage & Workflows

### 1. Workspace Initialization & Indexing
```bash
# Initialize .oxide metadata and embedded SurrealKV store
oxide-embed init

# Run full AST extraction, DocLinker, and Vector indexer
oxide-embed index

# Cognify workspace (Full Cognee-style GraphRAG index)
oxide-embed cognify
```

### 2. Search & Subgraph GraphRAG Traversal
```bash
# Hybrid semantic vector + text search
oxide-embed search "init_hardware"

# Search with multi-hop graph context
oxide-embed search "init_hardware" --with-graph --hops 2

# Explain a symbol and its relationship topology
oxide-embed explain "init_hardware" --hops 2
```

### 3. Context Hygiene & Execution
```bash
# Run command with automatic log caching & error condensing
oxide-embed run -- cargo test

# Read file with Pre-Read Guard (suppresses unchanged duplicate reads)
oxide-embed read src/main.rs
```

### 4. Session Handover & Token Accounting
```bash
# Save atomic handover checkpoint
oxide-embed handoff --goal "STM32 I2C Driver" --next "Run HIL tests"

# Display token efficiency & savings metrics
oxide-embed report
```

### 5. Memory Evolution & Active Forgetting
```bash
# Apply temporal decay and prune dead edges
oxide-embed memify

# Consolidate bug logs into CEREBRUM rules
oxide-embed consolidate
```

---

## 🧪 Testing & Benchmarks

### Running Tests
```bash
# Run all workspace unit and integration tests (21 tests)
cargo test --workspace
```

### Running Criterion Benchmarks
```bash
# Compile and verify all benchmark suites
cargo bench --workspace --no-run

# Run full Criterion benchmarks
cargo bench --workspace
```

Benchmark targets:
- `core_bench`: UUIDv7, SHA-256 throughput, Read Guard lookup, Terminal Condenser, Memify decay.
- `parser_bench`: Tree-sitter AST extraction (Rust, TS, Python), outline generation, doc linking.
- `embedder_bench`: 384-d vector projections, cosine similarity SIMD, K-Means clustering.
- `db_bench`: SurrealDB symbol upsert, vector similarity search, GraphRAG subgraph queries.

---

## 📄 License

Licensed under the Apache License, Version 2.0. See [LICENSE](LICENSE) for details.
