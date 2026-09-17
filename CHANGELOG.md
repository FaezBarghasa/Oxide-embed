# Changelog

All notable changes to the **`Oxide-embed`** project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [0.1.0] - 2026-09-17

### Added

#### 1. Core Architecture (`oxide-core`)
- **Deterministic ID Derivation**: UUIDv7 for project-level tracking and SHA-256 content hashes for deterministic `FileId`, `SymbolId`, and `ChunkId`.
- **Pre-Read Guard (`SessionReadGuard`)**: In-memory session tracker that eliminates redundant full-file reads for unchanged code and returns lightweight AST outline stubs.
- **Terminal Condenser (`TerminalCondenser`)**: Command output hygiene engine that caches full raw logs in `.oxide/cache/bash/` while presenting compressed, actionable error windows to AI agents.
- **Session Handover (`HandoffCheckpoint`)**: Automated generation and parsing of `.oxide/STATUS.md` capturing objective, modified files, active errors, and next steps.
- **Token Accounting (`TokenLedger`)**: Accurate ledger tracking input, output, cached, and reasoning tokens with estimated cost savings breakdown.
- **Memory Evolution (`MemifyEngine`)**: Temporal graph decay calculations (`calculate_decayed_weight`), orphan node garbage collection, and automated synthesis of resolved buglogs into `.oxide/docs/CEREBRUM.md`.

#### 2. AST Parsing & Doc Linking (`oxide-parser`)
- **Multi-Language Tree-sitter Extractors**: Zero-allocation symbol extractors for Rust, TypeScript, JavaScript, Python, Go, Java, Bash, HTML, CSS, JSON, YAML, and Markdown.
- **Anatomy Scanner (`AnatomyScanner`)**: Sub-millisecond workspace anatomy scanner producing hierarchical AST symbol maps.
- **Outline Generator (`OutlineGenerator`)**: Compact textual outline generation for rapid contextual orientation.
- **Chunking Engine (`Chunker`)**: Windowed symbol and outline chunkers with configurable overlap and boundary preservation.
- **ECL DocLinker (`DocLinker`)**: Deterministic Markdown section extractor and AST symbol linker establishing `doc_reference` graph edges without LLM dependencies.

#### 3. Machine Learning & Embeddings (`oxide-ml`)
- **Local Candle Embedder (`CandleBertEmbedder`)**: Pure Rust BERT-based text embedding with offline deterministic 384-dimensional normalized projection fallback.
- **Vector Math**: Optimized cosine similarity matrix computation and multi-dimensional dot products.
- **Vector Clustering**: K-Means clustering algorithm for automated grouping of related incidents and semantic code regions.
- **Mock Embedder**: Fast, deterministic mock embedder harness for integration and property tests.

#### 4. Embedded Database & GraphRAG (`oxide-db`)
- **SurrealDB 3.x Store (`SurrealProjectStore`)**: Embedded `SurrealKV` local key-value engine with full CRUD operations for files, symbols, chunks, and doc sections.
- **SCHEMAFULL Graph Tables**: Relations for `calls`, `contains`, `imports`, `doc_reference`, `cerebrum_rule`, and `buglog`.
- **Graph Traversal Service (`GraphTraversalService`)**: Multi-hop GraphRAG context retrieval assembling target symbols, signatures, caller chains, callee chains, and associated markdown doc sections.
- **Database Migrations (`MigrationManager`)**: Versioned schema migration runner for forward and backward compatibility.

#### 5. Command-Line Interface (`oxide-cli`)
- **Workspace Commands**:
  - `oxide-embed init`: Scaffolds `.oxide` configuration, cache, and database directories.
  - `oxide-embed index` / `oxide-embed cognify`: Executes full-workspace AST extraction, doc linking, and vector index generation.
  - `oxide-embed search`: Hybrid semantic and lexical code search with optional `--with-graph` multi-hop traversal.
  - `oxide-embed explain`: Deep multi-hop GraphRAG topology explainer for specific symbols.
  - `oxide-embed run`: Execution hygiene wrapper with automatic log caching and compiler error truncation.
  - `oxide-embed read`: Pre-Read Guard interceptor suppressing redundant file content dumps.
  - `oxide-embed handoff`: Atomic session handover state generator.
  - `oxide-embed report`: Token accounting report and efficiency metrics scoreboard.
  - `oxide-embed memify`: Temporal graph decay and dead edge pruner.
  - `oxide-embed consolidate`: Bug log clustering and Cerebrum rule consolidator.

#### 6. Test & Benchmark Infrastructure
- **Comprehensive Workspace Test Suite**: 21 unit and integration tests passing across all crates (`oxide-core`, `oxide-parser`, `oxide-ml`, `oxide-db`).
- **Criterion Benchmark Suite**:
  - `core_bench`: UUIDv7 generation, SHA-256 throughput, Read Guard lookup, Terminal Condenser throughput, and Memify decay calculation.
  - `parser_bench`: Tree-sitter extraction across Rust/TS/Python, symbol outline rendering, file chunking, and doc linking.
  - `embedder_bench`: 384-d vector projections, cosine similarity computation, and K-Means clustering.
  - `db_bench`: SurrealDB symbol upsert, vector similarity search, and GraphRAG subgraph queries.
