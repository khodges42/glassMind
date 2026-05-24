mod agent;
mod chunk;
mod cli;
mod config;
mod context;
mod db;
mod embedding;
mod logging;
mod markdown;
mod mcp;
mod server;
mod vault;

use anyhow::Result;
use clap::Parser;
use tracing::{debug, info};

use crate::cli::{CaptureKind, Cli, Commands, McpCommand, OutputFormat};
use crate::config::Config;
use crate::context::ContextBundle;
use crate::db::{IndexStore, SearchHit};
use crate::embedding::backend_from_config;
use crate::vault::VaultIndex;

fn main() -> Result<()> {
    let cli = Cli::parse();
    logging::init(cli.debug)?;

    let config = Config::load(cli.config.as_deref())?.with_cli_vault(cli.vault);
    config.validate()?;

    debug!(?config, "loaded config");

    match cli.command {
        Commands::Init { force } => init_project(&config, force),
        Commands::Index {
            json,
            embeddings,
            watch,
        } => run_index(&config, json, embeddings, watch),
        Commands::Stats { json } => {
            let index = VaultIndex::scan(&config)?;
            if json {
                println!("{}", serde_json::to_string_pretty(&index.summary())?);
            } else {
                println!("{}", index.summary());
            }
            Ok(())
        }
        Commands::Search {
            query,
            limit,
            debug_scores,
            output,
        } => {
            let db_path = ensure_index_cache(&config)?;
            let mut store = IndexStore::open(&db_path)?;
            let backend = backend_from_config(&config);
            store.generate_embeddings(backend.as_ref())?;
            let results = store.hybrid_search(&query, limit, backend.as_ref(), &config)?;
            match output {
                OutputFormat::Text => {
                    if results.is_empty() {
                        println!("No matches.");
                    }
                    print_search_results(&results, debug_scores);
                }
                OutputFormat::Json => println!("{}", serde_json::to_string_pretty(&results)?),
            }
            Ok(())
        }
        Commands::Context {
            query,
            limit,
            budget,
            output,
        } => {
            let db_path = ensure_index_cache(&config)?;
            let mut store = IndexStore::open(&db_path)?;
            let backend = backend_from_config(&config);
            store.generate_embeddings(backend.as_ref())?;
            let hits = store.hybrid_search(&query, limit, backend.as_ref(), &config)?;
            let bundle = ContextBundle::from_hits(&query, budget, hits);
            match output {
                OutputFormat::Text => println!("{}", bundle.to_markdown()),
                OutputFormat::Json => println!("{}", serde_json::to_string_pretty(&bundle)?),
            }
            Ok(())
        }
        Commands::Serve => {
            ensure_index_cache(&config)?;
            server::serve(&config)
        }
        Commands::Mcp { command } => match command {
            McpCommand::Tools => mcp::print_tools(),
            McpCommand::Search { query, limit } => {
                ensure_index_cache(&config)?;
                mcp::search(&config, &query, limit)
            }
            McpCommand::Context { query, limit } => {
                ensure_index_cache(&config)?;
                let mut store = IndexStore::open(&config.vault.path.join(&config.database.path))?;
                let backend = backend_from_config(&config);
                store.generate_embeddings(backend.as_ref())?;
                let hits = store.hybrid_search(&query, limit, backend.as_ref(), &config)?;
                let bundle = ContextBundle::from_hits(&query, 6000, hits);
                println!("{}", serde_json::to_string_pretty(&bundle)?);
                Ok(())
            }
            McpCommand::Read { path } => mcp::read(&config, &path),
        },
        Commands::Capture { kind } => {
            let (kind_name, args) = match kind {
                CaptureKind::Memory(args) => ("memory", args),
                CaptureKind::Task(args) => ("task", args),
                CaptureKind::Decision(args) => ("decision", args),
            };
            let path = agent::capture(&config, kind_name, &args.project, &args.text)?;
            println!("Captured {kind_name} at {}", path.display());
            Ok(())
        }
    }
}

fn init_project(config: &Config, force: bool) -> Result<()> {
    config.write_default_file(force)?;
    config.create_agent_dirs()?;
    println!("Initialized Glassmind at {}", config.vault.path.display());
    println!("Config: {}", Config::default_path().display());
    Ok(())
}

fn run_index(config: &Config, json: bool, embeddings: bool, watch: bool) -> Result<()> {
    loop {
        let index = VaultIndex::scan(config)?;
        config.create_agent_dirs()?;
        // Indexing writes the rebuildable cache. Deleting it is always allowed.
        let db_path = config.vault.path.join(&config.database.path);
        let mut store = IndexStore::open(&db_path)?;
        let writes = store.write_index(&index)?;
        if embeddings {
            let backend = backend_from_config(config);
            let written = store.generate_embeddings(backend.as_ref())?;
            info!(written, "generated embeddings");
        }
        let summary = index.summary_with_writes(writes);
        if json {
            println!("{}", serde_json::to_string_pretty(&summary)?);
        } else {
            println!("{summary}");
        }

        if !watch {
            return Ok(());
        }
        std::thread::sleep(std::time::Duration::from_secs(5));
    }
}

fn ensure_index_cache(config: &Config) -> Result<std::path::PathBuf> {
    let db_path = config.vault.path.join(&config.database.path);
    if db_path.exists() {
        return Ok(db_path);
    }

    let index = VaultIndex::scan(config)?;
    config.create_agent_dirs()?;
    let mut store = IndexStore::open(&db_path)?;
    store.write_index(&index)?;
    Ok(db_path)
}

fn print_search_results(results: &[SearchHit], debug_scores: bool) {
    for (position, result) in results.iter().enumerate() {
        println!("{}. {}", position + 1, result.path);
        println!("   title: {}", result.title);
        if !result.heading_path.is_empty() {
            println!("   heading: {}", result.heading_path);
        }
        println!("   tokens: {}", result.token_estimate);
        println!("   score: {:.4}", result.score);
        if debug_scores {
            println!(
                "   keyword {:.4}, semantic {:.4}, recency {:.4}, tags {:.4}, links {:.4}",
                result.keyword_score,
                result.semantic_score,
                result.recency_score,
                result.tag_score,
                result.link_score
            );
        }
        println!("   {}", result.snippet);
    }
}
