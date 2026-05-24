
# Glassmind Design Doc

## One-line Summary

Glassmind is a local-first RAG and memory API for Obsidian vaults. It indexes markdown notes, performs semantic and metadata-aware search, and exposes useful context to external agents such as Claude, Codex, Hermes, Nightshift, or local models.

## Non-goals

Glassmind is not:

- an autonomous agent
- a replacement for Obsidian
- a cloud memory service
- a chatbot
- a startup-brain-theft machine
- a magic AI filesystem

Glassmind is the memory layer.

Agents call Glassmind when they need context.

---

# 1. Core Philosophy

## Source of Truth

Obsidian markdown files are canonical.

The database is rebuildable.

```text
Obsidian vault = truth
SQLite DB = index/cache
Vector index = semantic search cache
.agent/ = Glassmind-owned workspace
````

If the database is deleted, Glassmind should be able to rebuild it from the vault.

## Ownership Boundary

User notes belong to the user.

Glassmind may freely write to:

```text
.agent/
```

Editing normal vault files should be optional and policy-controlled.

---

# 2. High-Level Architecture

```text
Obsidian Vault
  ↓
Glassmind Indexer
  ↓
SQLite Metadata Store
  ↓
Vector Search Layer
  ↓
CLI / HTTP API / MCP Server
  ↓
Claude / Codex / Hermes / Nightshift / Local Models
```

## Components

### Indexer

Walks the Obsidian vault and reads markdown files.

It extracts:

- file path
    
- title
    
- headings
    
- sections
    
- tags
    
- wikilinks
    
- frontmatter
    
- modified time
    
- content hash
    
- chunks for retrieval
    

### SQLite Store

Stores structured metadata.

Examples:

```text
notes
chunks
tags
links
headings
frontmatter
index_runs
```

### Vector Store

Stores embeddings for chunks.

For v1:

```text
sqlite-vec
```

Later options:

```text
Qdrant
LanceDB
Chroma
```

### Embedding Backend

Embeddings convert text into “meaning coordinates.”

Example:

```text
"local agent memory"
```

becomes a vector like:

```text
[0.12, -0.44, 0.89, ...]
```

Glassmind compares vectors to find semantically related chunks.

Recommended v1:

```text
Ollama embeddings
```

Later:

```text
fastembed-rs
```

### API Layer

Glassmind exposes:

- CLI commands
    
- HTTP API
    
- MCP tools
    

MCP means “Model Context Protocol.” It is a standard-ish way for AI tools to call external tools.

---

# 3. Vault Layout

Glassmind should work with any Obsidian structure.

It may optionally create:

```text
vault/
  .agent/
    memories/
    summaries/
    tasks/
    decisions/
    logs/
    cache/
```

## `.agent/` Purpose

This is the safe agent-owned area.

It can contain:

- generated project summaries
    
- captured memories
    
- task state
    
- context snapshots
    
- audit logs
    
- retrieval reports
    

Normal user notes are indexed, but not modified by default.

---

# 4. Core Concepts

## RAG

RAG means “Retrieval-Augmented Generation.”

Instead of shoving the entire vault into an LLM prompt, Glassmind retrieves only relevant notes and chunks.

```text
query → retrieve relevant context → give context to LLM
```

## Embeddings

Embeddings are semantic fingerprints.

They let Glassmind find notes by meaning, not just exact words.

Useful for finding:

```text
"that thing I wrote about local memory"
```

even if the note says:

```text
"persistent semantic cache for agents"
```

## Chunk

A chunk is a small section of a note.

Usually:

- heading section
    
- paragraph group
    
- checklist block
    
- code block
    
- fixed-size fallback slice
    

Glassmind searches chunks, not entire notes.

## Hot / Warm / Cold Memory

Glassmind does not forget by default.

Instead, it ranks memory temperature.

```text
Hot:
  recent notes
  active projects
  pinned context
  recently retrieved chunks

Warm:
  related notes
  linked notes
  older project notes

Cold:
  everything else
```

Hot memory is more likely to appear in context bundles.

## Context Bundle

A context bundle is an LLM-ready packet.

Example:

```text
User query:
  "help me continue Glassmind"

Context bundle:
  - top matching chunks
  - related project notes
  - recent decisions
  - open tasks
  - relevant links
  - source paths
```

This is probably the most important Glassmind feature.

---

# 5. API Design

## CLI

```bash
glassmind init
glassmind index
glassmind search "local memory tool"
glassmind context "help me continue designing Glassmind"
glassmind read "Projects/Glassmind.md"
glassmind serve
```

## HTTP API

```http
POST /search
POST /context
GET  /notes/{id}
POST /index
GET  /health
```

## MCP Tools

```text
glassmind_search
glassmind_context
glassmind_read
glassmind_hot_context
```

---

# 6. First-Class Commands

## `search`

Returns matching chunks.

```bash
glassmind search "obsidian rag memory"
```

Output:

```text
1. Projects/Glassmind.md#Architecture
2. Daily/2026-05-24.md#Local Agents
3. Software Projects/Memory Tool.md#Embeddings
```

## `context`

Returns an LLM-ready context bundle.

```bash
glassmind context "what was I thinking about local agent memory?"
```

This should include:

- summarized answer
    
- relevant chunks
    
- source paths
    
- confidence/ranking
    
- suggested follow-up reads
    

## `index`

Builds or updates the index.

```bash
glassmind index
```

Should support incremental indexing.

## `serve`

Runs local API server.

```bash
glassmind serve
```

Default:

```text
localhost only
```

No public network exposure by default.

---

# 7. Retrieval Strategy

Do not rely on only one method.

Glassmind should use hybrid scoring.

```text
final_score =
  semantic similarity
+ keyword match
+ tag match
+ path/project boost
+ wikilink proximity
+ recency boost
+ hot memory boost
```

## Why Hybrid Search?

Embeddings are good, but not perfect.

Keyword search is good, but brittle.

Tags are useful, but manually inconsistent.

Graph links are meaningful, but incomplete.

Hybrid search makes the system feel smarter.

---

# 8. Data Model Draft

## notes

```text
id
path
title
mtime
content_hash
created_at
updated_at
```

## chunks

```text
id
note_id
heading_path
content
chunk_type
start_line
end_line
token_estimate
content_hash
```

## tags

```text
id
name
```

## note_tags

```text
note_id
tag_id
```

## links

```text
id
source_note_id
target
link_type
```

## embeddings

```text
chunk_id
model
vector
created_at
```

## memory_events

```text
id
event_type
source
content
created_at
```

---

# 9. Write Policy

Glassmind should support write policies.

## Recommended Defaults

```toml
[writes]
agent_dir = true
user_notes = "propose"
```

Modes:

```text
off:
  no writes

agent-only:
  writes only to .agent/

propose:
  produce diffs for user approval

allow:
  direct edits to user notes
```

For your personal setup, you might allow more.

For a sane default, use:

```text
agent-only + proposed diffs
```

---

# 10. Implementation Stages

## Stage 0 — Skeleton

Goal: project exists and runs.

Tasks:

- create Rust project
    
- add config file support
    
- add CLI parser
    
- define vault path
    
- add logging
    
- add `glassmind init`
    
- create `.agent/` directory
    

Suggested crates:

```text
clap
serde
toml
tracing
anyhow
```

---

## Stage 1 — Vault Indexer

Goal: read markdown vault and store metadata.

Tasks:

- recursively walk vault
    
- ignore `.obsidian/`, `.git/`, `.agent/cache/`
    
- read `.md` files
    
- calculate content hash
    
- extract title
    
- extract tags
    
- extract wikilinks
    
- extract headings
    
- store note records in SQLite
    

Suggested crates:

```text
walkdir
rusqlite
sha2
pulldown-cmark
gray_matter
regex
```

Success test:

```bash
glassmind index
glassmind stats
```

Shows:

```text
Notes indexed: 1,247
Chunks indexed: 4,912
Tags indexed: 340
Links indexed: 2,103
```

---

## Stage 2 — Chunking

Goal: split notes into useful retrieval units.

Tasks:

- split by markdown headings
    
- preserve heading path
    
- preserve line numbers
    
- fallback to size-based chunks for long sections
    
- store chunks in SQLite
    

Chunk types:

```text
heading_section
paragraph
task_block
code_block
frontmatter
```

Success test:

```bash
glassmind read-chunks "Projects/Glassmind.md"
```

---

## Stage 3 — Keyword Search

Goal: useful search before embeddings.

Tasks:

- add SQLite FTS5 table
    
- index chunk text
    
- implement `glassmind search`
    
- return path, heading, snippet, score
    

FTS means “full-text search.”

It is SQLite’s built-in text search engine.

Success test:

```bash
glassmind search "obsidian memory"
```

---

## Stage 4 — Embeddings

Goal: semantic search.

Tasks:

- add embedding backend trait
    
- implement Ollama embedding backend
    
- store vectors in sqlite-vec
    
- embed chunks
    
- embed queries
    
- return nearest chunks
    

Backend trait:

```rust
trait EmbeddingBackend {
    fn embed(&self, text: &str) -> Result<Vec<f32>>;
}
```

Config:

```toml
[embeddings]
backend = "ollama"
model = "nomic-embed-text"
```

Success test:

```bash
glassmind search "thing about my local second brain"
```

Finds notes that do not literally say those words.

---

## Stage 5 — Hybrid Ranking

Goal: search feels good.

Tasks:

- combine semantic score
    
- combine keyword score
    
- boost recent notes
    
- boost matching tags
    
- boost active project paths
    
- boost wikilink neighbors
    
- show score breakdown in debug mode
    

Example debug output:

```text
score: 0.87
semantic: 0.52
keyword: 0.18
recency: 0.07
tag: 0.05
link: 0.05
```

---

## Stage 6 — Context Bundles

Goal: make output useful for agents.

Tasks:

- implement `glassmind context`
    
- deduplicate chunks from same note
    
- group by note
    
- summarize or trim context
    
- include source paths
    
- respect token budget
    
- output markdown and JSON
    

Example:

```bash
glassmind context "continue designing Glassmind" --budget 6000
```

Output sections:

```text
Relevant Notes
Recent Decisions
Open Questions
Suggested Context
Sources
```

---

## Stage 7 — HTTP Server

Goal: let tools call Glassmind.

Tasks:

- add local HTTP server
    
- implement `/search`
    
- implement `/context`
    
- implement `/notes/{id}`
    
- implement `/health`
    
- bind to localhost by default
    

Suggested crate:

```text
axum
```

Success test:

```bash
curl localhost:7331/health
```

---

## Stage 8 — MCP Server

Goal: Claude/Codex/Hermes can call Glassmind.

Tasks:

- expose MCP tools
    
- implement search tool
    
- implement context tool
    
- implement read tool
    
- document setup
    

Tools:

```text
glassmind_search
glassmind_context
glassmind_read
```

---

## Stage 9 — Agent-Owned Memory

Goal: allow safe writes to `.agent/`.

Tasks:

- `capture-memory`
    
- `capture-decision`
    
- `capture-task`
    
- append markdown files in `.agent/`
    
- index `.agent/` files
    
- keep audit log
    

Example:

```bash
glassmind capture decision \
  --project Glassmind \
  --text "Obsidian markdown is canonical; SQLite is rebuildable cache."
```

---

## Stage 10 — Polish / Real Use

Goal: make it worth using daily.

Tasks:

- config docs
    
- better errors
    
- incremental indexing
    
- watch mode
    
- pretty CLI output
    
- JSON output
    
- MCP examples
    
- benchmark indexing
    
- backup/rebuild story
    
- test vault fixture
    

---

# 11. MVP Definition

The MVP is successful when this works:

```bash
glassmind index
glassmind search "what was I thinking about Obsidian RAG?"
glassmind context "help me continue the Glassmind project"
glassmind serve
```

And an external agent can call:

```text
glassmind_context
```

to get useful context from the vault.

---

# 12. Risks

## Risk: Overengineering

Mitigation:

- CLI first
    
- search first
    
- no autonomous agent
    
- no graph DB in v1
    

## Risk: Bad Retrieval

Mitigation:

- hybrid search
    
- score debugging
    
- manual eval queries
    
- source visibility
    

## Risk: Vault Corruption

Mitigation:

- user notes read-only by default
    
- `.agent/` writes only
    
- proposed diffs for user files
    
- audit log
    

## Risk: Slow Indexing

Mitigation:

- content hashes
    
- incremental updates
    
- skip unchanged files
    

## Risk: AI Slop in Vault

Mitigation:

- agent output goes to `.agent/inbox/`
    
- user approval before promoting content
    
- clear generated-content markers
    

---

# 13. Opinionated v1 Tech Stack

```text
Language:
  Rust

CLI:
  clap

HTTP:
  axum

Database:
  SQLite

SQL access:
  rusqlite or sqlx

Vector search:
  sqlite-vec

Markdown parsing:
  pulldown-cmark

Frontmatter:
  gray_matter

Embeddings:
  Ollama first

Logging:
  tracing

Config:
  glassmind.toml
```

---

# 14. Example Config

```toml
[vault]
path = "C:/Users/kass/Documents/ObsidianVault"

[index]
include_agent_dir = true
ignore_dirs = [".git", ".obsidian", ".trash"]
chunk_target_tokens = 500
chunk_overlap_tokens = 80

[embeddings]
backend = "ollama"
model = "nomic-embed-text"
url = "http://localhost:11434"

[search]
semantic_weight = 0.55
keyword_weight = 0.25
recency_weight = 0.10
link_weight = 0.05
tag_weight = 0.05

[writes]
mode = "agent-only"
agent_dir = ".agent"

[server]
host = "127.0.0.1"
port = 7331
```

---

# 15. Glossary

## Agent

An AI-driven system that can use tools.

Examples:

```text
Claude Code
Codex
Hermes
Nightshift
Kiro
```

Glassmind is not the agent.

Glassmind is the tool the agent calls.

## RAG

Retrieval-Augmented Generation.

A system retrieves relevant context before the LLM answers.

## Embedding

A vector representation of text meaning.

Used for semantic search.

## Vector

A list of numbers representing meaning.

## Vector Search

Finding vectors near another vector.

This finds semantically similar text.

## Chunk

A smaller piece of a note used for retrieval.

## FTS

Full-text search.

Keyword search built into SQLite.

## MCP

Model Context Protocol.

A way for AI tools to call external tools.

## Canonical Source

The real source of truth.

For Glassmind, this is the Obsidian markdown vault.

## Cache

Rebuildable derived data.

The SQLite database is a cache/index, not the truth.

## Hot Memory

Recently or frequently useful context.

## Cold Memory

Old context that is still searchable but not automatically included.

---

# 16. North Star

Glassmind succeeds if an external agent can ask:

```text
“What context from my Obsidian vault matters for this?”
```

And Glassmind returns something good enough that the agent feels like it actually remembers your projects.

Not fake memory.

Not chatbot vibes.

Actual local context retrieval over your own notes.