use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use anyhow::{Context, Result};
use serde::Serialize;
use tracing::{debug, warn};
use walkdir::{DirEntry, WalkDir};

use crate::chunk::{NoteChunk, build_chunks};
use crate::config::Config;
use crate::db::sha256_hex;
use crate::markdown::{MarkdownBlock, Wikilink, parse_markdown};

#[derive(Clone, Debug, Serialize)]
pub struct VaultIndex {
    pub vault_path: PathBuf,
    pub notes: Vec<NoteMetadata>,
    pub markdown_count: usize,
    pub skipped_dirs: Vec<PathBuf>,
}

#[derive(Clone, Debug, Serialize)]
pub struct NoteMetadata {
    pub path: PathBuf,
    pub filename: String,
    pub title: String,
    pub modified_unix_secs: Option<u64>,
    pub file_size: u64,
    pub content_hash: String,
    pub headings: Vec<String>,
    pub blocks: Vec<MarkdownBlock>,
    pub chunks: Vec<NoteChunk>,
    pub wikilinks: Vec<Wikilink>,
    pub tags: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct IndexSummary {
    pub vault_path: PathBuf,
    pub notes_indexed: usize,
    pub markdown_files: usize,
    pub headings: usize,
    pub blocks: usize,
    pub chunks: usize,
    pub wikilinks: usize,
    pub tags: usize,
    pub skipped_dirs: Vec<PathBuf>,
    pub writes: Option<IndexWriteSummary>,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct IndexWriteSummary {
    pub notes_seen: usize,
    pub changed_notes: usize,
    pub unchanged_notes: usize,
    pub chunks_written: usize,
    pub tags_seen: usize,
    pub links_written: usize,
}

impl VaultIndex {
    pub fn scan(config: &Config) -> Result<Self> {
        let vault_path = config
            .vault
            .path
            .canonicalize()
            .unwrap_or_else(|_| config.vault.path.clone());
        let mut notes = Vec::new();
        let mut skipped_dirs = Vec::new();

        let walker = WalkDir::new(&config.vault.path)
            .follow_links(false)
            .into_iter()
            .filter_entry(|entry| {
                should_enter(entry, &config.vault.path, config, &mut skipped_dirs)
            });

        for entry in walker {
            let entry = match entry {
                Ok(entry) => entry,
                Err(err) => {
                    warn!("skipping unreadable path: {err}");
                    continue;
                }
            };

            if !entry.file_type().is_file() || !is_markdown(entry.path()) {
                continue;
            }

            let note = read_note(entry.path(), &config.vault.path, config)?;
            debug!(
                path = %note.path.display(),
                title = %note.title,
                size = note.file_size,
                headings = note.headings.len(),
                links = note.wikilinks.len(),
                "indexed note metadata"
            );
            notes.push(note);
        }

        notes.sort_by(|a, b| a.path.cmp(&b.path));
        let markdown_count = notes.len();

        Ok(Self {
            vault_path,
            notes,
            markdown_count,
            skipped_dirs,
        })
    }

    pub fn summary(&self) -> IndexSummary {
        IndexSummary {
            vault_path: self.vault_path.clone(),
            notes_indexed: self.notes.len(),
            markdown_files: self.markdown_count,
            headings: self.notes.iter().map(|note| note.headings.len()).sum(),
            blocks: self.notes.iter().map(|note| note.blocks.len()).sum(),
            chunks: self.notes.iter().map(|note| note.chunks.len()).sum(),
            wikilinks: self.notes.iter().map(|note| note.wikilinks.len()).sum(),
            tags: self.notes.iter().map(|note| note.tags.len()).sum(),
            skipped_dirs: self.skipped_dirs.clone(),
            writes: None,
        }
    }

    pub fn summary_with_writes(&self, writes: IndexWriteSummary) -> IndexSummary {
        IndexSummary {
            writes: Some(writes),
            ..self.summary()
        }
    }
}

impl fmt::Display for IndexSummary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Vault: {}", self.vault_path.display())?;
        writeln!(f, "Notes indexed: {}", self.notes_indexed)?;
        writeln!(f, "Markdown files: {}", self.markdown_files)?;
        writeln!(f, "Headings parsed: {}", self.headings)?;
        writeln!(f, "Markdown blocks: {}", self.blocks)?;
        writeln!(f, "Chunks: {}", self.chunks)?;
        writeln!(f, "Wikilinks: {}", self.wikilinks)?;
        writeln!(f, "Tags: {}", self.tags)?;
        writeln!(f, "Skipped dirs: {}", self.skipped_dirs.len())?;
        if let Some(writes) = &self.writes {
            writeln!(f, "Changed notes: {}", writes.changed_notes)?;
            writeln!(f, "Unchanged notes skipped: {}", writes.unchanged_notes)?;
            writeln!(f, "Chunks written: {}", writes.chunks_written)?;
        }
        Ok(())
    }
}

fn read_note(path: &Path, vault_path: &Path, config: &Config) -> Result<NoteMetadata> {
    let content =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    let metadata =
        fs::metadata(path).with_context(|| format!("failed to stat {}", path.display()))?;
    let relative_path = path.strip_prefix(vault_path).unwrap_or(path).to_path_buf();
    let source_path = relative_path.to_string_lossy().replace('\\', "/");
    let parsed = parse_markdown(&source_path, &content);
    let chunks = build_chunks(
        &parsed.blocks,
        config.index.chunk_target_tokens,
        config.index.chunk_overlap_tokens,
    );
    let content_hash = sha256_hex(&content);

    Ok(NoteMetadata {
        path: relative_path,
        filename: path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default()
            .to_string(),
        title: extract_title(path, &parsed.headings),
        modified_unix_secs: metadata
            .modified()
            .ok()
            .and_then(|modified| modified.duration_since(UNIX_EPOCH).ok())
            .map(|duration| duration.as_secs()),
        file_size: metadata.len(),
        content_hash,
        headings: parsed.headings,
        blocks: parsed.blocks,
        chunks,
        wikilinks: parsed.wikilinks,
        tags: parsed.tags,
    })
}

fn extract_title(path: &Path, headings: &[String]) -> String {
    headings.first().cloned().unwrap_or_else(|| {
        path.file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or("Untitled")
            .to_string()
    })
}

fn should_enter(
    entry: &DirEntry,
    vault_path: &Path,
    config: &Config,
    skipped_dirs: &mut Vec<PathBuf>,
) -> bool {
    if !entry.file_type().is_dir() {
        return true;
    }

    let relative = entry
        .path()
        .strip_prefix(vault_path)
        .unwrap_or(entry.path());
    if relative.as_os_str().is_empty() {
        return true;
    }

    let normalized = relative.to_string_lossy().replace('\\', "/");
    let ignored = config
        .index
        .ignore_dirs
        .iter()
        .any(|ignore| normalized == *ignore || normalized.starts_with(&format!("{ignore}/")));

    let agent_excluded = !config.index.include_agent_dir
        && normalized
            .split('/')
            .next()
            .is_some_and(|component| component == config.writes.agent_dir.to_string_lossy());

    if ignored || agent_excluded {
        skipped_dirs.push(relative.to_path_buf());
        false
    } else {
        true
    }
}

fn is_markdown(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("md"))
}
