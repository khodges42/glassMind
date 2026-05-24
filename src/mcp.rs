use anyhow::Result;
use serde::Serialize;

use crate::config::Config;
use crate::db::IndexStore;
use crate::embedding::backend_from_config;

#[derive(Serialize)]
struct ToolSpec {
    name: &'static str,
    description: &'static str,
}

pub fn print_tools() -> Result<()> {
    let tools = vec![
        ToolSpec {
            name: "glassmind_search",
            description: "Search indexed markdown chunks.",
        },
        ToolSpec {
            name: "glassmind_context",
            description: "Build a compact context bundle from markdown chunks.",
        },
        ToolSpec {
            name: "glassmind_read",
            description: "Read a note by vault-relative path.",
        },
    ];
    println!("{}", serde_json::to_string_pretty(&tools)?);
    Ok(())
}

pub fn search(config: &Config, query: &str, limit: usize) -> Result<()> {
    let store = IndexStore::open(&config.vault.path.join(&config.database.path))?;
    let backend = backend_from_config(config);
    let hits = store.hybrid_search(query, limit, backend.as_ref(), config)?;
    println!("{}", serde_json::to_string_pretty(&hits)?);
    Ok(())
}

pub fn read(config: &Config, path: &str) -> Result<()> {
    let path = config.vault.path.join(path);
    let content = std::fs::read_to_string(&path)?;
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "path": path.display().to_string(),
            "content": content
        }))?
    );
    Ok(())
}
