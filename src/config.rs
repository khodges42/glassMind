use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow, bail};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Config {
    pub vault: VaultConfig,
    pub database: DatabaseConfig,
    pub index: IndexConfig,
    pub embeddings: EmbeddingsConfig,
    pub search: SearchConfig,
    pub writes: WritesConfig,
    pub server: ServerConfig,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct VaultConfig {
    pub path: PathBuf,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DatabaseConfig {
    pub path: PathBuf,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct IndexConfig {
    pub include_agent_dir: bool,
    pub ignore_dirs: Vec<String>,
    pub chunk_target_tokens: usize,
    pub chunk_overlap_tokens: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct EmbeddingsConfig {
    pub backend: String,
    pub model: String,
    pub url: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SearchConfig {
    pub semantic_weight: f32,
    pub keyword_weight: f32,
    pub recency_weight: f32,
    pub link_weight: f32,
    pub tag_weight: f32,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct WritesConfig {
    pub mode: String,
    pub agent_dir: PathBuf,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

impl Config {
    pub fn load(path: Option<&Path>) -> Result<Self> {
        let path = path
            .map(Path::to_path_buf)
            .unwrap_or_else(Self::default_path);
        if !path.exists() {
            return Ok(Self::default());
        }

        let raw = fs::read_to_string(&path)
            .with_context(|| format!("failed to read config {}", path.display()))?;
        toml::from_str(&raw).with_context(|| format!("invalid config {}", path.display()))
    }

    pub fn default_path() -> PathBuf {
        PathBuf::from("glassmind.toml")
    }

    pub fn with_cli_vault(mut self, vault: Option<PathBuf>) -> Self {
        if let Some(vault) = vault {
            self.vault.path = vault;
        }
        self
    }

    pub fn validate(&self) -> Result<()> {
        if self.vault.path.as_os_str().is_empty() {
            bail!("vault.path must not be empty");
        }
        if self.index.chunk_target_tokens == 0 {
            bail!("index.chunk_target_tokens must be greater than zero");
        }
        if self.index.chunk_overlap_tokens >= self.index.chunk_target_tokens {
            bail!("index.chunk_overlap_tokens must be smaller than index.chunk_target_tokens");
        }
        if self.server.port == 0 {
            bail!("server.port must be greater than zero");
        }
        if self.database.path.as_os_str().is_empty() {
            bail!("database.path must not be empty");
        }
        match self.writes.mode.as_str() {
            "off" | "agent-only" | "propose" | "allow" => {}
            other => {
                bail!("writes.mode must be one of off, agent-only, propose, allow; got {other}")
            }
        }
        Ok(())
    }

    pub fn write_default_file(&self, force: bool) -> Result<()> {
        let path = Self::default_path();
        if path.exists() && !force {
            return Err(anyhow!(
                "{} already exists; pass --force to overwrite it",
                path.display()
            ));
        }

        let raw = toml::to_string_pretty(self).context("failed to serialize default config")?;
        fs::write(&path, raw).with_context(|| format!("failed to write {}", path.display()))
    }

    pub fn create_agent_dirs(&self) -> Result<()> {
        let base = self.vault.path.join(&self.writes.agent_dir);
        for dir in [
            "memories",
            "summaries",
            "tasks",
            "decisions",
            "logs",
            "cache",
        ] {
            fs::create_dir_all(base.join(dir))
                .with_context(|| format!("failed to create {}", base.join(dir).display()))?;
        }
        Ok(())
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            vault: VaultConfig {
                path: PathBuf::from("."),
            },
            database: DatabaseConfig {
                path: PathBuf::from(".agent/cache/glassmind.sqlite3"),
            },
            index: IndexConfig {
                include_agent_dir: true,
                ignore_dirs: vec![
                    ".git".to_string(),
                    ".obsidian".to_string(),
                    ".trash".to_string(),
                    ".agent/cache".to_string(),
                ],
                chunk_target_tokens: 500,
                chunk_overlap_tokens: 80,
            },
            embeddings: EmbeddingsConfig {
                backend: "ollama".to_string(),
                model: "nomic-embed-text".to_string(),
                url: "http://localhost:11434".to_string(),
            },
            search: SearchConfig {
                semantic_weight: 0.55,
                keyword_weight: 0.25,
                recency_weight: 0.10,
                link_weight: 0.05,
                tag_weight: 0.05,
            },
            writes: WritesConfig {
                mode: "agent-only".to_string(),
                agent_dir: PathBuf::from(".agent"),
            },
            server: ServerConfig {
                host: "127.0.0.1".to_string(),
                port: 7331,
            },
        }
    }
}
