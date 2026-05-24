use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use rusqlite::{Connection, OptionalExtension, params};
use sha2::{Digest, Sha256};
use tracing::debug;

use crate::markdown::MarkdownBlockKind;
use crate::vault::{IndexWriteSummary, NoteMetadata, VaultIndex};

pub struct IndexStore {
    conn: Connection,
}

impl IndexStore {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create db dir {}", parent.display()))?;
        }

        let conn = Connection::open(path)
            .with_context(|| format!("failed to open sqlite db {}", path.display()))?;
        let store = Self { conn };
        store.bootstrap()?;
        Ok(store)
    }

    pub fn write_index(&mut self, index: &VaultIndex) -> Result<IndexWriteSummary> {
        let tx = self.conn.transaction()?;
        let mut summary = IndexWriteSummary::default();

        // This is a rebuildable cache, so changed notes get their child rows replaced in place.
        for note in &index.notes {
            summary.notes_seen += 1;
            let existing_hash = existing_note_hash(&tx, &note.path)?;
            if existing_hash.as_deref() == Some(note.content_hash.as_str()) {
                summary.unchanged_notes += 1;
                debug!(path = %note.path.display(), "skipping unchanged note");
                continue;
            }

            summary.changed_notes += 1;
            let note_id = upsert_note(&tx, note)?;
            clear_note_children(&tx, note_id)?;
            insert_chunks(&tx, note_id, note, &mut summary)?;
            insert_tags(&tx, note_id, note, &mut summary)?;
            insert_links(&tx, note_id, note, &mut summary)?;
        }

        tx.commit()?;
        Ok(summary)
    }

    fn bootstrap(&self) -> Result<()> {
        self.conn.execute_batch(
            r#"
            PRAGMA foreign_keys = ON;

            CREATE TABLE IF NOT EXISTS migrations (
                id INTEGER PRIMARY KEY,
                name TEXT NOT NULL UNIQUE,
                applied_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            );

            CREATE TABLE IF NOT EXISTS notes (
                id INTEGER PRIMARY KEY,
                path TEXT NOT NULL UNIQUE,
                filename TEXT NOT NULL,
                title TEXT NOT NULL,
                modified_unix_secs INTEGER,
                file_size INTEGER NOT NULL,
                content_hash TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            );

            CREATE TABLE IF NOT EXISTS chunks (
                id INTEGER PRIMARY KEY,
                note_id INTEGER NOT NULL,
                chunk_index INTEGER NOT NULL,
                heading_path TEXT NOT NULL,
                content TEXT NOT NULL,
                chunk_type TEXT NOT NULL,
                start_line INTEGER NOT NULL,
                end_line INTEGER NOT NULL,
                token_estimate INTEGER NOT NULL,
                content_hash TEXT NOT NULL,
                FOREIGN KEY(note_id) REFERENCES notes(id) ON DELETE CASCADE,
                UNIQUE(note_id, chunk_index)
            );

            CREATE TABLE IF NOT EXISTS tags (
                id INTEGER PRIMARY KEY,
                name TEXT NOT NULL UNIQUE
            );

            CREATE TABLE IF NOT EXISTS note_tags (
                note_id INTEGER NOT NULL,
                tag_id INTEGER NOT NULL,
                FOREIGN KEY(note_id) REFERENCES notes(id) ON DELETE CASCADE,
                FOREIGN KEY(tag_id) REFERENCES tags(id) ON DELETE CASCADE,
                PRIMARY KEY(note_id, tag_id)
            );

            CREATE TABLE IF NOT EXISTS links (
                id INTEGER PRIMARY KEY,
                source_note_id INTEGER NOT NULL,
                target TEXT NOT NULL,
                alias TEXT,
                link_type TEXT NOT NULL DEFAULT 'wikilink',
                FOREIGN KEY(source_note_id) REFERENCES notes(id) ON DELETE CASCADE
            );

            INSERT OR IGNORE INTO migrations (id, name) VALUES (1, 'initial_metadata_index');
            "#,
        )?;
        Ok(())
    }
}

fn existing_note_hash(conn: &Connection, path: &Path) -> Result<Option<String>> {
    conn.query_row(
        "SELECT content_hash FROM notes WHERE path = ?1",
        [path_to_db(path)],
        |row| row.get(0),
    )
    .optional()
    .context("failed to read existing note hash")
}

fn upsert_note(conn: &Connection, note: &NoteMetadata) -> Result<i64> {
    conn.execute(
        r#"
        INSERT INTO notes (
            path,
            filename,
            title,
            modified_unix_secs,
            file_size,
            content_hash,
            updated_at
        )
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, CURRENT_TIMESTAMP)
        ON CONFLICT(path) DO UPDATE SET
            filename = excluded.filename,
            title = excluded.title,
            modified_unix_secs = excluded.modified_unix_secs,
            file_size = excluded.file_size,
            content_hash = excluded.content_hash,
            updated_at = CURRENT_TIMESTAMP
        "#,
        params![
            path_to_db(&note.path),
            note.filename,
            note.title,
            note.modified_unix_secs,
            note.file_size,
            note.content_hash,
        ],
    )?;

    conn.query_row(
        "SELECT id FROM notes WHERE path = ?1",
        [path_to_db(&note.path)],
        |row| row.get(0),
    )
    .context("failed to read upserted note id")
}

fn clear_note_children(conn: &Connection, note_id: i64) -> Result<()> {
    conn.execute("DELETE FROM chunks WHERE note_id = ?1", [note_id])?;
    conn.execute("DELETE FROM note_tags WHERE note_id = ?1", [note_id])?;
    conn.execute("DELETE FROM links WHERE source_note_id = ?1", [note_id])?;
    Ok(())
}

fn insert_chunks(
    conn: &Connection,
    note_id: i64,
    note: &NoteMetadata,
    summary: &mut IndexWriteSummary,
) -> Result<()> {
    for (idx, block) in note.blocks.iter().enumerate() {
        if block.text.trim().is_empty() {
            continue;
        }

        conn.execute(
            r#"
            INSERT INTO chunks (
                note_id,
                chunk_index,
                heading_path,
                content,
                chunk_type,
                start_line,
                end_line,
                token_estimate,
                content_hash
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
            "#,
            params![
                note_id,
                idx as i64,
                block.heading_path.join(" > "),
                block.text,
                chunk_type(&block.kind),
                block.start_line as i64,
                block.end_line as i64,
                estimate_tokens(&block.text) as i64,
                sha256_hex(&block.text),
            ],
        )?;
        summary.chunks_written += 1;
    }
    Ok(())
}

fn insert_tags(
    conn: &Connection,
    note_id: i64,
    note: &NoteMetadata,
    summary: &mut IndexWriteSummary,
) -> Result<()> {
    for tag in &note.tags {
        conn.execute("INSERT OR IGNORE INTO tags (name) VALUES (?1)", [tag])?;
        let tag_id: i64 = conn.query_row("SELECT id FROM tags WHERE name = ?1", [tag], |row| {
            row.get(0)
        })?;
        conn.execute(
            "INSERT OR IGNORE INTO note_tags (note_id, tag_id) VALUES (?1, ?2)",
            params![note_id, tag_id],
        )?;
        summary.tags_seen += 1;
    }
    Ok(())
}

fn insert_links(
    conn: &Connection,
    note_id: i64,
    note: &NoteMetadata,
    summary: &mut IndexWriteSummary,
) -> Result<()> {
    for link in &note.wikilinks {
        conn.execute(
            "INSERT INTO links (source_note_id, target, alias, link_type) VALUES (?1, ?2, ?3, 'wikilink')",
            params![note_id, link.target, link.alias],
        )?;
        summary.links_written += 1;
    }
    Ok(())
}

pub fn sha256_hex(content: &str) -> String {
    format!("{:x}", Sha256::digest(content.as_bytes()))
}

fn estimate_tokens(content: &str) -> usize {
    content.split_whitespace().count().max(1)
}

fn chunk_type(kind: &MarkdownBlockKind) -> &'static str {
    match kind {
        MarkdownBlockKind::Heading => "heading",
        MarkdownBlockKind::Paragraph => "paragraph",
        MarkdownBlockKind::CodeBlock => "code_block",
        MarkdownBlockKind::List => "list",
    }
}

fn path_to_db(path: &Path) -> String {
    PathBuf::from(path).to_string_lossy().replace('\\', "/")
}
