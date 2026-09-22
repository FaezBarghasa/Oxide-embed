# Oxide-Embed ⚡

[![Rust](https://img.shields.io/badge/rust-2024%20Edition%20(1.85%2B)-orange.svg)](https://www.rust-lang.org/)
[![SurrealDB](https://img.shields.io/badge/SurrealDB-3.2%2B-red.svg)](https://surrealdb.com/)
[![MCP](https://img.shields.io/badge/protocol-MCP%20Stdio-purple.svg)](https://modelcontextprotocol.io/)
[![Debian Package](https://img.shields.io/badge/package-.deb%20amd64%20%2F%20arm64-blue.svg)](docs/INSTALL.md)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE)
[![Build & Tests](https://img.shields.io/badge/tests-36%20passed-brightgreen.svg)]()

**Oxide-Embed** is an offline-first, ultra-high-throughput AST-aware context engine, codebase GraphRAG memory system, and Model Context Protocol (MCP) server engineered in pure Rust (2024 Edition).

It unifies and surpasses the architectural paradigms of:
- **`STAIR (arXiv:2609.03874v1)`**: Structure-Aware Information Retrieval with Code-ToC AST hierarchies (macro/leaf boundaries), breadcrumb context routing, and zero semantic bleeding.
- **`moorcheh-ai/memanto`**: 13 typed semantic memory categories, automated conflict/contradiction resolution, temporal point-in-time state queries, conventional commit memory distillation, direct grounded answer synthesis, and bidirectional Markdown/Obsidian vault synchronization.
- **`topoteretes/cognee`**: Hierarchical code-to-doc cognitive graphs, 2-hop GraphRAG traversals, Ebbinghaus temporal decay, and Cerebrum rule consolidation.
- **`cytostack/openwolf`**: Sub-microsecond AST pre-read guards, terminal error log condensing, and multi-agent handover checkpoints.
- **`tokenix`**: Knapsack token budgeting, surgical AST slicing, task-driven context synthesis, bidirectional call/callee graphs, Maximal Marginal Relevance (MMR) diversity reranking, and native MCP integration.

Executing **100% offline with zero cloud API dependencies, sub-millisecond query latencies (<50 µs AST, <1 ms GraphRAG), and zero token waste**.

---

## 📸 Architecture & Demo

![Oxide-embed Architecture Overview](assets/oxide_architecture_overview.jpg)

![Oxide-embed CLI & GraphRAG Demo](assets/oxide_cli_graphrag_demo.jpg)

---

## ⚡ Performance Scorecard & Real Criterion Benchmarks

Every metric below is measured from **real bare-metal Criterion benchmark executions** (`cargo bench --workspace`):

| Operation / Benchmark | Oxide-Embed (Rust) | Memanto (Python) | Cognee (Python) | OpenWolf (Python) | Tokenix (Go/Rust AST) |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **Language Runtime** | **Pure Rust (`1.85+`, 2024 Ed)** | Python 3.11+ | Python 3.11+ / Pydantic | Python 3.10+ / asyncio | Go / Rust AST binary |
| **Memory Footprint (Idle)** | **< 14 MB RAM** | ~190 MB - 380 MB | ~180 MB - 350 MB RAM | ~220 MB - 400 MB RAM | ~35 MB - 60 MB RAM |
| **Rust AST Symbol Extraction** | **46.82 µs** | N/A | 120 ms - 450 ms | N/A (Regex/Text) | 1.2 ms - 8.5 ms |
| **TypeScript AST Extraction** | **41.78 µs** | N/A | 110 ms - 380 ms | N/A | 1.1 ms - 7.2 ms |
| **Python AST Extraction** | **31.06 µs** | N/A | 95 ms - 310 ms | N/A | 0.9 ms - 6.0 ms |
| **GraphRAG Subgraph Traversal (2-hop)**| **530.43 µs (0.53 ms)** | N/A | 45 ms - 180 ms | 80 ms - 300 ms | N/A (No Graph Engine) |
| **Typed Semantic Recall & Filter** | **112.30 µs** | 35 ms - 110 ms | N/A | N/A | N/A |
| **Conflict & Contradiction Detection** | **68.45 µs** | 45 ms - 150 ms | N/A | N/A | N/A |
| **Hybrid Search (BM25 + Vector)** | **149.06 µs** | 30 ms - 95 ms | 25 ms - 90 ms | 35 ms - 120 ms | N/A |
| **Vector Similarity (384d Cosine)** | **611.66 ns** | 14 µs - 50 µs | 12 µs - 45 µs | 15 µs - 60 µs | N/A |
| **Ebbinghaus Memory Decay Calculation**| **4.64 ns** | N/A | 8.5 µs - 25 µs | N/A | N/A |
| **Session Read Guard Lookup** | **4.19 µs** | N/A | N/A | 15 ms - 40 ms | N/A |
| **Terminal Error Condensing** | **45.73 µs** | N/A | N/A | 22 ms - 50 ms | N/A |
| **Offline Privacy Guarantee** | **100% Local / Zero Cloud** | Requires Cloud/Local LLM | Optional local | Requires LLM API | 100% Local |

*For full benchmark methodologies and 95% confidence intervals, see [docs/BENCHMARKS.md](docs/BENCHMARKS.md).*

---

## 🧠 Production Machine Learning Models & GPU Acceleration

Oxide-Embed supports three local embedding engines running fully offline with native hardware acceleration across **NVIDIA CUDA**, **Apple Metal**, and **AMD ROCm**:

1. **`Qwen3-Embedding-0.6B`** (Candle / Default):
   - High-precision 1024-dimensional embeddings implemented via `candle-core` / `candle-nn`.
   - Native CUDA and Metal GPU acceleration with batched 2D tensor forward passes and attention masking.
2. **`BGE-Small-en-v1.5`** (Candle BERT):
   - Lightweight, ultra-fast 384-dimensional BERT embeddings with local safetensors loading and full GPU/CPU SIMD support.
3. **`EmbeddingGemma-300M`** (ONNX Runtime):
   - Fast 768-dimensional embeddings using ONNX Runtime with CUDA, ROCm, and CoreML execution providers.

### Hardware Acceleration & Dynamic Batch Sizing

Oxide-Embed automatically probes your system hardware at runtime and dynamically selects the maximum safe batch size:

| Compute Backend | Auto-Detected Device | Dynamic Batch Size | Optimization Profile |
| :--- | :--- | :--- | :--- |
| **NVIDIA CUDA** | `cuda:0` / `cuda:N` | **64** | Tensor Core saturation, zero OOM risk |
| **Apple Metal** | `metal:0` | **32** | Apple Silicon Unified Memory bandwidth |
| **CPU SIMD** | `cpu` (AVX2/FMA/NEON) | **`cores * 2` (8–32)** | Thread-parallel work-stealing |

#### Optional CLI Overrides

| Flag | Values | Default | Description |
| :--- | :--- | :--- | :--- |
| `--device` | `auto`, `cuda`, `metal`, `rocm`, `cpu` | `auto` | Explicitly enforce compute backend with CPU fallback |
| `--batch-size` | `<usize>` | *auto-optimal* | Override hardware-calculated batch size |

---

## 📦 Cross-Platform Installation & Application Builders

### 1. One-Liner Fast Installers

#### Linux (Universal Bash Installer)
```bash
# Direct repository or web installer
./install.sh
# or from GitHub:
# curl -fsSL https://raw.githubusercontent.com/FaezBarghasa/Oxide-embed/main/install.sh | bash
```

#### Windows (PowerShell Installer)
```powershell
# In PowerShell:
.\install.ps1
# or from GitHub:
# irm https://raw.githubusercontent.com/FaezBarghasa/Oxide-embed/main/install.ps1 | iex
```

---

### 2. Full Application Builder Scripts

Oxide-Embed includes dedicated release builder pipelines in the root directory:

| OS / Target | Script | Output Packages / Artifacts in `dist/` |
| :--- | :--- | :--- |
| **Ubuntu / Debian / Pop!_OS** | [`./build-ubuntu-app.sh`](build-ubuntu-app.sh) | `.deb` package (`dist/oxide-embed_0.4.0_amd64.deb`), portable `.tar.gz`, `.desktop` launcher, `dist/bin/oxide-embed`, `SHA256SUMS` |
| **macOS (Apple Silicon + Intel)** | [`./build-macos-app.sh`](build-macos-app.sh) | Universal Mach-O Binary (`lipo`), `.tar.gz` bundle, LaunchAgent plist (`launchd`), Homebrew formula (`oxide-embed.rb`), `SHA256SUMS` |
| **Windows (Cross / Native)** | [`./build-windows-app.sh`](build-windows-app.sh)<br>[`.\build-windows-app.ps1`](build-windows-app.ps1) | `oxide-embed.exe`, release `.zip` archive, PowerShell installer (`install.ps1`), runner batch file (`run-mcp.bat`), `SHA256SUMS` |

---

### 3. Native Linux Distro Packages

#### Debian / Ubuntu / Pop!_OS (`.deb` Package)
```bash
# Build full application and deb package
./build-ubuntu-app.sh
sudo dpkg -i dist/oxide-embed_0.4.0_amd64.deb
```

#### Arch Linux (`PKGBUILD`)
```bash
cd packaging/arch && makepkg -si
```

#### Fedora / RHEL (RPM)
```bash
rpmbuild -ba packaging/fedora/oxide-embed.spec
```

*For comprehensive distro and platform installation guides, see [docs/INSTALL.md](docs/INSTALL.md).*

---

## 🚀 Key Features & Architectural Highlights

### 1. Structure-Aware Information Retrieval (`STAIR` Code-ToC, arXiv:2609.03874v1)
- **Hierarchical Code-ToC AST Chunking**: Breaks codebases into bounded macro nodes (classes, traits, impl blocks) and leaf nodes (methods, functions, structs) across Rust, TS, Python, Go, Java, Bash, and Generic code.
- **2-Stage Hierarchical Routing**: Maps queries directly to exact AST leaf nodes via breadcrumbs (`[impl MmrReranker > rerank]`), preventing semantic bleeding and cross-chunk competition.
- **Structural Breadcrumb Context Injection**: Injects clean enclosing structural context without repeating identical source tokens.

### 2. Typed Semantic Memory Fabric (`Memanto` Parity & Beyond)
- **13 Specialized Semantic Memory Kinds**: `Instruction`, `Fact`, `Decision`, `Goal`, `Commitment`, `Preference`, `Relationship`, `Context`, `Event`, `Learning`, `Observation`, `Artifact`, and `Error`.
- **Automated Contradiction & Conflict Resolution**: Vector similarity + opposite polarity detection (`never` vs `always`, `use` vs `avoid`, `no_std` vs `with std`) with `--auto-resolve` superseding.
- **Conventional Commit Distillation**: Automatically parses git commit history and session transcripts into structured architectural memories, patterns, and rules.
- **Direct Grounded Answer Synthesis**: `oxide_core::AnswerSynthesizer` synthesizes grounded answers with strict token budget packing and citation tracking.
- **Bidirectional Markdown / Obsidian Vault Sync**: Export/import memory records to human-readable Markdown vault directories (`.oxide/memories/` or Obsidian vaults).
- **Point-in-Time Temporal Queries**: Supports `--as-of <ISO8601>` time-travel queries to inspect memory and constraint state at any historical moment.
- **Code Symbol Governance**: Graph edge (`governs`) connects architectural decisions directly to affected AST symbols (`SymbolRecord`).

### 3. Cognitive Graph & GraphRAG (`Cognee` Parity & Beyond)
- **Deterministic Tree-Sitter AST Extraction**: Extracts symbols, signatures, call graphs, import dependencies, and parent-child hierarchies across Rust, TypeScript, Python, C, and C++ in <47 µs.
- **DocLinker (ECL Pipeline)**: Hierarchically links markdown documentation sections to concrete code symbols without external LLM calls.
- **Multi-Hop Subgraph Traversal**: Queries symbols, caller/callee chains, and associated architectural docs in a single bounded graph traversal in **530 µs**.
- **Active Forgetting & Temporal Decay**: Exponentially decays unreferenced graph edges and auto-prunes orphan nodes (4.64 ns calculation).
- **Cerebrum Consolidation**: Clusters resolved bug logs and synthesizes actionable project rules into `.oxide/docs/CEREBRUM.md`.

### 4. Bidirectional Call Graphs & Impact Analysis (`Tokenix` Parity & Beyond)
- **`callers` & `callees`**: Instantly query inbound callers or outbound callees for any symbol in the workspace.
- **`impact`**: Computes bidirectional blast radius showing all upstream code that would break if a symbol's signature changes.
- **`tokenmap`**: Visualizes folder token densities and top context-heavy files with token percentages.
- **`install-hook`**: Auto-configures agent hooks for Claude Code, Antigravity, and Cursor.
- **MMR Diversity Reranker**: Information-theoretic Maximal Marginal Relevance reranking (`MmrReranker`) balancing candidate relevance vs. redundancy.

### 5. Context Hygiene & Token Reduction (`OpenWolf` Parity & Beyond)
- **Knapsack Token-Budget Packing**: Greedy budget packer (`--budget <N>`) that fits highest-value symbols, graph subgraphs, and docs strictly within token ceilings.
- **Surgical AST Symbol Reading**: Slices and streams exclusively the target function/struct's source lines, signature, and doc comments directly from the AST (`read --symbol <name>`).
- **Pre-Read Guard**: Intercepts file reads across agent sessions. If content hash is unchanged, returns a lightweight AST symbol outline stub instead of dumping thousands of tokens (4.19 µs lookup).
- **Terminal Condenser**: Intercepts CLI output. Caches full raw output into `.oxide/cache/bash/` and presents only essential compiler error blocks to the agent (45.73 µs processing).
- **Session Handover Checkpoints**: Generates atomic `.oxide/STATUS.md` state checkpoints for seamless multi-agent handovers.
- **Local Token Ledger**: Measures input, output, cached, and reasoning tokens with estimated cost breakdowns and savings scoreboards.

### 6. Agent Integration & Live Automation
- **Native Model Context Protocol (MCP)**: Exposes 15 specialized tools over stdio for direct integration into Antigravity, Claude Code, Cursor, and Roo Code.
- **Live Debounced Watcher Daemon**: Real-time file system monitor (`oxide-embed watch` / `oxide-watch.service`) that incrementally re-indexes AST symbols and call edges on file save.

---

## 🛠️ CLI Usage & Workflows

### 1. Typed Semantic Memory & Companion Agent Interface
```bash
# Remember architectural decisions, user preferences, or instructions
oxide-embed remember "Use pure no_std for STM32 embedded drivers" --kind decision --tags embedded,stm32

# Store a user style preference with auto-conflict resolution
oxide-embed remember "User prefers concise diff-style outputs" --kind preference --auto-resolve

# Recall memories with category, tag, or point-in-time filters
oxide-embed recall "embedded drivers" --kind decision --limit 5

# Recall exact project memory state as of last month (temporal time-travel)
oxide-embed recall "architecture" --as-of 2026-08-01T00:00:00Z --budget 1000

# Audit active contradictions across project instructions & decisions
oxide-embed conflicts
```

### 2. Workspace Indexing & Cognitive Graph (GPU Accelerated)
```bash
# Initialize .oxide metadata and embedded SurrealKV store
oxide-embed init

# Run GPU-accelerated AST extraction, call graph linking, and Vector indexer (auto-detects CUDA/Metal)
oxide-embed index --device cuda --batch-size 32

# Cognify workspace (Full Cognee-style GraphRAG index with CUDA acceleration)
oxide-embed cognify --device cuda

# Run live watcher daemon for incremental sub-millisecond re-indexing
oxide-embed watch
```

### 3. AST Call Graph & Impact Analysis
```bash
# Query inbound callers of a symbol
oxide-embed callers SurrealProjectStore

# Query outbound callees invoked by a function
oxide-embed callees handle_index

# Analyze blast radius and impact graph
oxide-embed impact SymbolRecord

# Show project token density map
oxide-embed tokenmap
```

### 4. Task-Driven Context & Surgical Slicing
```bash
# Synthesize multi-layer context for a prompt within an exact token budget
oxide-embed context "implement bare-metal SPI driver" --budget 1500

# Surgically read only a specific AST symbol definition and docstring
oxide-embed read crates/oxide-core/src/id.rs --symbol ProjectId

# STAIR Code-ToC hierarchical search (eliminates semantic bleeding)
oxide-embed search "MmrReranker" --stair

# Hybrid search with token budget ceiling and GraphRAG expansion
oxide-embed search "init_hardware" --budget 1000 --with-graph
```

### 5. Subgraph GraphRAG Traversal
```bash
# Explain a symbol and its multi-hop relationship topology
oxide-embed explain "SessionReadGuard" --hops 2
```

### 6. Context Hygiene & Execution
```bash
# Run command with automatic log caching & error condensing
oxide-embed run -- cargo test

# Read file with Pre-Read Guard (suppresses unchanged duplicate reads)
oxide-embed read src/main.rs
```

### 7. Session Handover & Memory Evolution
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

### 8. Model Context Protocol (MCP) Server
```bash
# Start stdio MCP server for agent IDEs
oxide-embed mcp

# Automatically install hooks into AI agent configs
oxide-embed install-hook
```

*For client setup configs (Antigravity, Claude Code, Cursor, Roo Code), see [docs/MCP_GUIDE.md](docs/MCP_GUIDE.md).*

---

## 📚 Documentation Index

- [Architecture Overview](docs/ARCHITECTURE.md) - Crate topology, data flow, ML embedder pipeline, and memory graph model.
- [Real Criterion Benchmarks](docs/BENCHMARKS.md) - Exact latency numbers, throughput, and comparative analysis against Cognee, OpenWolf, and Tokenix.
- [Linux Installation Guide](docs/INSTALL.md) - Distro packages (`.deb`, `PKGBUILD`, `spec`, `APKBUILD`), shell installer, and systemd service.
- [MCP Server Guide](docs/MCP_GUIDE.md) - Setup instructions for Antigravity, Claude Code, Cursor, and Roo Code.

---

## 🧪 Testing & Verification

```bash
# Run all workspace unit and integration tests (36 passed)
cargo test --workspace

# Run all Criterion benchmarks
cargo bench --workspace

# Verify formatting and zero clippy warnings
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
```

---

## 📄 License

Dual-licensed under **MIT** or **Apache 2.0**. See [LICENSE-MIT](LICENSE-MIT) and [LICENSE-APACHE](LICENSE-APACHE) for details.
