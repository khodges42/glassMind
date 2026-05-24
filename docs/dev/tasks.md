```md id="v2l7nq"
# tasks.md

# Glassmind Tasks

## Project Rules

- Prefer small, shippable tasks.
- Every stage should leave the project runnable.
- Avoid premature abstraction.
- Favor inspectability over magic.
- Small application philosophy
- Markdown files are canonical.
- Database state must be rebuildable.
- Local-first is a hard requirement.
- No cloud dependency in core architecture.
- No enshittification.

---

# Phase 1 — Project Skeleton & Foundations

## [x] GM-001 — Initialize Rust workspace

### Goals
- Create Rust project
- Verify build pipeline
- Establish workspace structure

### Tasks
- Run `cargo init`
- Create `/src`

- Create `/examples`
- Create `/fixtures`
- Create `/scripts`
- Create initial `.gitignore`
- Add GPL
- Verify clean build

### Acceptance Criteria
- `cargo build` succeeds
- Repo structure exists
- Project compiles on clean machine

---

## [x] GM-002 — Add core dependencies

### Goals
Install foundational crates.

### Tasks
Add:
- `clap`
- `serde`
- `serde_json`
- `toml`
- `tracing`
- `tracing-subscriber`
- `anyhow`

### Acceptance Criteria
- Project builds
- Logging works
- Config parsing stub exists

---

## [x] GM-003 — Implement CLI skeleton

### Goals
Create top-level CLI interface.

### Tasks
Add commands:
- `init`
- `index`
- `search`
- `context`
- `serve`
- `stats`

### Acceptance Criteria
- `glassmind --help` works
- Subcommands render correctly
- Unknown commands fail cleanly

---

## [x] GM-004 — Create config loader

### Goals
Load user config from disk.

### Tasks
- Define `glassmind.toml`
- Create config structs
- Implement config parsing
- Add defaults
- Add validation
- Add config path resolution

### Acceptance Criteria
- Config loads successfully
- Missing config generates defaults
- Invalid config errors clearly

---

## [x] GM-005 — Implement logging setup

### Goals
Establish consistent logging.

### Tasks
- Configure tracing subscriber
- Add log levels
- Add debug mode
- Add structured logs
- Add startup logging

### Acceptance Criteria
- Logs visible in CLI
- Debug mode works
- Errors produce stack traces

---

# Phase 2 — Vault Discovery

## [x] GM-006 — Implement vault walker

### Goals
Recursively discover markdown files.

### Tasks
- Add `walkdir`
- Walk configured vault path
- Detect `.md` files
- Skip ignored directories
- Support nested folders
- Add file count metrics

### Acceptance Criteria
- Vault scan succeeds
- Ignores work correctly
- Correct markdown count displayed

---

## [x] GM-007 — Implement ignore handling

### Goals
Allow configurable ignore patterns.

### Tasks
Ignore:
- `.git`
- `.obsidian`
- `.trash`
- `.agent/cache`

Add configurable ignores.

### Acceptance Criteria
- Ignored folders skipped
- Configurable ignores work
- No accidental recursion

---

## [x] GM-008 — Add note metadata extraction

### Goals
Extract basic note metadata.

### Tasks
Extract:
- path
- filename
- title
- modified timestamp
- file size

### Acceptance Criteria
- Metadata visible in debug output
- Data stored internally

---

## [x] GM-009 — Add markdown parsing

### Goals
Parse markdown structure.

### Tasks
Add:
- heading extraction
- paragraph extraction
- code block detection
- list detection

Suggested crate:
- `pulldown-cmark`

### Acceptance Criteria
- Headings parsed correctly
- Parser handles malformed markdown gracefully

---

## [x] GM-010 — Extract wikilinks

### Goals
Detect Obsidian-style links.

### Tasks
Support:
- `[[note]]`
- `[[note|alias]]`
- `[[folder/note]]`

Store:
- source
- target
- alias

### Acceptance Criteria
- Links parsed correctly
- Links stored in memory

---

## [ ] GM-011 — Extract tags

### Goals
Parse tags from notes.

### Tasks
Support:
- inline tags
- frontmatter tags

Normalize:
- lowercase
- trim whitespace

### Acceptance Criteria
- Tags extracted consistently
- Duplicate tags removed

---

# Phase 3 — Database Layer

## [ ] GM-012 — Add SQLite integration

### Goals
Create local metadata database.

### Tasks
- Add SQLite crate
- Create DB initialization
- Create migrations
- Create schema bootstrap

### Acceptance Criteria
- DB initializes automatically
- Schema created successfully

---

## [ ] GM-013 — Create notes table

### Goals
Store note metadata.

### Tasks
Create schema for:
- notes
- paths
- timestamps
- hashes

### Acceptance Criteria
- Notes persist correctly
- Duplicate handling works

---

## [ ] GM-014 — Create chunks table

### Goals
Store retrieval chunks.

### Tasks
Store:
- note ID
- chunk content
- heading path
- line numbers
- token estimates

### Acceptance Criteria
- Chunks persist correctly
- Relationships resolve correctly

---

## [ ] GM-015 — Add content hashing

### Goals
Detect changed notes efficiently.

### Tasks
- Add SHA256 hashing
- Hash note content
- Compare hashes on reindex
- Skip unchanged files

### Acceptance Criteria
- Incremental indexing works
- Unchanged files skipped

---

# Phase 4 — Chunking

## [ ] GM-016 — Implement heading-based chunking

### Goals
Split notes into useful retrieval units.

### Tasks
- Split by heading
- Preserve heading hierarchy
- Preserve ordering
- Preserve note references

### Acceptance Criteria
- Chunks remain readable
- Context boundaries make sense

---

## [ ] GM-017 — Add fallback chunk splitting

### Goals
Handle giant sections safely.

### Tasks
- Add max chunk size
- Add overlap windows
- Preserve sentence boundaries if possible

### Acceptance Criteria
- Large files chunk correctly
- No giant retrieval blobs

---

## [ ] GM-018 — Estimate token counts

### Goals
Prepare for LLM context budgeting.

### Tasks
- Add rough token estimator
- Store token counts
- Expose in debug mode

### Acceptance Criteria
- Estimates reasonably accurate
- Context budgeting possible

---

# Phase 5 — Search

## [ ] GM-019 — Implement SQLite FTS search

### Goals
Add keyword search.

### Tasks
- Enable FTS5
- Create search index
- Implement search query
- Add snippet extraction
- Add ranking

### Acceptance Criteria
- Search returns relevant results
- Results ranked correctly

---

## [ ] GM-020 — Implement basic CLI search command

### Goals
Expose usable search interface.

### Tasks
- Add search formatting
- Show paths
- Show headings
- Show snippets
- Add JSON output option

### Acceptance Criteria
- `glassmind search` usable daily
- Results readable
- JSON output valid

---

```md id="5m9zsw"
## Embeddings

### [ ] GM-021 — Create embedding backend trait

#### Goals
Abstract embedding providers behind a common interface.

#### Tasks
- Create `EmbeddingBackend` trait
- Define embedding request/response types
- Add async support if needed
- Add error handling
- Add provider config support

#### Acceptance Criteria
- Multiple backends can implement trait
- Search pipeline independent from provider implementation

---

### [ ] GM-022 — Implement Ollama embedding backend

#### Goals
Generate embeddings locally using Ollama.

#### Tasks
- Add Ollama HTTP client
- Implement embedding requests
- Add configurable embedding model
- Add retry handling
- Add timeout handling

#### Acceptance Criteria
- Query embeddings generated successfully
- Chunk embeddings generated successfully
- Backend configurable through TOML

---

### [ ] GM-023 — Add embedding generation pipeline

#### Goals
Generate embeddings during indexing.

#### Tasks
- Embed chunks during index phase
- Skip unchanged embeddings
- Batch embedding requests
- Add embedding queue abstraction
- Add progress reporting

#### Acceptance Criteria
- Vault indexing produces embeddings
- Reindex skips unchanged chunks

---

### [ ] GM-024 — Integrate sqlite-vec

#### Goals
Store and search vectors locally.

#### Tasks
- Add sqlite-vec dependency
- Create vector schema
- Store chunk vectors
- Add nearest-neighbor search
- Validate vector dimensions

#### Acceptance Criteria
- Embeddings persist correctly
- Similarity search returns results

---

### [ ] GM-025 — Implement semantic search

#### Goals
Search by meaning instead of keywords.

#### Tasks
- Embed query text
- Retrieve nearest vectors
- Rank results by similarity
- Return chunk metadata
- Add configurable result limits

#### Acceptance Criteria
- Semantically related notes retrieved
- Search quality noticeably useful

---

## Hybrid Retrieval

### [ ] GM-026 — Create retrieval scoring model

#### Goals
Combine multiple ranking systems.

#### Tasks
Add weighted scoring for:
- semantic similarity
- keyword relevance
- recency
- tags
- wikilinks
- path/project affinity

#### Acceptance Criteria
- Final ranking combines all scoring sources
- Weights configurable

---

### [ ] GM-027 — Add recency boosting

#### Goals
Favor recently active notes.

#### Tasks
- Define recency decay function
- Add configurable recency weights
- Support pinned notes
- Add debug scoring output

#### Acceptance Criteria
- Recent notes boosted appropriately
- Old notes still retrievable

---

### [ ] GM-028 — Add wikilink graph weighting

#### Goals
Use note relationships during retrieval.

#### Tasks
- Calculate link adjacency
- Boost linked neighbors
- Support bidirectional relationships
- Add graph traversal depth limit

#### Acceptance Criteria
- Related linked notes boosted
- Retrieval continuity improved

---

### [ ] GM-029 — Add retrieval debug mode

#### Goals
Make ranking explainable.

#### Tasks
Display:
- semantic score
- keyword score
- recency score
- tag score
- link score
- final score

#### Acceptance Criteria
- Users can inspect ranking behavior
- Retrieval tuning becomes practical

---

## Context Bundles

### [ ] GM-030 — Create context bundle builder

#### Goals
Generate LLM-ready retrieval payloads.

#### Tasks
- Define context bundle structure
- Deduplicate overlapping chunks
- Group by note
- Preserve ordering
- Add metadata blocks

#### Acceptance Criteria
- Context bundles readable
- Context bundles useful for LLM prompts

---

### [ ] GM-031 — Add token budgeting

#### Goals
Prevent oversized context payloads.

#### Tasks
- Track token estimates
- Add configurable token budget
- Trim low-priority chunks
- Preserve high-score chunks first

#### Acceptance Criteria
- Context stays within configured budget
- Retrieval quality remains useful

---

### [ ] GM-032 — Add context summarization hooks

#### Goals
Prepare for future summarization support.

#### Tasks
- Define summarizer interface
- Add optional summarization stage
- Add summary metadata fields
- Support disabling summarization

#### Acceptance Criteria
- Pipeline supports optional summarization
- Core retrieval still functions without summaries

---

### [ ] GM-033 — Implement `glassmind context`

#### Goals
Expose high-level retrieval workflow.

#### Tasks
- Add CLI command
- Format markdown output
- Add JSON mode
- Include sources
- Include retrieval metadata

#### Acceptance Criteria
- Command usable directly by humans
- Output usable by agents

---

## HTTP API

### [ ] GM-034 — Add Axum server skeleton

#### Goals
Expose Glassmind over HTTP.

#### Tasks
- Add Axum dependency
- Create server bootstrap
- Add config support
- Add graceful shutdown
- Bind localhost by default

#### Acceptance Criteria
- Server starts successfully
- Local requests succeed

---

### [ ] GM-035 — Implement `/search` endpoint

#### Goals
Expose search over HTTP.

#### Tasks
- Define request schema
- Define response schema
- Add pagination
- Add JSON serialization
- Add validation

#### Acceptance Criteria
- Endpoint returns valid search results
- Errors handled cleanly

---

### [ ] GM-036 — Implement `/context` endpoint

#### Goals
Expose context retrieval API.

#### Tasks
- Add context request schema
- Support token budget parameter
- Return structured context bundles
- Include source metadata

#### Acceptance Criteria
- API returns usable context payloads
- Response structure documented

---

### [ ] GM-037 — Implement `/notes/{id}` endpoint

#### Goals
Allow direct note retrieval.

#### Tasks
- Fetch note metadata
- Fetch chunk data
- Return markdown content
- Add error handling

#### Acceptance Criteria
- Notes retrievable by ID
- Missing notes handled correctly

---

### [ ] GM-038 — Add `/health` and `/stats`

#### Goals
Support monitoring/debugging.

#### Tasks
- Add health endpoint
- Add DB stats
- Add vault metrics
- Add embedding counts

#### Acceptance Criteria
- Health checks usable
- Stats endpoint informative

---

## MCP Support

### [ ] GM-039 — Create MCP server skeleton

#### Goals
Allow AI tools to call Glassmind directly.

#### Tasks
- Add MCP transport support
- Define tool registry
- Implement request dispatch
- Add structured tool responses

#### Acceptance Criteria
- MCP server starts successfully
- Tool calls function correctly

---

### [ ] GM-040 — Implement `glassmind_search` MCP tool

#### Goals
Expose search through MCP.

#### Tasks
- Define tool schema
- Add search execution
- Return structured results
- Include source paths

#### Acceptance Criteria
- MCP clients can search successfully

---

### [ ] GM-041 — Implement `glassmind_context` MCP tool

#### Goals
Expose context bundles through MCP.

#### Tasks
- Add context generation
- Add token budgeting
- Return structured context payloads

#### Acceptance Criteria
- MCP clients receive usable context bundles

---

### [ ] GM-042 — Implement `glassmind_read` MCP tool

#### Goals
Allow agents to inspect notes directly.

#### Tasks
- Fetch note content
- Support chunk-specific reads
- Add note metadata
- Add error handling

#### Acceptance Criteria
- Agents can retrieve note contents reliably

---

### [ ] GM-043 — Add MCP integration examples

#### Goals
Document real-world integration.

#### Tasks
- Add Claude Desktop example
- Add Codex example
- Add local agent example
- Add config examples

#### Acceptance Criteria
- Users can integrate Glassmind without guesswork

---

## Incremental Indexing

### [ ] GM-044 — Add file change detection

#### Goals
Avoid full vault reindexing.

#### Tasks
- Compare content hashes
- Detect added files
- Detect deleted files
- Detect modified files

#### Acceptance Criteria
- Incremental indexing functions correctly
- Unchanged notes skipped

---

### [ ] GM-045 — Add filesystem watch mode

#### Goals
Support live vault updates.

#### Tasks
- Add filesystem watcher
- Debounce rapid changes
- Trigger partial reindex
- Add watch logging

#### Acceptance Criteria
- File edits reflected automatically
- No runaway indexing loops

---

### [ ] GM-046 — Add partial embedding regeneration

#### Goals
Avoid recomputing unchanged vectors.

#### Tasks
- Detect changed chunks
- Recompute only dirty embeddings
- Preserve existing vectors
- Handle deleted chunks

#### Acceptance Criteria
- Reindex significantly faster after small edits

---

## Agent Workspace

### [ ] GM-047 — Create `.agent/` workspace structure

#### Goals
Establish safe agent-owned storage.

#### Tasks
Create:
- `.agent/memories`
- `.agent/tasks`
- `.agent/summaries`
- `.agent/logs`
- `.agent/cache`

#### Acceptance Criteria
- Workspace generated automatically
- Structure documented

---

### [ ] GM-048 — Add memory capture commands

#### Goals
Allow structured memory persistence.

#### Tasks
Add:
- `capture-memory`
- `capture-task`
- `capture-decision`

Store entries as markdown.

#### Acceptance Criteria
- Commands append correctly
- Entries index correctly

---

### [ ] GM-049 — Index `.agent/` content

#### Goals
Allow generated memory retrieval.

#### Tasks
- Include `.agent/` in indexing pipeline
- Tag generated content
- Preserve provenance metadata

#### Acceptance Criteria
- Agent-generated notes searchable
- Provenance visible

---

### [ ] GM-050 — Add retrieval audit logging

#### Goals
Track retrieval behavior for debugging.

#### Tasks
Log:
- query
- retrieved chunks
- retrieval scores
- timestamp
- requesting client

#### Acceptance Criteria
- Retrievals traceable
- Logs useful for tuning/debugging
```
---

# What's Next

## Retrieval Quality
- Evaluation datasets
- Ranking tuning
- Query debugging
- Explainable scoring

## Performance
- Parallel indexing
- Cached embeddings
- Batch embedding generation
- Large vault optimization

## Future Ideas
- Git history awareness
- Temporal retrieval
- Canvas parsing
- Code-aware chunking
- Multi-vault support
- Graph exploration
- Retrieval visualization
- Vault analytics
- Semantic diffing
- “What changed?” context reports
- Local reranking models
- Session continuity memory
- Agent-safe write proposals
```
