# Oxide-embed Product Requirements Document (PRD)

**Version**: 0.6.0  
**Status**: Implemented & Production-Hardened  
**Target Platform**: Linux, macOS, Windows  

---

## 1. Executive Summary & Vision

Modern Large Language Models (LLMs) used in automated software development suffer from three crippling bottlenecks:
1. **Token Exhaustion & Bleeding**: Full-file context dumping consumes context windows and floods prompts with irrelevant boilerplate.
2. **Flat Semantic Retrieval**: Generic vector search lacks awareness of syntax boundaries, class-method hierarchies, and call chains.
3. **Agent Amnesia & Duplication**: Agents lack durable long-term memory across sessions and repeatedly rewrite utilities that already exist in sibling workspace crates.

**Oxide-embed** is an offline-first, token-lean AST context engine, GraphRAG reasoning server, and Model Context Protocol (MCP) daemon engineered in pure Rust (2024 Edition) to solve these limitations with sub-millisecond query performance.

---

## 2. System Personas & Use Cases

- **Autonomous AI Agents (Antigravity, Claude Code, Cursor)**: Consumes codebase structure, reads exact symbol slices, recalls architectural decisions, and traces callers/callees via MCP tools.
- **Embedded & Systems Engineers**: Enforces strict `no_std` constraints, low-overhead workflows, and zero-cloud air-gapped development.
- **Multi-Crate Workspace Developers**: Discovers and reuses existing types and drivers across complex nested monorepos without reinventing crates.

---

## 3. Functional Requirements

### FR-1: Syntax-Aware AST Parsing & Slicing
- Must parse Rust, TypeScript, Python, Go, C, and C++ using native Tree-sitter bindings.
- Must extract symbol definitions, type signatures, docstrings, method bodies, and import relationships.
- Must support surgical symbol reading (`oxide-embed read <path> --symbol <name>`) returning only the exact symbol lines.

### FR-2: STAIR Code-ToC Hierarchical Retrieval
- Must index codebases into bounded macro nodes (traits, classes, impl blocks) and leaf nodes (functions, methods).
- Must return structural breadcrumbs (e.g. `[impl ProjectStore > upsert_chunks_batch]`) with zero semantic bleeding.

### FR-3: Typed Semantic Memory & Conflict Resolution
- Must support 13 distinct memory categories (`decision`, `preference`, `instruction`, `fact`, `goal`, etc.).
- Must detect semantic polarity contradictions (`never` vs `always`, `use` vs `avoid`) and auto-supersede conflicting assertions when requested.
- Must support point-in-time temporal queries (`--as-of <ISO8601>`).

### FR-4: Dual-Tier Cognitive Memory Fabric
- **Tier 1 (Project Context)**: Code-ToC chunks, AST symbols, call edges, file content hashes, and git distillation records.
- **Tier 2 (Agent Intelligence)**: Standing mental models, procedural AIP skills, consolidated Cerebrum rules, and workspace crate catalog (`~/RustroverProjects/`).

### FR-5: Incremental Delta Synchronization (`update`)
- Must detect modified, added, and removed files using SHA-256 content hashes.
- Must delete obsolete artifacts and mark affected standing mental models stale without rebuilding the entire database.
- Must scan sibling workspaces for reusable crates to prevent code duplication.

### FR-6: High-Performance Embedded SurrealDB Ingestion
- Must use in-process SurrealDB with local key-value storage (`SurrealKV`).
- Must batch chunk writes into transactions (50 chunks per transaction) to minimize fsync disk commits.
- Relational tables (`chunk`, `symbol`, `calls`) must be backed by composite and B-tree indexes.

### FR-7: Asynchronous Model Context Protocol (MCP) Server
- Must expose 19 tools over stdio compliant with the MCP specification.
- Must execute incoming tool calls concurrently on Tokio worker tasks without stalling the stdio reader loop.
- Must serialize stdout responses atomically through an MPSC channel.

---

## 4. Non-Functional Requirements & Performance SLOs

| Dimension | Target Metric | Verified Actual |
| :--- | :--- | :--- |
| **Rust AST Extraction** | < 100 µs | **46.82 µs** |
| **Python AST Extraction** | < 100 µs | **31.06 µs** |
| **GraphRAG 2-Hop Traversal** | < 1.0 ms | **0.53 ms (530 µs)** |
| **Vector Cosine Similarity (384d)** | < 1.0 µs | **611.66 ns** |
| **Memory Decay Calculation** | < 10.0 ns | **4.64 ns** |
| **Pre-Read Guard Lookup** | < 10.0 µs | **4.19 µs** |
| **Idle Memory Footprint** | < 25 MB RAM | **< 14 MB RAM** |
| **Offline Guarantee** | 100% Local | **Zero external network requests** |
