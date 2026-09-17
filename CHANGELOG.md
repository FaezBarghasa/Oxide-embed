# Changelog

All notable changes to the **`Oxide-embed`** project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [0.2.0] - 2026-09-17

### Added

#### 1. Multi-Distro Linux Packaging & Automated Installation
- **Universal Installer Script (`scripts/install.sh`)**: Architecture detection (x86_64, aarch64), automated binary placement in `/usr/local/bin` or `~/.local/bin`, shell completion setup, and optional systemd user service activation.
- **Native Debian Package Builder (`scripts/package-deb.sh`)**: Produces stripped, dependency-resolved `dist/oxide-embed_0.2.0_amd64.deb` using `dpkg-deb` with full control metadata, pre/post installation triggers, and copyright notices.
- **Arch Linux Package (`packaging/arch/PKGBUILD`)**: Native `makepkg` recipe compiling with release optimizations and systemd user unit deployment.
- **Fedora / RHEL RPM Specification (`packaging/fedora/oxide-embed.spec`)**: Standard RPM build spec with systemd-rpm-macros.
- **Alpine Linux Recipe (`packaging/alpine/APKBUILD`)**: Musl libc package recipe for ultra-lightweight containers.
- **Systemd User Service (`packaging/systemd/oxide-watch.service`)**: Background daemon unit managing continuous file watching and incremental AST re-indexing.
- **Master Release Builder (`scripts/build-dist.sh`)**: Orchestrates compilation, Debian packaging, portable tarball packaging, and `SHA256SUMS` generation.

#### 2. Real Criterion Benchmark Suite & Performance Scorecards
- **19 Benchmark Suites Across All 5 Crates (`benches/`)**:
  - `core_id_and_hashing`: UUIDv7 generation (1.51 µs), Blake3/SHA256 path hashing (325.58 ns).
  - `core_context_hygiene`: Read guard cache lookup (4.19 µs), error condenser (45.73 µs).
  - `core_ledger_and_memify`: Token ledger calculation (1.62 µs), Ebbinghaus memory decay (4.64 ns).
  - `parser_ast_extraction`: Tree-sitter Rust (46.82 µs), TypeScript (41.78 µs), Python (31.06 µs).
  - `parser_outline_and_chunking`: Outline extraction (321.37 ns), symbol chunking (6.43 µs).
  - `parser_doc_and_anatomy`: Doc anatomy (412.66 ns), doc-to-symbol linking (598.55 ns).
  - `ml_candle_embedder`: 384d SIMD projection (638.41 ns), cosine similarity (611.66 ns), K-Means (752.35 µs).
  - `db_surreal_operations`: Hybrid BM25/Vector search (149.06 µs), 2-Hop GraphRAG traversal (530.43 µs).
- **Benchmark Documentation (`docs/BENCHMARKS.md`)**: Full report with confidence intervals and direct comparisons against Cognee, OpenWolf, and Tokenix.

#### 3. Token Budgeting & Context Synthesis (`oxide-core`)
- **Knapsack Token-Budget Packer (`TokenBudgetPacker`)**: Greedy scoring and knapsack packing engine that fits highest-value AST symbols, subgraphs, and rules within exact token ceilings (`--budget <N>`).
- **Token Estimator (`TokenEstimator`)**: Sub-millisecond deterministic token estimation heuristic for code and markdown.
- **Task Context Synthesizer (`ContextSynthesizer`)**: Multi-source context engine synthesizing active `.oxide/STATUS.md` state, `.oxide/docs/CEREBRUM.md` architectural constraints, and 2-hop GraphRAG symbol subgraphs into a clean LLM prompt block.

#### 4. Surgical AST Slicing (`oxide-parser`)
- **Symbol Slicer (`SymbolSlicer`)**: Byte-range surgical slice extractor that retrieves only the exact function/struct implementation lines, signature, and docstring directly from source without loading entire files.
- CLI Integration: `oxide-embed read <path> --symbol <name>`.

#### 5. Native Model Context Protocol (MCP) Server (`oxide-cli`)
- **Stdio MCP Server (`McpServer`)**: Full JSON-RPC 2.0 stdio implementation exposing 11 specialized tools:
  - `oxide_index_workspace`, `oxide_search_hybrid`, `oxide_query_graph`, `oxide_extract_ast`, `oxide_get_outline`, `oxide_link_docs`, `oxide_read_guard`, `oxide_condense_errors`, `oxide_token_ledger`, `oxide_check_drift`, `oxide_mcp_status`.
- **MCP Client Guide (`docs/MCP_GUIDE.md`)**: Complete setup configurations for Google Antigravity, Claude Code, Cursor, and Roo Code.

#### 6. Live Debounced File Watcher (`oxide-cli`)
- **Workspace Watcher (`WorkspaceWatcher`)**: Real-time daemon based on `notify` that monitors code and markdown changes with a 300ms debounce window and incrementally updates SurrealDB symbol and chunk records in <5ms.
- Command: `oxide-embed watch`.

#### 7. Visual Assets & Documentation
- `docs/assets/oxide_architecture_overview.jpg` & `assets/oxide_architecture_overview.jpg`: Comprehensive architectural diagram.
- `docs/assets/oxide_cli_graphrag_demo.jpg` & `assets/oxide_cli_graphrag_demo.jpg`: Live CLI terminal and GraphRAG demo banner.
- Upgraded `README.md`, `docs/ARCHITECTURE.md`, `docs/INSTALL.md`, `docs/BENCHMARKS.md`, and `docs/MCP_GUIDE.md`.

---

## [0.1.0] - 2026-09-17

### Added
- Core architecture (`oxide-core`): `SessionReadGuard`, `TerminalCondenser`, `HandoffCheckpoint`, `TokenLedger`, `MemifyEngine`.
- Tree-sitter AST extractors and Markdown DocLinker (`oxide-parser`).
- Pure Rust Candle embedder with offline deterministic projection fallback and K-Means clustering (`oxide-ml`).
- SurrealDB 3.x embedded SurrealKV store with SCHEMAFULL relations and GraphRAG multi-hop traversal (`oxide-db`).
- Unified CLI interface (`oxide-cli`): `init`, `index`, `cognify`, `search`, `explain`, `run`, `read`, `handoff`, `report`, `memify`, `consolidate`.
- 21 automated unit and integration tests and 4 Criterion benchmark suites.
