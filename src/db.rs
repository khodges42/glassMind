use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use rusqlite::{Connection, OptionalExtension, params};
use sha2::{Digest, Sha256};
use tracing::debug;

use crate::chunk::chunk_type_name;
use crate::embedding::{EmbeddingBackend, cosine_similarity};
use crate::vault::{IndexWriteSummary, NoteMetadata, VaultIndex};

const INDEX_VERSION: i64 = 3;

pub struct IndexStore {
    conn: Connection,
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct SearchHit {
    pub chunk_id: i64,
    pub path: String,
    pub title: String,
    pub heading_path: String,
    pub snippet: String,
    pub score: f64,
    pub keyword_score: f64,
    pub semantic_score: f64,
    pub recency_score: f64,
    pub link_score: f64,
    pub tag_score: f64,
    pub token_estimate: usize,
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
            let fresh = existing_note_fresh(&tx, &note.path, &note.content_hash)?;
            if fresh {
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

        delete_missing_notes(&tx, index)?;
        rebuild_fts_if_empty(&tx)?;
        tx.commit()?;
        Ok(summary)
    }

    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchHit>> {
        let fts_query = fts_query(query);
        if fts_query.is_empty() {
            return Ok(Vec::new());
        }

        let mut stmt = self.conn.prepare(
            r#"
            SELECT
                chunks.id,
                notes.path,
                notes.title,
                chunks.heading_path,
                snippet(chunks_fts, 0, '[', ']', '...', 18) AS snippet,
                bm25(chunks_fts) AS score,
                chunks.token_estimate
            FROM chunks_fts
            JOIN chunks ON chunks.id = chunks_fts.rowid
            JOIN notes ON notes.id = chunks.note_id
            WHERE chunks_fts MATCH ?1
            ORDER BY score
            LIMIT ?2
            "#,
        )?;

        let hits = stmt
            .query_map(params![fts_query, limit as i64], |row| {
                Ok(SearchHit {
                    chunk_id: row.get(0)?,
                    path: row.get(1)?,
                    title: row.get(2)?,
                    heading_path: row.get(3)?,
                    snippet: row.get(4)?,
                    score: -row.get::<_, f64>(5)?,
                    keyword_score: -row.get::<_, f64>(5)?,
                    semantic_score: 0.0,
                    recency_score: 0.0,
                    link_score: 0.0,
                    tag_score: 0.0,
                    token_estimate: row.get::<_, i64>(6)? as usize,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        Ok(hits)
    }

    pub fn hybrid_search(
        &self,
        query: &str,
        limit: usize,
        backend: &dyn EmbeddingBackend,
        config: &crate::config::Config,
    ) -> Result<Vec<SearchHit>> {
        let mut hits = self.search(query, limit.saturating_mul(3).max(limit))?;
        let query_embedding = backend.embed(query)?;

        for hit in &mut hits {
            if let Some(vector) = self.embedding_for_chunk(hit.chunk_id, backend.model())? {
                hit.semantic_score = f64::from(cosine_similarity(&query_embedding.vector, &vector));
            }
            hit.recency_score = self.recency_score(hit.chunk_id)?;
            hit.link_score = self.link_score(&hit.path)?;
            hit.tag_score = self.tag_score(&hit.path, query)?;
            hit.score = hit.keyword_score * f64::from(config.search.keyword_weight)
                + hit.semantic_score * f64::from(config.search.semantic_weight)
                + hit.recency_score * f64::from(config.search.recency_weight)
                + hit.link_score * f64::from(config.search.link_weight)
                + hit.tag_score * f64::from(config.search.tag_weight);
        }

        hits.sort_by(|a, b| b.score.total_cmp(&a.score));
        hits.truncate(limit);
        self.audit_retrieval(query, &hits)?;
        Ok(hits)
    }

    pub fn generate_embeddings(&mut self, backend: &dyn EmbeddingBackend) -> Result<usize> {
        let tx = self.conn.transaction()?;
        let mut stmt = tx.prepare(
            r#"
            SELECT chunks.id, chunks.content
            FROM chunks
            LEFT JOIN embeddings
                ON embeddings.chunk_id = chunks.id
                AND embeddings.model = ?1
            WHERE embeddings.chunk_id IS NULL
            "#,
        )?;
        let pending = stmt
            .query_map([backend.model()], |row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        drop(stmt);

        let mut written = 0;
        for (chunk_id, content) in pending {
            let embedding = backend.embed(&content)?;
            tx.execute(
                "INSERT OR REPLACE INTO embeddings (chunk_id, model, dimensions, vector, created_at) VALUES (?1, ?2, ?3, ?4, CURRENT_TIMESTAMP)",
                params![
                    chunk_id,
                    embedding.model,
                    embedding.vector.len() as i64,
                    serde_json::to_string(&embedding.vector)?,
                ],
            )?;
            written += 1;
        }

        tx.commit()?;
        Ok(written)
    }

    pub fn stats(&self) -> Result<StoreStats> {
        Ok(StoreStats {
            notes: count(&self.conn, "notes")?,
            chunks: count(&self.conn, "chunks")?,
            tags: count(&self.conn, "tags")?,
            links: count(&self.conn, "links")?,
            embeddings: count(&self.conn, "embeddings")?,
        })
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
                index_version INTEGER NOT NULL DEFAULT 3,
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

            CREATE VIRTUAL TABLE IF NOT EXISTS chunks_fts
            USING fts5(
                content,
                path UNINDEXED,
                title UNINDEXED,
                heading_path UNINDEXED
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

            CREATE TABLE IF NOT EXISTS embeddings (
                chunk_id INTEGER NOT NULL,
                model TEXT NOT NULL,
                dimensions INTEGER NOT NULL,
                vector TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY(chunk_id) REFERENCES chunks(id) ON DELETE CASCADE,
                PRIMARY KEY(chunk_id, model)
            );

            CREATE TABLE IF NOT EXISTS retrieval_audit (
                id INTEGER PRIMARY KEY,
                query TEXT NOT NULL,
                result_paths TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                client TEXT NOT NULL DEFAULT 'cli'
            );

            CREATE TABLE IF NOT EXISTS memory_events (
                id INTEGER PRIMARY KEY,
                event_type TEXT NOT NULL,
                source TEXT NOT NULL,
                content TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            );

            INSERT OR IGNORE INTO migrations (id, name) VALUES (1, 'initial_metadata_index');
            "#,
        )?;
        ensure_notes_index_version(&self.conn)?;
        Ok(())
    }
}

fn delete_missing_notes(conn: &Connection, index: &VaultIndex) -> Result<()> {
    let current = index
        .notes
        .iter()
        .map(|note| path_to_db(&note.path))
        .collect::<std::collections::BTreeSet<_>>();
    let mut stmt = conn.prepare("SELECT id, path FROM notes")?;
    let existing = stmt
        .query_map([], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    drop(stmt);

    for (note_id, path) in existing {
        if !current.contains(&path) {
            clear_note_children(conn, note_id)?;
            conn.execute("DELETE FROM notes WHERE id = ?1", [note_id])?;
        }
    }
    Ok(())
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct StoreStats {
    pub notes: i64,
    pub chunks: i64,
    pub tags: i64,
    pub links: i64,
    pub embeddings: i64,
}

fn existing_note_fresh(conn: &Connection, path: &Path, content_hash: &str) -> Result<bool> {
    let existing = conn
        .query_row(
            "SELECT content_hash, index_version FROM notes WHERE path = ?1",
            [path_to_db(path)],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)),
        )
        .optional()
        .context("failed to read existing note freshness")?;

    Ok(existing.is_some_and(|(hash, version)| hash == content_hash && version == INDEX_VERSION))
}

fn ensure_notes_index_version(conn: &Connection) -> Result<()> {
    let mut stmt = conn.prepare("PRAGMA table_info(notes)")?;
    let columns = stmt
        .query_map([], |row| row.get::<_, String>(1))?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    if !columns.iter().any(|column| column == "index_version") {
        conn.execute(
            "ALTER TABLE notes ADD COLUMN index_version INTEGER NOT NULL DEFAULT 1",
            [],
        )
        .context("failed to add notes.index_version")?;
    }

    Ok(())
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
            index_version,
            updated_at
        )
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, CURRENT_TIMESTAMP)
        ON CONFLICT(path) DO UPDATE SET
            filename = excluded.filename,
            title = excluded.title,
            modified_unix_secs = excluded.modified_unix_secs,
            file_size = excluded.file_size,
            content_hash = excluded.content_hash,
            index_version = excluded.index_version,
            updated_at = CURRENT_TIMESTAMP
        "#,
        params![
            path_to_db(&note.path),
            note.filename,
            note.title,
            note.modified_unix_secs,
            note.file_size,
            note.content_hash,
            INDEX_VERSION,
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
    delete_note_fts(conn, note_id)?;
    conn.execute(
        "DELETE FROM embeddings WHERE chunk_id IN (SELECT id FROM chunks WHERE note_id = ?1)",
        [note_id],
    )?;
    conn.execute("DELETE FROM chunks WHERE note_id = ?1", [note_id])?;
    conn.execute("DELETE FROM note_tags WHERE note_id = ?1", [note_id])?;
    conn.execute("DELETE FROM links WHERE source_note_id = ?1", [note_id])?;
    Ok(())
}

impl IndexStore {
    fn embedding_for_chunk(&self, chunk_id: i64, model: &str) -> Result<Option<Vec<f32>>> {
        self.conn
            .query_row(
                "SELECT vector FROM embeddings WHERE chunk_id = ?1 AND model = ?2",
                params![chunk_id, model],
                |row| row.get::<_, String>(0),
            )
            .optional()?
            .map(|raw| serde_json::from_str(&raw).context("invalid stored embedding vector"))
            .transpose()
    }

    fn recency_score(&self, chunk_id: i64) -> Result<f64> {
        let modified: Option<i64> = self
            .conn
            .query_row(
                "SELECT notes.modified_unix_secs FROM chunks JOIN notes ON notes.id = chunks.note_id WHERE chunks.id = ?1",
                [chunk_id],
                |row| row.get(0),
            )
            .optional()?
            .flatten();
        let Some(modified) = modified else {
            return Ok(0.0);
        };
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |duration| duration.as_secs() as i64);
        let age_days = ((now - modified).max(0) as f64) / 86_400.0;
        Ok(1.0 / (1.0 + age_days / 30.0))
    }

    fn link_score(&self, path: &str) -> Result<f64> {
        let stem = Path::new(path)
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or(path);
        let count: i64 = self.conn.query_row(
            "SELECT count(*) FROM links WHERE lower(target) LIKE '%' || lower(?1) || '%'",
            [stem],
            |row| row.get(0),
        )?;
        Ok((count as f64).min(5.0) / 5.0)
    }

    fn tag_score(&self, path: &str, query: &str) -> Result<f64> {
        let query = query.to_lowercase();
        let mut stmt = self.conn.prepare(
            r#"
            SELECT tags.name
            FROM tags
            JOIN note_tags ON note_tags.tag_id = tags.id
            JOIN notes ON notes.id = note_tags.note_id
            WHERE notes.path = ?1
            "#,
        )?;
        let tags = stmt
            .query_map([path], |row| row.get::<_, String>(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        if tags.is_empty() {
            return Ok(0.0);
        }
        let matches = tags
            .iter()
            .filter(|tag| query.contains(tag.as_str()))
            .count();
        Ok(matches as f64 / tags.len() as f64)
    }

    pub fn audit_retrieval(&self, query: &str, hits: &[SearchHit]) -> Result<()> {
        let paths = hits.iter().map(|hit| hit.path.clone()).collect::<Vec<_>>();
        self.conn.execute(
            "INSERT INTO retrieval_audit (query, result_paths, client) VALUES (?1, ?2, 'cli')",
            params![query, serde_json::to_string(&paths)?],
        )?;
        Ok(())
    }
}

fn insert_chunks(
    conn: &Connection,
    note_id: i64,
    note: &NoteMetadata,
    summary: &mut IndexWriteSummary,
) -> Result<()> {
    for chunk in &note.chunks {
        if chunk.content.trim().is_empty() {
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
                chunk.index as i64,
                chunk.heading_path.join(" > "),
                chunk.content,
                chunk_type_name(&chunk.chunk_type),
                chunk.start_line as i64,
                chunk.end_line as i64,
                chunk.token_estimate as i64,
                chunk.content_hash,
            ],
        )?;
        let chunk_id = conn.last_insert_rowid();
        insert_chunk_fts(conn, chunk_id, note, chunk)?;
        summary.chunks_written += 1;
    }
    Ok(())
}

fn insert_chunk_fts(
    conn: &Connection,
    chunk_id: i64,
    note: &NoteMetadata,
    chunk: &crate::chunk::NoteChunk,
) -> Result<()> {
    conn.execute(
        "INSERT INTO chunks_fts (rowid, content, path, title, heading_path) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            chunk_id,
            chunk.content,
            path_to_db(&note.path),
            note.title,
            chunk.heading_path.join(" > "),
        ],
    )?;
    Ok(())
}

fn delete_note_fts(conn: &Connection, note_id: i64) -> Result<()> {
    let mut stmt = conn.prepare("SELECT id FROM chunks WHERE note_id = ?1")?;
    let chunk_ids = stmt
        .query_map([note_id], |row| row.get::<_, i64>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    for chunk_id in chunk_ids {
        conn.execute("DELETE FROM chunks_fts WHERE rowid = ?1", [chunk_id])?;
    }
    Ok(())
}

fn rebuild_fts_if_empty(conn: &Connection) -> Result<()> {
    let fts_count: i64 = conn.query_row("SELECT count(*) FROM chunks_fts", [], |row| row.get(0))?;
    let chunk_count: i64 = conn.query_row("SELECT count(*) FROM chunks", [], |row| row.get(0))?;
    if fts_count > 0 || chunk_count == 0 {
        return Ok(());
    }

    conn.execute(
        r#"
        INSERT INTO chunks_fts (rowid, content, path, title, heading_path)
        SELECT chunks.id, chunks.content, notes.path, notes.title, chunks.heading_path
        FROM chunks
        JOIN notes ON notes.id = chunks.note_id
        "#,
        [],
    )?;
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

fn path_to_db(path: &Path) -> String {
    PathBuf::from(path).to_string_lossy().replace('\\', "/")
}

fn fts_query(query: &str) -> String {
    query
        .split_whitespace()
        .map(|term| term.trim_matches(|c: char| !c.is_alphanumeric() && c != '_' && c != '-'))
        .filter(|term| !term.is_empty())
        .map(|term| format!("\"{}\"", term.replace('"', "\"\"")))
        .collect::<Vec<_>>()
        .join(" ")
}

fn count(conn: &Connection, table: &str) -> Result<i64> {
    let sql = format!("SELECT count(*) FROM {table}");
    conn.query_row(&sql, [], |row| row.get(0))
        .with_context(|| format!("failed to count {table}"))
}
