use std::path::PathBuf;
use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "oxide-embed")]
#[command(author = "Faez Barghasa")]
#[command(version = "0.1.0")]
#[command(about = "Local, portable, AST-aware memory and embedding engine for AI coding agents", long_about = None)]
pub struct Cli {
    #[arg(short, long, global = true, help = "Path to project root (defaults to current directory)")]
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

    #[command(about = "Index project files, generate AST outlines, and build memory")]
    Index {
        #[arg(long, help = "Force re-index all files even if unchanged")]
        force: bool,
    },

    #[command(about = "Show AST outline for a file without reading full content")]
    Outline {
        #[arg(help = "Relative path to target file")]
        path: String,
    },

    #[command(about = "Search project memory (semantic + lexical hybrid)")]
    Search {
        #[arg(help = "Search query string")]
        query: String,

        #[arg(short, long, default_value = "5", help = "Number of hits to return")]
        limit: usize,
    },

    #[command(about = "Export memory to portable .oxem bundle")]
    Export {
        #[arg(short, long, default_value = "memory.oxem", help = "Output .oxem bundle path")]
        out: PathBuf,
    },

    #[command(about = "Import memory from portable .oxem bundle")]
    Import {
        #[arg(help = "Input .oxem bundle path")]
        bundle: PathBuf,
    },
}
