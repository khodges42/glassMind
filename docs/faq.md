````md
# FAQ.md

# Frequently Asked Questions

## What is Glassmind?

Glassmind is a local-first semantic retrieval and memory system for markdown knowledge bases.

It indexes markdown files, builds semantic and structural search indexes, and exposes retrieval APIs for:
- AI assistants
- local models
- agents
- MCP clients
- automation tooling
- humans using the CLI directly

Glassmind is designed to work especially well with Obsidian vaults, but only requires a directory of markdown files.

---

# What problem is Glassmind solving?

Modern LLMs are powerful but stateless.

They:
- lose context
- forget projects
- cannot inherently understand your local files
- have limited prompt windows
- hallucinate when context is missing

Meanwhile many people already maintain:
- engineering documentation
- project journals
- research notes
- worldbuilding
- task tracking
- personal knowledge systems

inside markdown repositories.

Glassmind bridges those worlds.

It provides:
- retrieval
- semantic search
- context construction
- memory indexing

over existing markdown workflows.

---

# Is Glassmind an AI agent?

No.

Glassmind is retrieval infrastructure.

It does not:
- autonomously execute tasks
- reason independently
- act as a chatbot
- replace orchestration frameworks

It is closer to:
- a search engine
- a semantic index
- a memory API
- a retrieval layer

Agents and AI tools call Glassmind to retrieve relevant context.

---

# Is Glassmind tied to Obsidian?

No.

Glassmind is markdown-native.

It works with:
- Obsidian vaults
- plain markdown directories
- docs repositories
- PKM systems
- engineering notebooks
- wiki-style folder structures

Obsidian is simply a particularly good fit because:
- it is local-first
- it uses markdown
- it has strong linking semantics
- it is widely adopted

Glassmind treats markdown files as canonical regardless of editor.

---

# Why markdown?

Because markdown is:
- portable
- durable
- inspectable
- editor-agnostic
- version-control friendly
- human-readable

Glassmind intentionally avoids proprietary storage formats for primary knowledge.

The markdown files remain the source of truth.

Everything else is rebuildable.

---

# What is the source of truth?

The markdown files.

Glassmind builds:
- indexes
- caches
- embeddings
- retrieval metadata

on top of them.

The database is disposable and rebuildable.

If Glassmind disappears, the notes still work.

---

# What database does Glassmind use?

Planned v1:

```text
SQLite
sqlite-vec
```

SQLite stores:
- note metadata
- chunk metadata
- tags
- links
- retrieval state
- indexes

sqlite-vec stores:
- semantic vectors ("embeddings")

The database is local and rebuildable.

---

# Why SQLite?

Because SQLite is:
- local-first
- fast enough
- battle-tested
- portable
- operationally simple

Glassmind intentionally avoids requiring:
- external database servers
- cloud infrastructure
- distributed systems
- operational overhead

for normal usage.

---

# What are embeddings?

Embeddings are vector representations of semantic meaning.

A chunk of text is transformed into a vector like:

```text
[0.12, -0.44, 0.89, ...]
```

Vectors with similar meaning are located near each other mathematically.

This enables semantic search.

Example:

```text
"persistent semantic cache"
```

can match:

```text
"local memory system"
```

even if the wording differs.

---

# Does Glassmind require online APIs?

No.

Glassmind is designed for local operation.

Planned local embedding options:
- Ollama
- fastembed-rs
- llama.cpp-compatible backends

Cloud embeddings may eventually be optional, but local-first is the default philosophy.

---

# What is hybrid retrieval?

Glassmind does not rely solely on embeddings.

Retrieval combines:
- semantic similarity
- keyword matching
- tags
- wikilinks
- recency
- project/path weighting
- hot memory boosting

This generally performs better than pure vector search.

---

# What is a chunk?

A chunk is a retrieval unit.

Instead of embedding entire files, Glassmind splits documents into smaller pieces.

Usually:
- heading sections
- paragraphs
- task blocks
- code blocks
- fixed-size fallback windows

Chunking improves:
- retrieval quality
- precision
- context density
- token efficiency

---

# What is a context bundle?

A context bundle is an LLM-ready retrieval result.

Instead of returning raw search matches only, Glassmind can assemble:
- relevant chunks
- related notes
- recent project activity
- linked concepts
- source references

into a structured payload optimized for AI consumption.

Example:

```text
"Help me continue the Glassmind architecture work"
```

might retrieve:
- recent architecture notes
- TODOs
- design decisions
- linked experiments
- related discussions

within a configurable token budget.

---

# What is MCP?

MCP stands for:

```text
Model Context Protocol
```

It is a protocol used by AI tools to interact with external systems and tools.

Glassmind plans to expose MCP-compatible retrieval tools such as:

```text
glassmind_search
glassmind_context
glassmind_read
```

This allows tools like Claude Code or other agent systems to retrieve vault context directly.

---

# How is Glassmind different from traditional RAG systems?

Many RAG systems are:
- cloud-first
- opaque
- tightly coupled to vector databases
- detached from user workflows
- document-ingestion pipelines rather than knowledge systems

Glassmind is designed around:
- local-first operation
- markdown-native workflows
- inspectability
- rebuildability
- human-readable source material
- AI + human co-usage

Glassmind assumes the markdown corpus is already meaningful.

It focuses on retrieval quality and continuity.

---

# How is Glassmind different from vector databases?

Vector databases store embeddings and perform nearest-neighbor search.

Glassmind is:
- retrieval orchestration
- indexing
- chunking
- metadata extraction
- semantic ranking
- context assembly
- markdown-aware infrastructure

Glassmind may use vector storage internally, but it is not merely a vector DB wrapper.

---

# Will Glassmind modify my notes?

By default:
- no direct user note modification

Glassmind may optionally write to:

```text
.agent/
```

for:
- summaries
- logs
- generated memory
- task state
- context artifacts

Future configurable modes may support:
- proposed diffs
- explicit approvals
- direct modification

but user ownership and safety are priorities.

---

# Why not just use grep or ripgrep?

Keyword search is extremely useful and Glassmind still supports it.

But semantic retrieval solves problems like:

```text
"I know I wrote about this concept but I forgot the terminology."
```

Glassmind combines:
- keyword retrieval
- semantic retrieval
- structural metadata
- recency
- graph relationships

rather than replacing traditional search entirely.

---

# Why not use a graph database?

Maybe eventually.

But for v1:
- simplicity
- rebuildability
- portability
- operational sanity

matter more.

SQLite plus semantic indexing is likely sufficient for:
- personal vaults
- power-user vaults
- local AI workflows

Graph semantics can still exist logically without introducing a distributed graph infrastructure problem on day one.

---

# Is Glassmind intended for teams?

Not initially.

The primary target is:
- individuals
- researchers
- engineers
- writers
- local AI workflows
- personal knowledge systems

Future multi-user support is possible but not the immediate focus.

---

# What does “local-first” actually mean here?

The intended default behavior is:

- local storage
- localhost-only networking
- optional offline operation
- local embeddings
- markdown canonical storage
- rebuildable indexes
- no required cloud dependency
- no telemetry

Glassmind should remain usable:
- disconnected
- self-hosted
- archived
- years into the future

---

# What does “hot memory” mean?

Glassmind conceptually separates retrieval into:
- hot memory
- warm memory
- cold memory

Hot memory includes:
- recent notes
- active projects
- pinned information
- recently retrieved context

Cold memory still exists, but is less likely to be automatically surfaced.

This helps context selection remain relevant without deleting historical information.

---

# What are the long-term goals?

Long-term goals include:
- strong retrieval quality
- excellent local AI workflows
- durable markdown-native memory infrastructure
- robust MCP integration
- context continuity across sessions
- transparent retrieval behavior
- inspectable ranking systems
- ergonomic semantic search

Not:
- replacing human thought
- building autonomous AGI office workers
- trapping users inside proprietary ecosystems

---

# Why is the project opinionated?

Because retrieval quality and long-term maintainability depend heavily on architecture choices.

Glassmind intentionally prefers:
- explicit systems
- rebuildable state
- inspectability
- portability
- user ownership
- operational simplicity

over:
- hidden magic
- giant opaque pipelines
- cloud dependence
- maximal abstraction

---

# Why the name “Glassmind”?

The original idea was:

```text
semantic transparency into your own thoughts
```

The system is supposed to feel like:
- peering through glass
- inspecting memory
- traversing thought structures

rather than interacting with a black box.

Also it sounded less alarming than some of the other candidate names.
````
