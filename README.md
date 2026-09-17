# Oxide-Embed ⚡

[![Rust](https://img.shields.io/badge/rust-2024%20Edition%20(1.85%2B)-orange.svg)](https://www.rust-lang.org/)
[![SurrealDB](https://img.shields.io/badge/SurrealDB-3.2%2B-red.svg)](https://surrealdb.com/)
[![MCP](https://img.shields.io/badge/protocol-MCP%20Stdio-purple.svg)](https://modelcontextprotocol.io/)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)
[![Status](https://img.shields.io/badge/build-passing-brightgreen.svg)]()

**Oxide-Embed** is a high-performance, local-first code graph indexing, cognitive memory, and context hygiene engine engineered in pure Rust.

It synthesizes the architectural paradigms of **`topoteretes/cognee`** (hierarchical code-to-doc cognitive graphs, GraphRAG, temporal decay, and Cerebrum rule consolidation), **`cytostack/openwolf`** (sub-millisecond AST pre-read guards, terminal error log condensing, and session handover checkpoints), and **`tokenix`** (knapsack token budgeting, surgical AST slicing, task-driven context synthesis, and native Model Context Protocol server), executing **100% offline with zero cloud API dependencies and zero token waste**.

---

## 🏛️ Architectural Overview

```
                                    +-----------------------------------------+
                                    |         Oxide-Embed CLI Engine         |
                                    |  (CLI Commands + Native MCP Stdio Serv) |
                                    +-----------------------------------------+
                                           /             |             \
                                          /              |              \
                    +--------------------+   +-----------+-----------+   +--------------------+
                    |   oxide-parser     |   |       oxide-ml        |   |    oxide-core      |
                    | Tree-sitter AST,   |   | Candle Embeddings &   |   | Context Hygiene,   |
                    | Slicer & DocLinker |   | Vector Clustering     |   | Knapsack Budget &  |
                    |                    |   |                       |   | Memify Evolution   |
                    +--------------------+   +-----------------------+   +--------------------+
                                          \              |              /
                                           \             |             /
                                    +-----------------------------------------+
                                    |                oxide-db                 |
                                    |    SurrealDB 3.x Embedded KV Store     |
                                    |    (Vectors + SCHEMAFULL Graph RAG)     |
                                    +-----------------------------------------+
```

---

## 🚀 Key Features

### 1. Cognitive Graph & GraphRAG (`Cognee` Parity)
- **Deterministic AST Extraction**: Extracts symbols, signatures, call graphs, import dependencies, and parent-child hierarchies across polyglot workspaces.
- **DocLinker (ECL Pipeline)**: Hierarchically links markdown documentation sections to concrete code symbols without external LLM calls.
- **Multi-Hop Subgraph Traversal**: Queries symbols, caller/callee chains, and associated architectural docs in a single bounded graph traversal.
- **Active Forgetting & Temporal Decay**: Exponentially decays unreferenced graph edges and auto-prunes orphan nodes.
- **Cerebrum Consolidation**: Clusters resolved bug logs and synthesizes actionable project rules into `.oxide/docs/CEREBRUM.md`.

### 2. Context Hygiene & Token Reduction (`OpenWolf` & `Tokenix` Parity)
- **Knapsack Token-Budget Packing**: Greedy budget packer (`--budget <N>`) that fits highest-value symbols, graph subgraphs, and docs strictly within token ceilings.
- **Surgical AST Symbol Reading**: Slices and streams exclusively the target function/struct's source lines, signature, and doc comments directly from the AST (`read --symbol <name>`).
- **Pre-Read Guard**: Intercepts file reads across agent sessions. If content hash is unchanged, returns a lightweight AST symbol outline stub instead of dumping thousands of tokens.
- **Terminal Condenser**: Intercepts CLI output. Caches full raw output into `.oxide/cache/bash/` and presents only essential compiler error blocks to the agent.
- **Session Handover Checkpoints**: Generates atomic `.oxide/STATUS.md` state checkpoints for seamless multi-agent handovers.
- **Local Token Ledger**: Measures input, output, cached, and reasoning tokens with estimated cost breakdowns and savings scoreboards.

### 3. Agent Integration & Live Automation
- **Native Model Context Protocol (MCP)**: Runs as an MCP server (`oxide-embed mcp`) over stdio for direct integration into Antigravity, Claude Code, Cursor, and Roo Code.
- **Live Debounced Watcher Daemon**: Real-time file system monitor (`oxide-embed watch`) that incrementally re-indexes AST symbols and call edges on file save in <5ms.

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

# Run live watcher daemon for incremental sub-millisecond re-indexing
oxide-embed watch
```

### 2. Task-Driven Context & Surgical Slicing
```bash
# Synthesize multi-layer context for a prompt within an exact token budget
oxide-embed context "implement bare-metal SPI driver" --budget 1500

# Surgically read only a specific AST symbol definition and docstring
oxide-embed read crates/oxide-core/src/id.rs --symbol ProjectId

# Search with token budget ceiling
oxide-embed search "init_hardware" --budget 1000 --with-graph
```

### 3. Subgraph GraphRAG Traversal
```bash
# Explain a symbol and its multi-hop relationship topology
oxide-embed explain "init_hardware" --hops 2
```

### 4. Context Hygiene & Execution
```bash
# Run command with automatic log caching & error condensing
oxide-embed run -- cargo test

# Read file with Pre-Read Guard (suppresses unchanged duplicate reads)
oxide-embed read src/main.rs
```

### 5. Session Handover & Memory Evolution
```bash
# Save atomic handover checkpoint
oxide-embed handoff --goal "STM32 I2C Driver" --next "Run HIL tests"

# Display token efficiency & savings metrics
oxide-embed report

# Apply temporal decay and prune dead edges
oxide-embed memify

# Consolidate bug logs into CEREBRUM rules
oxide-embed consolidate
```

### 6. Model Context Protocol (MCP) Server
```bash
# Start stdio MCP server for agent IDEs
oxide-embed mcp
```

---

## 🧪 Testing & Benchmarks

```bash
# Run all workspace unit and integration tests (26 tests)
cargo test --workspace

# Compile and verify all Criterion benchmark suites
cargo bench --workspace --no-run
```

---

## 📄 License

Licensed under the Apache License, Version 2.0. See [LICENSE](LICENSE) for details.
