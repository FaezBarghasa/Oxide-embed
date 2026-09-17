# Changelog

All notable changes to the **`Oxide-embed`** project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [0.2.0] - 2026-09-17

### Added

#### 1. Token Budgeting & Context Synthesis (`oxide-core`)
- **Knapsack Token-Budget Packer (`TokenBudgetPacker`)**: Greedy scoring and knapsack packing engine that fits highest-value AST symbols, subgraphs, and rules within exact token ceilings (`--budget <N>`).
- **Token Estimator (`TokenEstimator`)**: Sub-millisecond deterministic token estimation heuristic for code and markdown.
- **Task Context Synthesizer (`ContextSynthesizer`)**: Multi-source context engine synthesizing active `.oxide/STATUS.md` state, `.oxide/docs/CEREBRUM.md` architectural constraints, and 2-hop GraphRAG symbol subgraphs into a clean LLM prompt block.

#### 2. Surgical AST Slicing (`oxide-parser`)
- **Symbol Slicer (`SymbolSlicer`)**: Byte-range surgical slice extractor that retrieves only the exact function/struct implementation lines, signature, and docstring directly from source without loading entire files.
- CLI Integration: `oxide-embed read <path> --symbol <name>`.

#### 3. Native Model Context Protocol (MCP) Server (`oxide-cli`)
- **Stdio MCP Server (`McpServer`)**: Full JSON-RPC 2.0 stdio implementation exposing:
  - `oxide_context`: Task-driven context generator (`--budget`).
  - `oxide_read_symbol`: Surgical AST symbol read.
  - `oxide_search`: Semantic & graph hybrid search with token budget.
  - `oxide_explain`: Multi-hop GraphRAG symbol explainer.
  - `oxide_condense`: Terminal command execution with output condenser.
- Command: `oxide-embed mcp`.

#### 4. Live Debounced File Watcher (`oxide-cli`)
- **Workspace Watcher (`WorkspaceWatcher`)**: Real-time daemon based on `notify` that monitors code and markdown changes with a 300ms debounce window and incrementally updates SurrealDB symbol and chunk records in <5ms.
- Command: `oxide-embed watch`.

---

## [0.1.0] - 2026-09-17

### Added
- Core architecture (`oxide-core`): `SessionReadGuard`, `TerminalCondenser`, `HandoffCheckpoint`, `TokenLedger`, `MemifyEngine`.
- Tree-sitter AST extractors and Markdown DocLinker (`oxide-parser`).
- Pure Rust Candle embedder with offline deterministic projection fallback and K-Means clustering (`oxide-ml`).
- SurrealDB 3.x embedded SurrealKV store with SCHEMAFULL relations and GraphRAG multi-hop traversal (`oxide-db`).
- Unified CLI interface (`oxide-cli`): `init`, `index`, `cognify`, `search`, `explain`, `run`, `read`, `handoff`, `report`, `memify`, `consolidate`.
- 21 automated unit and integration tests and 4 Criterion benchmark suites.
