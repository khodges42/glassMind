mod chunk;
mod cli;
mod config;
mod db;
mod logging;
mod markdown;
mod vault;

use anyhow::Result;
use clap::Parser;
use tracing::{debug, info};

use crate::cli::{Cli, Commands, OutputFormat};
use crate::config::Config;
use crate::db::{IndexStore, SearchHit};
use crate::vault::VaultIndex;

fn main() -> Result<()> {
    let cli = Cli::parse();
    logging::init(cli.debug)?;

    let config = Config::load(cli.config.as_deref())?.with_cli_vault(cli.vault);
    config.validate()?;

    debug!(?config, "loaded config");

    match cli.command {
        Commands::Init { force } => init_project(&config, force),
        Commands::Index { json } => {
            let index = VaultIndex::scan(&config)?;
            config.create_agent_dirs()?;
            // Indexing writes the rebuildable cache, while search can still scan live markdown.
            let db_path = config.vault.path.join(&config.database.path);
            let mut store = IndexStore::open(&db_path)?;
            let writes = store.write_index(&index)?;
            let summary = index.summary_with_writes(writes);
            if json {
                println!("{}", serde_json::to_string_pretty(&summary)?);
            } else {
                println!("{summary}");
            }
            Ok(())
        }
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
            output,
        } => {
            let db_path = ensure_index_cache(&config)?;
            let store = IndexStore::open(&db_path)?;
            let results = store.search(&query, limit)?;
            match output {
                OutputFormat::Text => {
                    if results.is_empty() {
                        println!("No matches.");
                    }
                    print_search_results(&results);
                }
                OutputFormat::Json => println!("{}", serde_json::to_string_pretty(&results)?),
            }
            Ok(())
        }
        Commands::Context {
            query,
            limit,
            output,
        } => {
            let index = VaultIndex::scan(&config)?;
            let bundle = index.context_bundle(&query, limit);
            match output {
                OutputFormat::Text => println!("{}", bundle.to_markdown()),
                OutputFormat::Json => println!("{}", serde_json::to_string_pretty(&bundle)?),
            }
            Ok(())
        }
        Commands::Serve => {
            info!("serve command is reserved for the HTTP API milestone");
            println!(
                "HTTP API is not implemented yet. Planned bind: {}:{}",
                config.server.host, config.server.port
            );
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

fn print_search_results(results: &[SearchHit]) {
    for (position, result) in results.iter().enumerate() {
        println!("{}. {}", position + 1, result.path);
        println!("   title: {}", result.title);
        if !result.heading_path.is_empty() {
            println!("   heading: {}", result.heading_path);
        }
        println!("   tokens: {}", result.token_estimate);
        println!("   score: {:.4}", result.score);
        println!("   {}", result.snippet);
    }
}
