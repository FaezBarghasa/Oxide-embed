# Oxide-embed System Design Brief

**System**: Oxide-embed  
**Target Domain**: High-Throughput Local Codebase GraphRAG & Context Engine  
**Author**: Faez Barghasa  

---

## 1. Problem Statement

Current AI agent architectures struggle with large, complex repositories:
1. **Context Window Saturation**: Passing whole files wastes thousands of tokens on boilerplate, exceeding model context budgets.
2. **Loss of Structural Topology**: Traditional vector databases treat code as flat natural language strings, losing caller/callee relationships, trait implementations, and macro/leaf boundaries.
3. **Cross-Project Blindness & Duplication**: When agents work across multiple crates in a workspace, they cannot discover existing reusable crates and end up writing duplicate types and drivers.
4. **Cloud API Dependency**: Most commercial context tools depend on remote SaaS endpoints, creating latency bottlenecks and privacy risks for air-gapped industrial codebases.

---

## 2. Core Architectural Solutions

### A. Dual-Tier Cognitive Memory (Hindsight Parity)
- **Tier 1 (Project Context)**: Local AST syntax trees, Code-ToC chunks, 2-hop caller/callee graph edges, docstring linkages, and git commit history specific to the active repository.
- **Tier 2 (Agent Metacognitive Intelligence)**: Reusable procedural skills (AIP), standing mental models with instant cached recall, consolidated Cerebrum rules, and workspace crate catalog across `~/RustroverProjects/` for zero code duplication.

### B. Unified SurrealDB 3.2+ Multi-Model Storage
Instead of managing separate databases for vectors, graphs, and key-value records:
- **Embedded SurrealKV**: Embedded, in-process, zero-network-hop storage engine.
- **Batched Ingestion**: 50 chunks per transaction block, achieving a 50x reduction in fsync disk commits.
- **Relational Graph Modeling**: `CALLS`, `DEFINED_IN`, `IMPORTS`, `DOCUMENTS`, and `GOVERNS` relations traversed in sub-millisecond batch queries using `IN $targets`.
- **B-Tree Indexing**: Instant sub-millisecond filtering on `chunk`, `symbol`, and `calls` tables.

### C. Structure-Aware Information Retrieval (STAIR Code-ToC)
- Breaks source files into macro blocks (traits, classes, impl blocks) and leaf nodes (methods, functions).
- Emits structural breadcrumbs (e.g. `[impl ProjectStore > upsert_chunks_batch]`) that allow agents to pinpoint exact target symbols without semantic bleeding.

### D. Zero-Blocking Async Concurrency (Tokio)
- All blocking operations (recursive filesystem walks, git process execution) are offloaded to `tokio::task::spawn_blocking` and `tokio::process::Command`.
- The MCP server decouples the stdio reader loop from tool execution, processing concurrent requests while streaming atomic responses via an MPSC channel.

---

## 3. Measurable Impact & Token Savings

| Metric | Flat File Dumping | Oxide-Embed AST Engine | Efficiency Gain |
| :--- | :--- | :--- | :--- |
| **Typical Context Cost** | 4,500 - 12,000 tokens | **350 - 1,200 tokens** | **~80% - 90% reduction** |
| **Query Latency** | 200 - 800 ms (API call) | **< 1 ms (Local SurrealKV)** | **> 200x faster** |
| **Crate Reuse** | Low (frequent reinvention) | **High (automated crate catalog)** | **Zero duplicate drivers** |
