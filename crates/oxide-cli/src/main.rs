mod cli;
mod commands;
mod mcp;
mod watcher;

use clap::Parser;
use cli::{Cli, Commands};
use commands::{
    handle_answer, handle_callees, handle_callers, handle_conflicts, handle_consolidate,
    handle_context, handle_distill, handle_doctor, handle_explain, handle_export, handle_handoff,
    handle_impact, handle_import, handle_index, handle_init, handle_install_hook, handle_memify,
    handle_outline, handle_read, handle_recall, handle_remember, handle_report, handle_run,
    handle_search, handle_sync_notes, handle_tokenmap,
};
use mcp::McpServer;
use std::env;
use std::process::ExitCode;
use watcher::WorkspaceWatcher;

#[tokio::main]
async fn main() -> ExitCode {
    tracing_subscriber::fmt::init();

    let args = Cli::parse();
    let current_dir = env::current_dir().unwrap_or_default();
    let project_root = args.project_dir.as_deref().unwrap_or(&current_dir);

    let result = match args.command {
        Commands::Init { name } => handle_init(project_root, name),
        Commands::Doctor => handle_doctor(project_root).await,
        Commands::Index {
            force,
            device,
            batch_size,
        }
        | Commands::Cognify {
            force,
            device,
            batch_size,
        } => handle_index(project_root, force, device.as_deref(), batch_size).await,
        Commands::Outline { path } => handle_outline(project_root, &path).await,
        Commands::Context { task, budget } => handle_context(project_root, &task, budget).await,
        Commands::Search {
            query,
            limit,
            with_graph,
            hops,
            budget,
            stair,
        } => handle_search(project_root, &query, limit, with_graph, hops, budget, stair).await,
        Commands::Explain { symbol, hops } => handle_explain(project_root, &symbol, hops).await,
        Commands::Callers { symbol, budget } => handle_callers(project_root, &symbol, budget).await,
        Commands::Callees { symbol, budget } => handle_callees(project_root, &symbol, budget).await,
        Commands::Impact { symbol } => handle_impact(project_root, &symbol).await,
        Commands::Tokenmap { depth } => handle_tokenmap(project_root, depth).await,
        Commands::InstallHook { tool } => handle_install_hook(project_root, &tool).await,
        Commands::Run { command } => handle_run(project_root, &command).await,
        Commands::Read {
            path,
            symbol,
            force,
        } => handle_read(project_root, &path, symbol.as_deref(), force).await,
        Commands::Handoff { goal, next } => handle_handoff(project_root, goal, next).await,
        Commands::Report => handle_report(project_root).await,
        Commands::Memify {
            decay_days,
            prune_threshold,
        } => handle_memify(project_root, decay_days, prune_threshold).await,
        Commands::Remember {
            content,
            kind,
            title,
            tags,
            symbol,
            auto_resolve,
        } => {
            handle_remember(
                project_root,
                &content,
                kind.as_deref(),
                title.as_deref(),
                &tags,
                symbol.as_deref(),
                auto_resolve,
            )
            .await
        }
        Commands::Recall {
            query,
            kind,
            tags,
            as_of,
            budget,
            limit,
        } => {
            handle_recall(
                project_root,
                &query,
                kind.as_deref(),
                &tags,
                as_of.as_deref(),
                budget,
                limit,
            )
            .await
        }
        Commands::Conflicts => handle_conflicts(project_root).await,
        Commands::Answer {
            question,
            kind,
            budget,
        } => handle_answer(project_root, &question, kind.as_deref(), budget).await,
        Commands::Distill { commits, save } => {
            handle_distill(project_root, commits.as_deref(), save).await
        }
        Commands::SyncNotes { direction } => {
            handle_sync_notes(project_root, direction.as_deref()).await
        }
        Commands::Mcp => {
            let server = McpServer::new(project_root);
            server.run_stdio().await
        }
        Commands::Watch => {
            let watcher = WorkspaceWatcher::new(project_root);
            watcher.run().await
        }
        Commands::Consolidate => handle_consolidate(project_root).await,
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
