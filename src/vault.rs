use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use anyhow::{Context, Result};
use serde::Serialize;
use tracing::{debug, warn};
use walkdir::{DirEntry, WalkDir};

use crate::config::Config;
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
    pub headings: Vec<String>,
    pub blocks: Vec<MarkdownBlock>,
    pub wikilinks: Vec<Wikilink>,
}

#[derive(Clone, Debug, Serialize)]
pub struct IndexSummary {
    pub vault_path: PathBuf,
    pub notes_indexed: usize,
    pub markdown_files: usize,
    pub headings: usize,
    pub blocks: usize,
    pub wikilinks: usize,
    pub skipped_dirs: Vec<PathBuf>,
}

#[derive(Clone, Debug, Serialize)]
pub struct SearchResult {
    pub note: NoteMetadata,
    pub score: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct ContextBundle {
    pub query: String,
    pub sources: Vec<SearchResult>,
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

            let note = read_note(entry.path(), &config.vault.path)?;
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
            wikilinks: self.notes.iter().map(|note| note.wikilinks.len()).sum(),
            skipped_dirs: self.skipped_dirs.clone(),
        }
    }

    pub fn search(&self, query: &str, limit: usize) -> Vec<SearchResult> {
        let terms = query_terms(query);
        let mut results: Vec<_> = self
            .notes
            .iter()
            .filter_map(|note| {
                let haystack = format!(
                    "{} {} {}",
                    note.path.display(),
                    note.title,
                    note.blocks
                        .iter()
                        .map(|block| block.text.as_str())
                        .collect::<Vec<_>>()
                        .join(" ")
                )
                .to_lowercase();
                let score = terms
                    .iter()
                    .filter(|term| haystack.contains(term.as_str()))
                    .count();
                (score > 0).then(|| SearchResult {
                    note: note.clone(),
                    score,
                })
            })
            .collect();

        results.sort_by(|a, b| {
            b.score
                .cmp(&a.score)
                .then_with(|| a.note.path.cmp(&b.note.path))
        });
        results.truncate(limit);
        results
    }

    pub fn context_bundle(&self, query: &str, limit: usize) -> ContextBundle {
        ContextBundle {
            query: query.to_string(),
            sources: self.search(query, limit),
        }
    }
}

impl ContextBundle {
    pub fn to_markdown(&self) -> String {
        let mut out = format!("# Glassmind Context\n\nQuery: `{}`\n\n", self.query);
        if self.sources.is_empty() {
            out.push_str("No matching markdown notes were found.\n");
            return out;
        }

        out.push_str("## Sources\n\n");
        for (idx, result) in self.sources.iter().enumerate() {
            out.push_str(&format!(
                "{}. `{}` - score {}\n",
                idx + 1,
                result.note.path.display(),
                result.score
            ));
            out.push_str(&format!("   - title: {}\n", result.note.title));
            if !result.note.headings.is_empty() {
                out.push_str(&format!(
                    "   - headings: {}\n",
                    result.note.headings.join(" > ")
                ));
            }
            if !result.note.wikilinks.is_empty() {
                let links = result
                    .note
                    .wikilinks
                    .iter()
                    .map(|link| match &link.alias {
                        Some(alias) => format!("{} as {}", link.target, alias),
                        None => link.target.clone(),
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                out.push_str(&format!("   - wikilinks: {links}\n"));
            }
        }
        out
    }
}

impl fmt::Display for IndexSummary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Vault: {}", self.vault_path.display())?;
        writeln!(f, "Notes indexed: {}", self.notes_indexed)?;
        writeln!(f, "Markdown files: {}", self.markdown_files)?;
        writeln!(f, "Headings parsed: {}", self.headings)?;
        writeln!(f, "Markdown blocks: {}", self.blocks)?;
        writeln!(f, "Wikilinks: {}", self.wikilinks)?;
        writeln!(f, "Skipped dirs: {}", self.skipped_dirs.len())
    }
}

fn read_note(path: &Path, vault_path: &Path) -> Result<NoteMetadata> {
    let content =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    let metadata =
        fs::metadata(path).with_context(|| format!("failed to stat {}", path.display()))?;
    let relative_path = path.strip_prefix(vault_path).unwrap_or(path).to_path_buf();
    let source_path = relative_path.to_string_lossy().replace('\\', "/");
    let parsed = parse_markdown(&source_path, &content);

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
        headings: parsed.headings,
        blocks: parsed.blocks,
        wikilinks: parsed.wikilinks,
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

fn query_terms(query: &str) -> Vec<String> {
    query
        .split_whitespace()
        .map(|term| {
            term.trim_matches(|c: char| !c.is_alphanumeric())
                .to_lowercase()
        })
        .filter(|term| !term.is_empty())
        .collect()
}
