# Oxide-embed Model Context Protocol (MCP) Guide

Oxide-embed implements a high-performance **Model Context Protocol (MCP)** server over stdio, empowering AI coding agents with sub-millisecond AST queries, GraphRAG multi-hop code reasoning, and token hygiene.

---

## 1. Quick Automated Hook Installation

You can automatically configure your AI agent (Claude Code, Google Antigravity, Cursor) with a single command:

```bash
oxide-embed install-hook
```

---

## 2. Manual Client Configuration

### A. Google Antigravity / Gemini CLI (`mcp_config.json`)

Add the following to `~/.gemini/antigravity-ide/mcp/oxide-embed/config.json` or your global `mcp_config.json`:

```json
{
  "mcpServers": {
    "oxide-embed": {
      "command": "oxide-embed",
      "args": ["mcp"],
      "env": {
        "RUST_LOG": "info"
      }
    }
  }
}
```

### B. Claude Code (`~/.claude.json` or `claude mcp add`)

```bash
claude mcp add oxide-embed -- oxide-embed mcp
```

Or configure manually in `claude_desktop_config.json`:
```json
{
  "mcpServers": {
    "oxide-embed": {
      "command": "/usr/local/bin/oxide-embed",
      "args": ["mcp"]
    }
  }
}
```

### C. Cursor / Roo Code (`mcp.json`)

```json
{
  "servers": {
    "oxide-embed": {
      "command": "oxide-embed",
      "args": ["mcp"]
    }
  }
}
```

---

## 3. Available MCP Tools

Oxide-embed exposes **11 specialized tools** for AI coding assistants:

| Tool Name | Description | Key Arguments |
| :--- | :--- | :--- |
| `oxide_index_workspace` | Incremental AST and vector indexing of codebase | `path` (string), `force` (bool) |
| `oxide_search_hybrid` | Combined BM25 lexical & vector semantic search | `query` (string), `limit` (int) |
| `oxide_query_graph` | Multi-hop GraphRAG symbol traversal (callers, callees) | `symbol` (string), `depth` (int) |
| `oxide_extract_ast` | Tree-sitter symbol outline and AST anatomy | `file_path` (string) |
| `oxide_get_outline` | High-level symbol hierarchy (classes, functions, traits) | `file_path` (string) |
| `oxide_link_docs` | Bi-directional docstring and symbol relationship map | `file_path` (string) |
| `oxide_read_guard` | Context deduplication filter to eliminate repetitive reads | `session_id` (string), `file_path` (string) |
| `oxide_condense_errors` | Compresses raw compiler/terminal output for token economy | `raw_output` (string) |
| `oxide_token_ledger` | Session token balance, burn rate, and Ebbinghaus decay | `session_id` (string) |
| `oxide_check_drift` | Validates database index consistency against filesystem | `path` (string) |
| `oxide_mcp_status` | Returns live health, memory stats, and index metrics | None |

---

## 4. Tool Usage Examples

### Example 1: Multi-hop GraphRAG Query
Requesting callers and structural dependencies for a struct or function:
```json
{
  "name": "oxide_query_graph",
  "arguments": {
    "symbol": "SessionReadGuard",
    "depth": 2
  }
}
```
**Response Time**: ~530 µs (sub-millisecond embedded traversal).

### Example 2: Compressing Compiler Output
```json
{
  "name": "oxide_condense_errors",
  "arguments": {
    "raw_output": "error[E0433]: failed to resolve: use of undeclared crate or module `tokio`\n  --> src/main.rs:12:5\n   |\n12 |     tokio::spawn(async move { ... });"
  }
}
```
