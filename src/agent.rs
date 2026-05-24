use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;

use anyhow::{Context, Result};

use crate::config::Config;

pub fn capture(config: &Config, kind: &str, project: &str, text: &str) -> Result<PathBuf> {
    config.create_agent_dirs()?;
    let folder = match kind {
        "task" => "tasks",
        "decision" => "decisions",
        _ => "memories",
    };
    let path = config
        .vault
        .path
        .join(&config.writes.agent_dir)
        .join(folder)
        .join(format!("{}.md", slug(project)));

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .with_context(|| format!("failed to open {}", path.display()))?;

    // Agent notes are markdown on purpose, so humans can read and edit them later.
    writeln!(file, "\n## {}\n\n{}\n", timestamp(), text)?;
    append_audit(config, kind, project, text)?;
    Ok(path)
}

fn append_audit(config: &Config, kind: &str, project: &str, text: &str) -> Result<()> {
    let path = config
        .vault
        .path
        .join(&config.writes.agent_dir)
        .join("logs")
        .join("memory-events.md");
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    writeln!(
        file,
        "- {} `{}` `{}`: {}",
        timestamp(),
        kind,
        project,
        text.replace('\n', " ")
    )?;
    Ok(())
}

fn slug(input: &str) -> String {
    input
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

fn timestamp() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs());
    format!("unix-{secs}")
}
