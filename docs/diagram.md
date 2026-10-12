# Oxide-embed Architectural & Schema Diagrams

This document contains visual diagrams depicting the architecture, data schemas, memory layers, and graph topologies of **Oxide-embed**.

---

## 1. Dual-Tier Cognitive Memory Fabric

```mermaid
graph TB
    subgraph Tier1["Tier 1: Project-Specific Context Engine (SurrealKV)"]
        direction TB
        AST["Tree-Sitter AST Engine\n(Rust, TS, Python, Go, C/C++)"]
        CodeToC["STAIR Code-ToC\nMacro Nodes & Leaf Slices"]
        GraphEdges["Relational Topology\nCALLS, IMPORTS, DEFINED_IN"]
        DocLink["DocLinker\nMarkdown to Code Symbol Maps"]
        FileHash["Content Hash Ledger\nDelta Invalidation"]
    end

    subgraph Tier2["Tier 2: Agent Metacognitive Intelligence (Cross-Project & Persistent)"]
        direction TB
        Memanto["Memanto Semantic Memory\n13 Typed Kinds & Polarity Resolution"]
        MentalModels["Standing Mental Models\nCached High-Level Syntheses"]
        AIPSkills["Procedural Skills (AIP)\nWorkflow Triggers & Recipes"]
        CrateCatalog["Crate Anti-Duplication Catalog\n~/RustroverProjects/ Semantic Index"]
        Cerebrum["Cerebrum Rule Consolidation\nLearnings & Policy Constraints"]
    end

    subgraph Runtime["Agent Runtime & MCP Interface"]
        MCP["Model Context Protocol Server\n19 Asynchronous Tools"]
        RRF["4-Way RRF Reflection Engine\nDense + STAIR + GraphRAG + Decay"]
        Knapsack["Token Budget Packer\nGreedy Knapsack Fitting"]
    end

    AST --> CodeToC
    CodeToC --> GraphEdges
    Tier1 --> RRF
    Tier2 --> RRF
    RRF --> Knapsack
    Knapsack --> MCP
    CrateCatalog -.->|Anti-Duplication Discovery| Tier1
```

---

## 2. SurrealDB Multi-Model Schema & Graph Relational Diagram

```mermaid
erDiagram
    file ||--o{ symbol : "contains"
    file ||--o{ chunk : "sliced_into"
    symbol ||--o{ chunk : "has_macro_leaf"
    symbol ||--o{ calls : "inbound_or_outbound"
    symbol ||--o{ imports : "imported_by"
    doc_section ||--o{ documents : "explains_symbol"
    symbol ||--o{ documents : "annotated_by"
    memory_record ||--o{ symbol : "governs"
    mental_model }o--o{ symbol : "references"
    mental_model }o--o{ file : "depends_on"

    file {
        string id PK
        string project_id
        string relative_path
        string content_hash
        int size_bytes
        datetime indexed_at
    }

    symbol {
        string id PK
        string file_id FK
        string name
        string qualified_name
        string kind
        string signature
        int start_line
        int end_line
    }

    chunk {
        string id PK
        string file_id FK
        string symbol_id FK
        string kind
        string outline
        string text
        array embedding
        string content_hash
    }

    calls {
        string id PK
        string caller_id FK
        string callee_id FK
        string caller_name
        string callee_name
        int line_number
    }

    mental_model {
        string id PK
        string topic
        string question
        string answer
        bool is_stale
        datetime updated_at
    }

    crate_catalog {
        string id PK
        string crate_name
        string path
        string version
        string target_arch
        array embedding
    }
```

---

## 3. GraphRAG 2-Hop Subgraph Topology

```mermaid
graph LR
    TargetSymbol["<b>Target Symbol</b><br/><code>fn handle_update()</code>"]
    
    Caller1["Caller: <code>fn main()</code>"]
    Caller2["Caller: <code>fn mcp_oxide_update()</code>"]
    
    Callee1["Callee: <code>ProjectWalker::walk()</code>"]
    Callee2["Callee: <code>store.upsert_chunks_batch()</code>"]
    Callee3["Callee: <code>scan_and_catalog_crates()</code>"]
    
    Doc["DocSection: <code># Delta Sync Spec</code>"]
    Rule["Rule: <code>Zero blocking calls on Tokio</code>"]

    Caller1 -->|CALLS| TargetSymbol
    Caller2 -->|CALLS| TargetSymbol
    TargetSymbol -->|CALLS| Callee1
    TargetSymbol -->|CALLS| Callee2
    TargetSymbol -->|CALLS| Callee3
    Doc -->|DOCUMENTS| TargetSymbol
    Rule -->|GOVERNS| TargetSymbol

    style TargetSymbol fill:#ff8800,stroke:#333,stroke-width:2px,color:#fff
```

---

## 4. STAIR Code-ToC Hierarchical Breadcrumb Structure

```mermaid
graph TD
    File["File: <code>crates/oxide-db/src/surreal.rs</code>"]
    Macro["Macro Node: <code>impl ProjectStore for SurrealProjectStore</code>"]
    Leaf1["Leaf Node: <code>async fn upsert_chunks_batch()</code>"]
    Leaf2["Leaf Node: <code>async fn get_subgraph()</code>"]
    Leaf3["Leaf Node: <code>async fn search_hybrid()</code>"]

    File --> Macro
    Macro -->|Breadcrumb: [impl ProjectStore > upsert_chunks_batch]| Leaf1
    Macro -->|Breadcrumb: [impl ProjectStore > get_subgraph]| Leaf2
    Macro -->|Breadcrumb: [impl ProjectStore > search_hybrid]| Leaf3
```
