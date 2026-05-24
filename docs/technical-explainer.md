# Glassmind Technical Explainer

This document explains how Glassmind works for someone who is comfortable with software development, but new to RAG, embeddings, vector search, Obsidian-style markdown indexing, or MCP-style tool surfaces.

Glassmind is a local retrieval layer for a directory of markdown files. It treats your notes as the source of truth, builds a rebuildable SQLite cache from them, and then uses that cache to answer search and context requests.

The short version:

```text
markdown files
  -> scanner
  -> markdown parser
  -> chunker
  -> SQLite cache
  -> keyword index
  -> embedding cache
  -> hybrid retriever
  -> search results / context bundle / HTTP / MCP-style tools
```

## What Problem Glassmind Solves

Large language models do not automatically know what is in your local notes. Even if an AI tool can read files, dumping an entire vault into a prompt is slow, expensive, noisy, and usually impossible because context windows are limited.

RAG means Retrieval-Augmented Generation. The idea is simple:

```text
user asks something
system retrieves relevant source material
LLM receives only that relevant context
LLM answers with better grounding
```

Glassmind is the retrieval part. It does not try to be the chatbot. It finds the pieces of your markdown vault that matter.

## Core Design Rule

Markdown is canonical.

That means:

- your `.md` files are the real data
- SQLite is only a cache
- embeddings are only derived data
- deleting the database should not delete knowledge
- indexing should be repeatable

By default, Glassmind only writes generated project data under `.agent/`. Normal notes are read, not edited.

## Vault Scanning

The scanner walks the configured vault path and finds markdown files.

By default it skips folders such as:

- `.git`
- `.obsidian`
- `.trash`
- `.agent/cache`

It intentionally does not skip all of `.agent/`, because generated memories, decisions, and task notes should be searchable. It only skips `.agent/cache`, which is where the SQLite database lives.

For each markdown file, Glassmind records metadata:

- relative path
- filename
- title
- modified timestamp
- file size
- SHA256 content hash

The hash is important. It lets Glassmind tell whether a note changed since the last index run.

## Markdown Parsing

After reading a file, Glassmind parses useful markdown structure.

It extracts:

- headings
- paragraphs
- code blocks
- list items
- Obsidian-style wikilinks
- tags

Supported wikilinks include:

```text
[[note]]
[[note|alias]]
[[folder/note]]
```

Tags can come from inline markdown:

```md
This is about #rust and #local-first tooling.
```

Or frontmatter:

```md
---
tags: [rust, retrieval, notes]
---
```

Tags are normalized to lowercase and deduplicated.

## Chunking

Search works better on smaller pieces of text than whole files. Those pieces are called chunks.

Glassmind currently chunks by heading section first. For example:

```md
# Project

Intro text.

## Design

Design text.

## Tasks

Task text.
```

This becomes separate retrieval chunks for the top-level section and child sections. Each chunk keeps its heading path, so results can point back to where they came from:

```text
Project > Design
Project > Tasks
```

If a section is too large, Glassmind splits it into smaller overlapping chunks. Overlap helps avoid cutting useful context exactly at a boundary.

Each chunk stores:

- note id
- chunk index
- heading path
- content
- chunk type
- start line
- end line
- rough token estimate
- chunk content hash

The token estimate is currently simple word counting. It is not perfect, but it is good enough for budgeting context bundles.

## SQLite Cache

The local database lives here by default:

```text
.agent/cache/glassmind.sqlite3
```

It is ignored by Git and can be rebuilt from markdown.

The main tables are:

- `notes`: one row per markdown note
- `chunks`: retrieval chunks
- `tags`: normalized tag names
- `note_tags`: many-to-many join table
- `links`: wikilinks from notes
- `embeddings`: vector cache for chunks
- `retrieval_audit`: search history for debugging retrieval behavior
- `memory_events`: generated memory records
- `migrations`: schema bootstrap marker

On index, Glassmind compares the current file hash with the hash stored in `notes`.

If the hash matches and the index version matches, the note is skipped.

If the note changed, Glassmind rewrites its child rows:

```text
old chunks
old FTS rows
old embeddings
old tags mapping
old links
```

Then it inserts the fresh metadata.

If a file was deleted from the vault, the indexer removes that note and its derived rows from the cache.

## Keyword Search With FTS

SQLite includes a full-text search engine called FTS5. Glassmind creates an FTS table for chunk content.

When chunks are written, matching FTS rows are written too.

A keyword search runs roughly like this:

```sql
SELECT chunk metadata, snippet, rank
FROM chunks_fts
JOIN chunks
JOIN notes
WHERE chunks_fts MATCH query
ORDER BY bm25 rank
```

FTS gives Glassmind:

- fast local keyword search
- ranked results
- snippets with matched terms highlighted

This is the most reliable baseline search mode because it does not require a model.

## Embeddings

Embeddings are numeric representations of text meaning.

Conceptually:

```text
"local memory for agents"
  -> [0.12, -0.04, 0.77, ...]
```

Texts with similar meaning should produce vectors that are close to each other.

Glassmind has an `EmbeddingBackend` trait:

```text
text in
vector out
```

Right now there is a deterministic local embedding backend. It is not a real language model embedding, but it lets the full pipeline work locally and predictably while the storage and retrieval flow stabilizes.

There is also an Ollama-shaped backend stub. The code has the right boundary for an Ollama implementation, but the current version does not call Ollama over HTTP yet.

Embeddings are stored in SQLite as JSON arrays in the `embeddings` table.

This is not the final high-performance vector storage design. The intended future path is native `sqlite-vec`. The current implementation keeps everything runnable with plain SQLite while preserving the architecture.

## Semantic Search

Semantic search compares the query embedding to chunk embeddings.

The comparison uses cosine similarity:

```text
1.0  = very similar
0.0  = unrelated
-1.0 = opposite direction
```

In practice, Glassmind:

1. embeds the query
2. loads candidate chunks
3. compares query vector to chunk vectors
4. assigns a semantic score

The current semantic path is useful as plumbing and scoring infrastructure. Search quality will improve when the Ollama or sqlite-vec pieces become real model-backed vector search.

## Hybrid Retrieval

Pure keyword search is brittle. Pure semantic search can be fuzzy or surprising.

Glassmind combines multiple scoring signals:

- keyword score
- semantic score
- recency score
- tag score
- wikilink score

The config has weights:

```toml
[search]
semantic_weight = 0.55
keyword_weight = 0.25
recency_weight = 0.10
link_weight = 0.05
tag_weight = 0.05
```

The final score is a weighted blend.

You can inspect the pieces with:

```powershell
cargo run -- search "local memory" --debug-scores
```

That makes retrieval behavior less magical. If a result is weird, you can see whether it came from keyword matching, semantic similarity, recency, tags, or links.

## Context Bundles

Search results are useful for humans, but agents usually need a compact context packet.

That is what `glassmind context` builds.

Example:

```powershell
cargo run -- context "continue glassmind" --budget 4000
```

The context builder:

1. runs retrieval
2. takes the highest-scoring chunks
3. respects the token budget
4. outputs markdown by default
5. includes source paths

The result is meant to be pasted into an LLM prompt or returned to an agent.

There is also a summarizer hook. It is disabled right now, but the interface exists so local summarization can be added later without changing the bundle format.

## Agent Workspace

Glassmind owns `.agent/`.

The current structure is:

```text
.agent/
  memories/
  summaries/
  tasks/
  decisions/
  logs/
  cache/
```

Capture commands append markdown into this workspace:

```powershell
cargo run -- capture memory --project Glassmind --text "SQLite is rebuildable cache."
cargo run -- capture task --project Glassmind --text "Wire real Ollama embeddings."
cargo run -- capture decision --project Glassmind --text "Markdown remains canonical."
```

These generated files are indexed like normal markdown. That gives agents a place to write memory without touching user-owned notes.

## HTTP API

`glassmind serve` starts a small localhost HTTP server.

Default bind:

```text
127.0.0.1:7331
```

Endpoints:

- `GET /health`
- `GET /stats`
- `POST /search`
- `POST /context`
- `GET /notes/{path}`

Example:

```powershell
curl http://127.0.0.1:7331/health
```

Search request:

```json
{
  "query": "local memory",
  "limit": 5
}
```

Context request:

```json
{
  "query": "continue glassmind",
  "limit": 8,
  "budget": 6000
}
```

The current server is intentionally simple and uses the Rust standard library. A later version can swap this for Axum without changing the core indexing and retrieval flow.

## MCP-Style Tool Commands

MCP means Model Context Protocol. It is a way for AI tools to call external tools.

Glassmind currently has an MCP-style command surface:

```powershell
cargo run -- mcp tools
cargo run -- mcp search "local memory"
cargo run -- mcp context "continue glassmind"
cargo run -- mcp read "README.md"
```

This is not a full MCP transport yet. It is the command and response shape that a real MCP server can reuse later.

## Watch Mode

There is a simple polling watch mode:

```powershell
cargo run -- index --watch
```

It reindexes every five seconds.

This is intentionally plain. A real filesystem watcher can replace it later, but the current loop proves the live indexing behavior without another moving part.

## Retrieval Audit Logging

Hybrid searches write audit rows into SQLite.

The audit log stores:

- query
- returned paths
- timestamp
- client label

This is for tuning. Retrieval systems are hard to improve if you cannot inspect what they returned and why.

## Typical Local Workflow

For this repo:

```powershell
cargo run -- index --embeddings
cargo run -- search "glassmind local memory" --debug-scores
cargo run -- context "continue glassmind" --budget 3000
```

For a personal Obsidian vault:

```powershell
cargo run -- --vault "E:\notes\Brain" index --embeddings
cargo run -- --vault "E:\notes\Brain" search "project ideas" --debug-scores
cargo run -- --vault "E:\notes\Brain" context "what was I thinking about local agents?"
```

If the path has spaces, keep the quotes.

## What Is Real Now

The working spine is:

```text
scan
parse
chunk
hash
cache
FTS search
embedding cache
hybrid scoring
context bundles
HTTP surface
MCP-style commands
agent memory capture
audit logging
```

That is enough to test the product shape end to end.

## What Is Still Placeholder Or Lightweight

Some pieces are intentionally MVP-level:

- the Ollama backend has the right interface but does not call Ollama yet
- vector storage is SQLite JSON, not native `sqlite-vec`
- semantic search is brute-force over candidate chunks
- HTTP uses a tiny standard-library server, not Axum
- MCP is command-shaped, not a full MCP protocol server
- watch mode is polling, not filesystem events

These are implementation swaps, not architecture rewrites.

The main architecture is already pointing in the right direction:

```text
markdown source of truth
rebuildable local cache
inspectable retrieval
agent-safe writes
human-readable output
```

