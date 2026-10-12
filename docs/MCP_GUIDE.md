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

Oxide-embed exposes **19 specialized tools** for AI coding assistants:

| Tool Name | Description | Key Arguments |
| :--- | :--- | :--- |
| `oxide_context` | Synthesize token-budgeted cognitive context for a task (rules + active handoff + 2-hop GraphRAG) | `task` (string), `budget` (int) |
| `oxide_read_symbol` | Surgically read only the exact AST symbol definition and docstring from a file | `file_path` (string), `symbol` (string) |
| `oxide_search` | Search code symbols and knowledge graph with optional budget limit | `query` (string), `budget` (int), `with_graph` (bool) |
| `oxide_explain` | Explain a symbol's multi-hop call graph and documentation topology | `symbol` (string), `hops` (int) |
| `oxide_condense` | Condense raw terminal compiler output and cache full log to disk | `command` (string), `raw_output` (string), `exit_code` (int) |
| `oxide_remember` | Store typed semantic memory (decision, preference, instruction) with conflict checks | `content` (string), `kind` (string), `title` (string), `tags` (array), `symbol_ref` (string), `auto_resolve` (bool) |
| `oxide_recall` | Recall typed semantic memories with category, tags, and temporal filters | `query` (string), `kind` (string), `tags` (array), `as_of` (string), `budget` (int), `limit` (int) |
| `oxide_get_rules` | Returns active consolidated architectural constraints, decisions, and instructions | None |
| `oxide_answer` | Generate direct grounded answers synthesizing recalled memories, rules, and live AST code graph | `question` (string), `kind` (string), `budget` (int) |
| `oxide_distill` | Distills decisions, learnings, and resolved errors from recent git commit history into project memory | `commits` (string), `save` (bool) |
| `oxide_sync_memories` | Synchronizes memories bidirectionally with Obsidian / Markdown notes in `.oxide/memories/` | `direction` ("export" / "import" / "bidirectional") |
| `oxide_stair_search` | STAIR hierarchical Code-ToC leaf retrieval with breadcrumb routing | `query` (string), `limit` (int) |
| `oxide_distill_skill` | Distill an engineered procedural skill (Agent Instruction Protocol / AIP) into `.oxide/skills/` | `name` (string), `description` (string), `workflow_steps` (array) |
| `oxide_recall_skill` | Recall procedural skills matching an intent or trigger | `query` (string) |
| `oxide_reflect` | Tiered cognitive reflection: standing mental models or 4-way RRF recall | `query` (string), `budget` (int), `force_fresh` (bool) |
| `oxide_model_list` | List all active standing mental models and their freshness states | None |
| `oxide_model_get` | Get a specific standing mental model by its topic slug | `topic` (string) |
| `oxide_model_create` | Create or update a standing mental model with question, answer, and referenced symbols | `topic` (string), `question` (string), `answer` (string) |
| `oxide_update` | Incrementally updates project context (changed/added/removed files) and agent intelligence (reusable crates & git distillation) | `scan_crates` (bool), `distill_git` (bool) |

---

## 4. Tool Usage Examples

### Example 1: Remembering an Architectural Decision
```json
{
  "name": "oxide_remember",
  "arguments": {
    "content": "Always use static bounded buffers (heapless) in STM32 bare-metal drivers",
    "kind": "decision",
    "tags": ["embedded", "stm32", "memory"],
    "auto_resolve": true
  }
}
```

### Example 2: Recalling Active Rules with Token Budget
```json
{
  "name": "oxide_recall",
  "arguments": {
    "query": "bare-metal STM32 driver constraints",
    "kind": "decision",
    "budget": 500
  }
}
```

### Example 3: Multi-hop GraphRAG Query
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

### Example 4: Compressing Compiler Output
```json
{
  "name": "oxide_condense_errors",
  "arguments": {
    "raw_output": "error[E0433]: failed to resolve: use of undeclared crate or module `tokio`\n  --> src/main.rs:12:5\n   |\n12 |     tokio::spawn(async move { ... });"
  }
}
```
