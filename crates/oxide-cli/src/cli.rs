use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "oxide-embed")]
#[command(author = "Faez Barghasa")]
#[command(version = "0.1.0")]
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

    #[command(
        about = "Read a file through SessionReadGuard or surgically slice an AST symbol"
    )]
    Read {
        #[arg(help = "Relative path to target file")]
        path: PathBuf,

        #[arg(short, long, help = "Surgically extract and slice only a specific symbol")]
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

    #[command(about = "Start Model Context Protocol (MCP) server over stdio for AI agent integration")]
    Mcp,

    #[command(about = "Start real-time debounced file watcher for incremental sub-millisecond AST re-indexing")]
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
