use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "oxide-embed")]
#[command(author = "Faez Barghasa")]
#[command(version = "0.3.0")]
#[command(about = "Local, portable, AST-aware memory and embedding engine for AI coding agents", long_about = None)]
pub struct Cli {
    #[arg(
        short,
        long,
        global = true,
        help = "Path to project root (defaults to current directory)"
    )]
    pub project_dir: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    #[command(about = "Initialize .oxide memory container in project root")]
    Init {
        #[arg(short, long, help = "Project name override")]
        name: Option<String>,
    },

    #[command(about = "Run local health and configuration diagnostics")]
    Doctor,

    #[command(
        about = "Index project files, generate AST outlines, extract docs, and build knowledge graph"
    )]
    Index {
        #[arg(long, help = "Force re-index all files even if unchanged")]
        force: bool,
    },

    #[command(
        about = "Cognify project: full AST parsing, vector embedding, doc-to-graph linking, and knowledge graph construction"
    )]
    Cognify {
        #[arg(long, help = "Force full re-cognify")]
        force: bool,
    },

    #[command(about = "Show AST outline for a file without reading full content")]
    Outline {
        #[arg(help = "Relative path to target file")]
        path: String,
    },

    #[command(
        about = "Synthesize multi-layer token-budgeted context for a task (active status + Cerebrum rules + 2-hop GraphRAG)"
    )]
    Context {
        #[arg(help = "Task description or query")]
        task: String,

        #[arg(
            short,
            long,
            default_value = "1500",
            help = "Maximum token budget for synthesized context"
        )]
        budget: usize,
    },

    #[command(
        about = "Search project memory (semantic + lexical hybrid with optional GraphRAG expansion)"
    )]
    Search {
        #[arg(help = "Search query string")]
        query: String,

        #[arg(short, long, default_value = "5", help = "Number of hits to return")]
        limit: usize,

        #[arg(long, help = "Expand search hits with multi-hop subgraph context")]
        with_graph: bool,

        #[arg(
            long,
            default_value = "1",
            help = "Number of hops for subgraph expansion"
        )]
        hops: usize,

        #[arg(long, help = "Token budget ceiling for packed search results")]
        budget: Option<usize>,

        #[arg(
            long,
            help = "Use STAIR (Structure-Aware Information Retriever) hierarchical Code-ToC routing"
        )]
        stair: bool,
    },

    #[command(
        about = "Explain a symbol with its multi-hop knowledge graph context (calls, callers, parent file, referenced docs)"
    )]
    Explain {
        #[arg(help = "Symbol name or path to explain")]
        symbol: String,

        #[arg(long, default_value = "2", help = "Traversal hop depth")]
        hops: usize,
    },

    #[command(
        about = "Run a terminal command through TerminalCondenser (caches raw output and truncates large logs)"
    )]
    Run {
        #[arg(
            required = true,
            trailing_var_arg = true,
            help = "Command and arguments to execute"
        )]
        command: Vec<String>,
    },

    #[command(about = "Read a file through SessionReadGuard or surgically slice an AST symbol")]
    Read {
        #[arg(help = "Relative path to target file")]
        path: PathBuf,

        #[arg(
            short,
            long,
            help = "Surgically extract and slice only a specific symbol"
        )]
        symbol: Option<String>,

        #[arg(long, help = "Force full file content read even if unchanged")]
        force: bool,
    },

    #[command(about = "Generate AI session handoff checkpoint and write .oxide/STATUS.md")]
    Handoff {
        #[arg(long, help = "Active engineering goal")]
        goal: Option<String>,

        #[arg(long, help = "Next immediate action required")]
        next: Option<String>,
    },

    #[command(about = "Display Token Efficiency, Context Savings breakdown, and cost statistics")]
    Report,

    #[command(
        about = "Memify: perform active forgetting, decay edge weights, and prune orphan nodes"
    )]
    Memify {
        #[arg(long, default_value = "30.0", help = "Decay half life in days")]
        decay_days: f64,

        #[arg(long, default_value = "0.15", help = "Pruning weight threshold")]
        prune_threshold: f32,
    },

    #[command(
        about = "Consolidate buglogs and recurring memory patterns into .oxide/docs/CEREBRUM.md"
    )]
    Consolidate,

    #[command(about = "Query inbound callers of a target symbol")]
    Callers {
        #[arg(help = "Target symbol name or qualified path")]
        symbol: String,

        #[arg(short, long, help = "Token budget ceiling")]
        budget: Option<usize>,
    },

    #[command(about = "Query outbound callees invoked by a target symbol")]
    Callees {
        #[arg(help = "Target symbol name or qualified path")]
        symbol: String,

        #[arg(short, long, help = "Token budget ceiling")]
        budget: Option<usize>,
    },

    #[command(about = "Show bidirectional blast radius and impact graph for a symbol")]
    Impact {
        #[arg(help = "Target symbol name")]
        symbol: String,
    },

    #[command(about = "Show token density and context map tree across project directories")]
    Tokenmap {
        #[arg(short, long, default_value = "8", help = "Maximum directory depth")]
        depth: usize,
    },

    #[command(about = "Automatically install and configure Oxide-embed hooks for AI agents")]
    InstallHook {
        #[arg(
            short,
            long,
            default_value = "all",
            help = "Target tool: all, antigravity, claude, cursor"
        )]
        tool: String,
    },

    #[command(
        about = "Remember a typed semantic memory (instruction, decision, preference, fact, learning, etc.)"
    )]
    Remember {
        #[arg(help = "Memory content to store")]
        content: String,

        #[arg(
            short,
            long,
            help = "Category: instruction, fact, decision, goal, commitment, preference, relationship, context, event, learning, observation, artifact, error"
        )]
        kind: Option<String>,

        #[arg(short, long, help = "Optional title")]
        title: Option<String>,

        #[arg(short, long, value_delimiter = ',', help = "Comma-separated tags")]
        tags: Vec<String>,

        #[arg(long, help = "Symbol or function governed by this memory")]
        symbol: Option<String>,

        #[arg(long, help = "Automatically supersede older conflicting memory")]
        auto_resolve: bool,
    },

    #[command(about = "Recall typed semantic memories with category and temporal filters")]
    Recall {
        #[arg(help = "Search query or topic")]
        query: String,

        #[arg(
            short,
            long,
            help = "Category filter: instruction, fact, decision, goal, commitment, preference, relationship, context, event, learning, observation, artifact, error"
        )]
        kind: Option<String>,

        #[arg(short, long, value_delimiter = ',', help = "Filter by tags")]
        tags: Vec<String>,

        #[arg(long, help = "Point-in-time timestamp (RFC3339)")]
        as_of: Option<String>,

        #[arg(short, long, help = "Token budget ceiling")]
        budget: Option<usize>,

        #[arg(short, long, default_value = "5", help = "Max results")]
        limit: usize,
    },

    #[command(about = "Review active contradictions and conflicts across rules and decisions")]
    Conflicts,

    #[command(
        about = "Generate direct grounded answers synthesizing recalled memories, rules, and code graph"
    )]
    Answer {
        #[arg(help = "Question or query to answer")]
        question: String,

        #[arg(
            short,
            long,
            help = "Category filter: instruction, decision, fact, preference, etc."
        )]
        kind: Option<String>,

        #[arg(
            short,
            long,
            default_value = "1000",
            help = "Token budget ceiling for the answer"
        )]
        budget: usize,
    },

    #[command(
        about = "Distill architectural decisions, learnings, and errors from git commit history"
    )]
    Distill {
        #[arg(short, long, help = "Git commit range (e.g. HEAD~10..HEAD)")]
        commits: Option<String>,

        #[arg(short, long, help = "Save distilled memories directly to database")]
        save: bool,
    },

    #[command(
        about = "Synchronize memories bidirectionally with Obsidian / Markdown notes in .oxide/memories/"
    )]
    SyncNotes {
        #[arg(short, long, help = "Direction: export, import, bidirectional")]
        direction: Option<String>,
    },

    #[command(
        about = "Start Model Context Protocol (MCP) server over stdio for AI agent integration",
        alias = "mcp-serve",
        alias = "serve"
    )]
    Mcp,

    #[command(
        about = "Start real-time debounced file watcher for incremental sub-millisecond AST re-indexing"
    )]
    Watch,

    #[command(about = "Export memory to portable .oxem bundle")]
    Export {
        #[arg(
            short,
            long,
            default_value = "memory.oxem",
            help = "Output .oxem bundle path"
        )]
        out: PathBuf,
    },

    #[command(about = "Import memory from portable .oxem bundle")]
    Import {
        #[arg(help = "Input .oxem bundle path")]
        bundle: PathBuf,
    },
}
