# Glassmind

![Glassmind logo](docs/images/logo.png)

> Local-first retrieval for Obsidian-like markdown knowledge bases and AI workflows.

Glassmind turns a folder of markdown notes into searchable local memory for humans, agents, and local model workflows.

It works well with Obsidian vaults, but Obsidian is not required. A plain directory of `.md` files is enough.

Your notes stay local. Markdown stays canonical. The SQLite database is a rebuildable cache.

## Current Status

Glassmind now runs as a Rust CLI MVP.

It can:

- scan a markdown vault
- parse headings, paragraphs, lists, code blocks, tags, and wikilinks
- split notes into heading-based retrieval chunks
- store metadata and chunks in SQLite
- index chunks with SQLite FTS5 keyword search
- generate local deterministic embeddings
- score results with keyword, semantic, recency, tag, and wikilink signals
- build context bundles with token budgets
- expose a small localhost HTTP API
- expose MCP-style command output
- write agent-owned memories, tasks, and decisions under `.agent/`
- skip unchanged files with content hashes
- audit retrievals for debugging

Some pieces are still intentionally lightweight:

- the Ollama backend has the right interface, but does not call Ollama over HTTP yet
- vectors are stored as JSON in SQLite, not native `sqlite-vec` yet
- the HTTP server is a small standard-library server, not Axum yet
- MCP support is command-shaped, not a full MCP protocol server yet
- watch mode is simple polling

The core local retrieval flow is in place and usable for testing.

## What Glassmind Is

Glassmind is not:

- a chatbot
- an Obsidian plugin
- an autonomous agent
- a replacement for Obsidian
- a cloud memory service

Glassmind is a memory and retrieval layer.

```text
Claude / Codex / local model / your tooling
                |
           Glassmind
                |
       your markdown vault
```

The goal is simple:

> Given this task, what context from my vault actually matters?

## Quick Start

Build it:

```powershell
cargo build
```

Index the current repo:

```powershell
cargo run -- index --embeddings
```

Search:

```powershell
cargo run -- search "local memory" --debug-scores
```

Build a context bundle:

```powershell
cargo run -- context "continue glassmind" --budget 3000
```

Use a personal Obsidian vault:

```powershell
cargo run -- --vault "E:\notes\Brain" index --embeddings
cargo run -- --vault "E:\notes\Brain" search "project ideas" --debug-scores
cargo run -- --vault "E:\notes\Brain" context "what was I thinking about local agents?"
```

If your vault path has spaces, keep the quotes.

## Configuration

Glassmind reads `glassmind.toml` by default.

Useful defaults:

```toml
[vault]
path = "."

[database]
path = ".agent/cache/glassmind.sqlite3"

[index]
include_agent_dir = true
ignore_dirs = [".git", ".obsidian", ".trash", ".agent/cache"]
chunk_target_tokens = 500
chunk_overlap_tokens = 80

[embeddings]
backend = "ollama"
model = "nomic-embed-text"
url = "http://localhost:11434"

[server]
host = "127.0.0.1"
port = 7331
```

The database path is inside `.agent/cache` so it stays out of Git and can be rebuilt.

## CLI Commands

Initialize config and agent workspace:

```powershell
cargo run -- init
```

Index once:

```powershell
cargo run -- index
```

Index and generate missing embeddings:

```powershell
cargo run -- index --embeddings
```

Poll and reindex every five seconds:

```powershell
cargo run -- index --watch
```

Search:

```powershell
cargo run -- search "obsidian rag memory"
```

Search with score breakdown:

```powershell
cargo run -- search "obsidian rag memory" --debug-scores
```

JSON search:

```powershell
cargo run -- search "obsidian rag memory" --output json
```

Context bundle:

```powershell
cargo run -- context "help me continue the Glassmind project" --budget 6000
```

Stats:

```powershell
cargo run -- stats
```

## Agent Memory

Glassmind owns `.agent/`.

```text
.agent/
  memories/
  summaries/
  tasks/
  decisions/
  logs/
  cache/
```

Capture generated memory:

```powershell
cargo run -- capture memory --project Glassmind --text "Markdown remains canonical."
cargo run -- capture task --project Glassmind --text "Wire real Ollama HTTP embeddings."
cargo run -- capture decision --project Glassmind --text "SQLite is rebuildable cache."
```

Those files are markdown and are indexed on the next run.

## HTTP API

Start the local server:

```powershell
cargo run -- serve
```

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

## MCP-Style Commands

List tools:

```powershell
cargo run -- mcp tools
```

Search:

```powershell
cargo run -- mcp search "local memory"
```

Context:

```powershell
cargo run -- mcp context "continue glassmind"
```

Read:

```powershell
cargo run -- mcp read "README.md"
```

## Architecture

```text
Markdown vault
  -> scanner
  -> parser
  -> heading chunker
  -> SQLite metadata cache
  -> FTS keyword index
  -> embedding cache
  -> hybrid retriever
  -> CLI / HTTP / MCP-style tools
```

Core principle:

```text
markdown = source of truth
sqlite = rebuildable cache
embeddings = derived retrieval data
.agent/ = Glassmind-owned workspace
```

## Documentation

- [Design Document](docs/design.md)
- [FAQ](docs/faq.md)
- [HUH? Beginners Guide](docs/huh.md)
- [Technical Explainer](docs/technical-explainer.md)

## Security And Privacy

By default:

- runs locally
- binds HTTP to localhost
- keeps notes on disk
- avoids modifying normal user notes
- writes generated data under `.agent/`
- stores indexes under `.agent/cache`
- does not require cloud APIs
- has no telemetry

## Tech Stack

Current:

- Rust
- SQLite
- SQLite FTS5
- `rusqlite`
- `clap`
- `serde`
- `pulldown-cmark`
- `tracing`

Planned improvements:

- real Ollama HTTP embeddings
- native `sqlite-vec`
- Axum HTTP server
- full MCP transport
- filesystem watcher

## Legal

Glassmind is an independent project and is not affiliated with or endorsed by Obsidian.
