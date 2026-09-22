# Changelog

All notable changes to the **`Oxide-embed`** project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [0.4.0] - 2026-09-22

### Added

#### 1. Full Cross-Platform Application Builders
- **Ubuntu / Debian / Pop!_OS (`build-ubuntu-app.sh`)**:
  - Full automated application build script compiling release binary with LTO, stripping symbols, assembling Debian package (`.deb`), generating portable `.tar.gz`, `.desktop` launcher, and `SHA256SUMS`.
- **macOS Universal Application (`build-macos-app.sh`)**:
  - Universal 2 Mach-O binary packaging via `lipo` (Apple Silicon `aarch64-apple-darwin` + Intel `x86_64-apple-darwin`).
  - Generates `launchd` LaunchAgent service plist (`com.faezbarghasa.oxide-embed.plist`), Homebrew formula (`oxide-embed.rb`), release tarball, and SHA256 checksums.
- **Windows Applications (`build-windows-app.sh` & `build-windows-app.ps1`)**:
  - Cross-compilation support via `cargo-xwin` (MSVC) or MinGW (`x86_64-pc-windows-gnu`).
  - Native PowerShell builder (`build-windows-app.ps1`), release `.zip` package, runner batch scripts, and SHA256 checksums.

#### 2. Cross-Platform One-Liner Installers
- **Universal Linux Installer (`install.sh`)**:
  - Distro package manager detection (`dpkg` fast-path for `.deb`), user-level (`~/.local/bin`) or root (`/usr/local/bin`) installation, automated shell completions (Bash, Zsh, Fish), and Systemd user daemon setup (`oxide-watch.service`).
- **Universal Windows PowerShell Installer (`install.ps1`)**:
  - Auto-locates binary or compiles on-demand, manages Windows User `PATH`, configures PowerShell completions, and adds desktop / batch launchers.

---

## [0.3.1] - 2026-09-19

### Added

#### 1. Hardware GPU Acceleration (CUDA, Apple Metal, AMD ROCm)
- **Multi-Backend Acceleration (`oxide-ml`)**:
  - `cuda`: Integrated `candle-core/cuda`, `candle-nn/cuda`, `candle-transformers/cuda`, and `ort/cuda` for NVIDIA GPUs.
  - `metal`: Integrated `candle-core/metal` and `ort/coreml` for Apple Silicon GPU / Apple Neural Engine.
  - `rocm`: Integrated `ort/rocm` Execution Provider for AMD GPUs.
- **Dynamic Device Selector (`oxide_ml::device`)**:
  - Auto-probes CUDA and Metal at runtime with graceful CPU SIMD fallback.
  - Added `--device <auto|cuda|metal|rocm|cpu>` CLI parameter to `index` and `cognify`.
- **True Batched Tensor Forward Pass**:
  - Replaced sequential single-item embeddings with 2D tensor batching (`tokenizer.encode_batch`), attention masking (`[B, S]`), mean pooling, and L2 normalization in Candle BERT and Qwen embedders.
  - Added dynamic hardware-optimal batch calculation (`optimal_batch_size`), defaulting to 64 for CUDA, 32 for Metal, and `cores * 2` for CPU SIMD, with optional `--batch-size <N>` override.

---

## [0.3.0] - 2026-09-18

### Added

#### 1. STAIR (Structure-Aware Information Retriever, arXiv:2609.03874v1) Code-ToC Engine
- **AST Macro & Leaf Node Hierarchical Chunking**:
  - Implemented across all extractors: Rust, TypeScript, Python, Go, Java, Bash, and Generic fallback.
  - Automatically identifies macro container boundaries (`is_macro_node`) and leaf nodes, extracting breadcrumbs (e.g. `[impl MmrReranker > rerank]`) and natural summaries.
- **Hierarchical STAIR Retrieval**:
  - Direct 2-stage AST leaf-node routing in `oxide-db` via SurrealDB with deterministic file mapping.
  - Added `--stair` flag to `oxide-embed search` CLI command.
  - Added `oxide_stair_search` MCP tool for agentic workflows to eliminate semantic bleeding.

#### 2. Memanto Semantic Memory Fabric Primitives
- **Conventional Commit & Log Distillation (`MemoryDistiller`)**:
  - Automatically distills git commits and raw session logs into structured architectural memories, patterns, and rules.
- **Direct Grounded Answer Synthesis (`AnswerSynthesizer`)**:
  - Synthesizes grounded answers directly from retrieved AST context and memories with citation mapping and token budget limits.
- **Bidirectional Markdown / Obsidian Sync (`MarkdownMemorySync`)**:
  - Bidirectional synchronization between SurrealDB typed memory records and Markdown vault directories (`.oxide/memories/` or Obsidian).
- **13 Specialized Semantic Memory Categories (`MemoryKind`)**:
  - `Instruction`, `Fact`, `Decision`, `Goal`, `Commitment`, `Preference`, `Relationship`, `Context`, `Event`, `Learning`, `Observation`, `Artifact`, `Error`.
- **Automated Contradiction & Conflict Resolution (`ConflictDetector`)**:
  - Semantic vector similarity combined with lexical opposite polarity detection (`never` vs `always`, `use` vs `avoid`, `no_std` vs `with std`, etc.).
  - Automatic superseding with `--auto-resolve` flag or interactive audit via `oxide-embed conflicts`.
- **Temporal Point-in-Time State Querying (`--as-of`)**:
  - Query workspace memory state as of any ISO-8601 timestamp with automatic `valid_until` boundary evaluation.
- **Code Symbol Governance (`governs` Graph Edge)**:
  - Graph edges connecting architectural rules and decisions directly to target AST code symbols (`SymbolRecord`).

#### 3. Information-Theoretic MMR Diversity Reranker (`oxide-ml`)
- **`MmrReranker`**:
  - Implements Maximal Marginal Relevance to balance candidate relevance vs. redundancy, eliminating duplicate context tokens.

#### 4. Model Context Protocol (MCP) Expansion (15 Tools)
- Added `oxide_stair_search`, `oxide_remember`, and `oxide_recall` MCP tools for IDE-native agent workflows.

---

## [0.2.0] - 2026-09-17

### Added

#### 1. Multi-Model Production Embedding Engines (`oxide-ml`)
- **`EmbeddingGemma-300M` (ONNX Runtime)**:
  - Hardware-accelerated ONNX Runtime (`ort 2.0.0-rc.13`) integration with `libonnxruntime.so`.
  - 768-dimensional embeddings with token truncation safety and L2 normalization.
- **`Qwen3-Embedding-0.6B` (Candle)**:
  - High-precision 1024-dimensional embeddings implemented via pure `candle-core` / `candle-nn`.
  - Thread-safe Mutex forward pass for batch and concurrent indexing without Python runtime.
- **`BGE-Small-en-v1.5` (Candle BERT)**:
  - 384-dimensional BERT embeddings with local safetensors auto-detection and fallback.

#### 2. AST Call Graph & Dependency Extraction (`oxide-parser`)
- **Bidirectional Call & Import Extraction**:
  - `LanguageExtractor::extract_call_edges()` and `LanguageExtractor::extract_import_edges()`.
  - Comprehensive AST parsing across Rust, TypeScript, Python, C, and C++.
  - Extraction of function calls, method calls, macro invocations, and module imports.
- **New CLI Commands**:
  - `oxide-embed callers <symbol>`: Query inbound callers of any symbol in the workspace.
  - `oxide-embed callees <symbol>`: Query outbound functions/methods invoked by a target.
  - `oxide-embed impact <symbol>`: Compute blast radius showing upstream code that would break if a symbol changes.
  - `oxide-embed tokenmap`: Tree-based token density mapping with folder-by-folder breakdowns and percentage weights.
  - `oxide-embed install-hook`: Automatically configures hooks for Claude Code, Antigravity, and Cursor.

#### 3. Canonical DB Resolution & Robust File Watcher (`oxide-core` & `oxide-cli`)
- **Canonical Storage Path Resolution (`resolve_db_path`)**:
  - Automatically resolves `.oxide/project.db` (SurrealKV) with backward-compatibility fallbacks.
  - Fixes database lock conflicts and ensures unified storage between CLI and daemon watcher.
- **Live Debounced Watcher Daemon (`WorkspaceWatcher`)**:
  - Real-time file system monitoring with debounced incremental re-indexing of AST symbols, call edges, and doc references.

#### 4. Multi-Distro Linux Packaging & Automated Installation
- **Universal Installer Script (`scripts/install.sh`)**: Architecture detection (x86_64, aarch64), automated binary placement in `/usr/local/bin` or `~/.local/bin`, shell completion setup, and optional systemd user service activation.
- **Native Debian Package Builder (`scripts/package-deb.sh`)**: Produces stripped, dependency-resolved `dist/oxide-embed_0.2.0_amd64.deb` using `dpkg-deb`.
- **Arch Linux Package (`packaging/arch/PKGBUILD`)**: Native `makepkg` recipe compiling with release optimizations.
- **Fedora / RHEL RPM Specification (`packaging/fedora/oxide-embed.spec`)**: Standard RPM build spec with systemd-rpm-macros.
- **Alpine Linux Recipe (`packaging/alpine/APKBUILD`)**: Musl libc package recipe for ultra-lightweight containers.
- **Systemd User Service (`packaging/systemd/oxide-watch.service`)**: Background daemon unit managing continuous file watching.

#### 5. Real Criterion Benchmark Suite & Performance Scorecards
- **19 Benchmark Suites Across All 5 Crates (`benches/`)**:
  - UUIDv7 generation (1.51 µs), Blake3/SHA256 path hashing (325.58 ns).
  - Read guard cache lookup (4.19 µs), error condenser (45.73 µs).
  - Token ledger calculation (1.62 µs), Ebbinghaus memory decay (4.64 ns).
  - Tree-sitter Rust (46.82 µs), TypeScript (41.78 µs), Python (31.06 µs).
  - Hybrid BM25/Vector search (149.06 µs), 2-Hop GraphRAG traversal (530.43 µs).
- **Benchmark Documentation (`docs/BENCHMARKS.md`)**: Full report with confidence intervals and direct comparisons against Cognee, OpenWolf, and Tokenix.

#### 6. Dual Licensing (MIT OR Apache-2.0)
- Added `LICENSE-MIT`, `LICENSE-APACHE`, and updated root `Cargo.toml`.

---

## [0.1.0] - 2026-09-17

### Added
- Initial workspace architecture (`oxide-core`, `oxide-db`, `oxide-parser`, `oxide-ml`, `oxide-cli`).
- Embedded SurrealDB integration with in-memory and SurrealKV datastores.
- Tree-sitter AST extraction and chunking.
- Model Context Protocol (MCP) server foundation over stdio.
