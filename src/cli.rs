use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};

#[derive(Debug, Parser)]
#[command(name = "glassmind")]
#[command(about = "Local-first retrieval over markdown vaults")]
#[command(version)]
pub struct Cli {
    /// Path to glassmind.toml.
    #[arg(long, global = true)]
    pub config: Option<PathBuf>,

    /// Override the vault path from config.
    #[arg(long, global = true)]
    pub vault: Option<PathBuf>,

    /// Enable debug logging.
    #[arg(long, global = true)]
    pub debug: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Create a starter config and the agent-owned workspace.
    Init {
        /// Overwrite an existing glassmind.toml.
        #[arg(long)]
        force: bool,
    },
    /// Scan the configured vault and report discovered markdown notes.
    Index {
        /// Emit JSON instead of text.
        #[arg(long)]
        json: bool,
        /// Generate missing embeddings after writing chunks.
        #[arg(long)]
        embeddings: bool,
        /// Poll and reindex the vault every few seconds.
        #[arg(long)]
        watch: bool,
    },
    /// Search the current markdown vault with lightweight local matching.
    Search {
        query: String,
        #[arg(short, long, default_value_t = 10)]
        limit: usize,
        #[arg(long)]
        debug_scores: bool,
        #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
        output: OutputFormat,
    },
    /// Build a human-readable context bundle from matching notes.
    Context {
        query: String,
        #[arg(short, long, default_value_t = 5)]
        limit: usize,
        #[arg(long, default_value_t = 6000)]
        budget: usize,
        #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
        output: OutputFormat,
    },
    /// Start the future localhost HTTP API.
    Serve,
    /// Print simple MCP tool metadata.
    Mcp {
        #[command(subcommand)]
        command: McpCommand,
    },
    /// Append generated markdown into the agent-owned workspace.
    Capture {
        #[command(subcommand)]
        kind: CaptureKind,
    },
    /// Show vault scan metrics.
    Stats {
        /// Emit JSON instead of text.
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Subcommand)]
pub enum McpCommand {
    Tools,
    Search {
        query: String,
        #[arg(short, long, default_value_t = 10)]
        limit: usize,
    },
    Context {
        query: String,
        #[arg(short, long, default_value_t = 5)]
        limit: usize,
    },
    Read {
        path: String,
    },
}

#[derive(Debug, Subcommand)]
pub enum CaptureKind {
    Memory(CaptureArgs),
    Task(CaptureArgs),
    Decision(CaptureArgs),
}

#[derive(Clone, Debug, clap::Args)]
pub struct CaptureArgs {
    #[arg(long, default_value = "general")]
    pub project: String,
    #[arg(long)]
    pub text: String,
}

#[derive(Clone, Debug, ValueEnum)]
pub enum OutputFormat {
    Text,
    Json,
}
