mod cli;
mod commands;

use std::env;
use std::process::ExitCode;
use clap::Parser;
use cli::{Cli, Commands};
use commands::{
    handle_doctor, handle_export, handle_import, handle_index, handle_init, handle_outline,
    handle_search,
};

#[tokio::main]
async fn main() -> ExitCode {
    tracing_subscriber::fmt::init();

    let args = Cli::parse();
    let current_dir = env::current_dir().unwrap_or_default();
    let project_root = args.project_dir.as_deref().unwrap_or(&current_dir);

    let result = match args.command {
        Commands::Init { name } => handle_init(project_root, name),
        Commands::Doctor => handle_doctor(project_root).await,
        Commands::Index { force } => handle_index(project_root, force).await,
        Commands::Outline { path } => handle_outline(project_root, &path).await,
        Commands::Search { query, limit } => handle_search(project_root, &query, limit).await,
        Commands::Export { out } => handle_export(project_root, &out).await,
        Commands::Import { bundle } => handle_import(project_root, &bundle).await,
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}
