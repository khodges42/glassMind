# Glassmind

> Local-first semantic retrieval for Obsidian-like markdown knowledge bases and AI workflows.

* This is in development, it doesn't run yet. Want to help? Get in contact! * 

Glassmind turns folders of markdown notes into searchable semantic memory for AI tools and humans.

It works especially well with Obsidian vaults, but Obsidian is not required.

It indexes markdown, understands links/tags/headings, performs hybrid semantic retrieval, and exposes context through a CLI, HTTP API, and MCP tools.

Your notes stay local.
Your vault stays canonical.
The database is rebuildable.
No cloud required.

---

## What is this?

Glassmind is **not**:

* a chatbot
* an obsidian plugin
* an autonomous agent
* a replacement for Obsidian
* a SaaS startup trying to ingest your second brain into a valuation event

Glassmind is a **memory and retrieval layer**.

Think:

```text
Claude / Codex / Hermes / local model
                ↓
           Glassmind
                ↓
         Your Obsidian vault
```

The goal is simple:

> “Given this task, what context from my vault actually matters?”

---

# Features

## Current / Planned

* Markdown vault indexing
* Semantic search
* Hybrid retrieval

  * embeddings
  * keyword search
  * tags
  * wikilinks
  * recency
* Context bundle generation
* MCP integration
* HTTP API
* Local-first operation
* Rebuildable indexes
* Incremental indexing
* Agent-safe `.agent/` workspace
* Obsidian-compatible by default

---

# Philosophy

Glassmind treats your vault like memory, not files.

```text
Obsidian markdown = source of truth
SQLite = rebuildable index/cache
Embeddings = semantic retrieval layer
```

Your notes remain human-readable markdown.

Glassmind exists to make retrieval useful, fast, and agent-friendly without turning your vault into proprietary soup.

---

# Example

```bash
glassmind index

glassmind search "local memory tool ideas"

glassmind context "help me continue the Glassmind project"

glassmind serve
```

---

# Why?

Because existing “AI memory” systems tend to be one of:

* cloud-first
* opaque
* startup-shaped
* agent-shaped
* overengineered
* weirdly hostile to user ownership

Meanwhile, many of us are already using Obsidian as informal long-term memory.

Glassmind formalizes that idea.

---

# Documentation

* [Design Document](docs/design.md)
* [FAQ](docs/faq.md)
* [HUH? (Beginners ELI5 guide)](docs/huh.md)

---

# Architecture

```text
Obsidian Vault
  ↓
Indexer
  ↓
SQLite + Vector Search
  ↓
CLI / HTTP / MCP
  ↓
Agents and local models
```

---

# Tech Stack

Planned v1 stack:

```text
Rust
SQLite
sqlite-vec
Ollama embeddings
Axum
MCP
```

---

# Status

Early development.

Currently building:

* vault indexer
* chunking
* semantic retrieval
* context generation

---

# Security / Privacy

Glassmind is designed to run locally.

By default:

* binds to localhost
* keeps notes local
* avoids modifying user notes
* stores indexes separately
* treats markdown as canonical

No telemetry is planned.

No cloud dependency is required.

No “AI-enhanced knowledge monetization platform” nonsense.

No enshitification ever. I stake my professional reputation on it.

---

# Name

Why “Glassmind”?

Because it’s supposed to feel like peering through semantic glass into your own thoughts.

Also because `brainworm` felt a little aggressive for a tool people may actually deploy at work. 


---

# Contributing

Eventually.

Right now the project is still in the “rapid architectural mutation” phase.

If you want to throw me a PR or two I'll give you one (1) really good compliment.

---

# Legal

Glassmind is an independent project and is not affiliated with or endorsed by [Obsidian](https://obsidian.md).

---

# I am a recruiter

Hi.

You may also enjoy:

* [LinkedIn / khodges42](https://linkedin.com/in/khodges42?utm_source=chatgpt.com)


