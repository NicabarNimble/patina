# Design: Core Extraction — Shrink Patina to Protocol + Stores

## Why This Work Exists

Patina is a domain-agnostic knowledge system, not a development tool.
[[patina-is-domain-agnostic-knowledge-system]] says it clearly: "Plugins
determine what domain Patina operates in." But the binary tells a
different story. A law firm installing Patina today compiles 2,345 lines
of GitHub API knowledge, tree-sitter Rust grammar parsing, and a spec
lifecycle engine it will never use. The core is fat with domain-specific
code that assumes software development.

The pivot: shrink core to protocol + stores. The protocol verbs
(capture/index/search/believe/evolve) and the stores they operate on
(git, SQLite, embeddings) are domain-agnostic by construction. Everything
else — GitHub knowledge, code parsing, development workflow tooling — is
a plugin that a development-focused project installs and a law firm
doesn't.

This isn't optimization. It's identity. If [[beliefs-are-the-product]],
then core should contain exactly what the belief system needs to function.
Nothing more.

**Origin:** [[session-20260303-190855]] ("scrape code is NOT core — it's
a capability added when a project needs code analysis"),
[[session-20260304-120702]] (decomposed scrape by protocol verb,
established external data is not scrape's job).

## What IS Core — Protocol + Stores

Core implements the protocol verbs and manages the stores. Nothing in
core is domain-specific.

### The Protocol Verbs

| Verb | What it does | Core implementation |
|------|-------------|-------------------|
| **capture** | Discover what changed | Git delta detection, layer/ file scanning |
| **index** | Process captured data into projections | FTS5 indexing, embedding generation |
| **search** | Find relevant knowledge | scry (vectors), assay (structural), context (progressive) |
| **believe** | Ground evidence into beliefs | Evidence chains, supports/attacks, grounding scores |
| **evolve** | Beliefs change over time | Contestation, revision, entrenchment |

### The Stores

| Store | Technology | What it holds | Disposable? |
|-------|-----------|--------------|-------------|
| **Declaration store** | Git | Beliefs, specs, sessions, layer/, text | No — source of truth |
| **Event store** | SQLite (events.db) | Append-only autobiography | No — source of truth |
| **Projection store** | SQLite (patina.db) + embeddings/ | FTS5 indexes, materialized views, vectors | Yes — rebuild from git + events |

### The Infrastructure

- **Plugin dispatch** — load WASM, route by world/role, capability gating
- **Mother** — federation, registries, cross-project coordination
- **Embeddings** — ONNX Runtime, model management (oxidize)

All of this is domain-agnostic. Event sourcing doesn't know about GitHub.
Beliefs don't know about code. Search doesn't know about Rust syntax.

## What ISN'T Core — The Extraction Targets

### Forge (2,345 LOC) — Domain: software development

GitHub API knowledge: issue sync, PR sync, rate limiting, pagination,
staging pipeline, dedup logic.

- `src/forge/` (533 LOC: mod.rs, none.rs, types.rs, writer.rs)
- `src/forge/github/` (442 LOC: mod.rs, internal.rs)
- `src/forge/sync/` (708 LOC: mod.rs, internal.rs)
- `src/commands/scrape/forge/` (604 LOC: forge subcommand handler)
- `src/generated/schemas/forge.rs` (58 LOC)

**Becomes:** A connector plugin. Uses `host/http` for GitHub API calls,
`host/emit` to write forge.issue and forge.pr facts. Manifest declares
`role = "connector"`, `host_http = ["api.github.com"]`, schemas = forge.

### Code Analysis — Domain: software development

Tree-sitter parsing, language grammars, AST traversal, call graph
construction. [[code-is-not-core]]: "A law firm, a CRM, a game AI
system — none of these need Rust syntax knowledge."

- `src/commands/scrape/code/` — code scraper dispatch
- Pipeline plugins (grammar-rust, grammar-python, etc.) — partially
  extracted already

**Becomes:** Grammar plugins (already in progress). The scrape code
subcommand that dispatches to them is removed — scrape doesn't do code.
Grammar plugins are invoked during indexing via the pipeline world.

### Spec Subsystem — Domain: development workflow

Spec lifecycle management: create, promote, pause, resume, complete,
abandon, split, block. This is development workflow tooling — a project
management system for building Patina.

- `src/spec.rs` — spec types and operations
- `src/commands/spec/` — CLI commands
- `src/mcp/server/spec.rs` — MCP tool interface

**Becomes:** An extension plugin. Needs new WIT capabilities (filesystem
write, git operations, MCP tool registration) that don't exist yet. This
is the hardest extraction and may require new host interfaces.

**Note:** Spec FILES in `layer/surface/build/` are parsed by the layer
scraper. That parsing stays in core — it's reading the declaration store.
What's extracted is the command system that manages spec lifecycle.

## The Line: Protocol vs Domain

The critical architectural question: where does protocol end and domain
begin?

**The rule:** `layer/` is in every Patina project. Git is always part of
Patina. Anything that reads the declaration store (git + layer/) is
protocol. Anything that knows about the outside world is domain.

| Component | Protocol or Domain? | Why |
|-----------|-------------------|-----|
| Git commit parsing | **Protocol** | Reads the declaration store |
| layer/ markdown parsing | **Protocol** | Reads beliefs, patterns, sessions, specs |
| Belief file parsing | **Protocol** | Beliefs are the product — always core |
| Code parsing (tree-sitter) | **Domain** | Not every project has code |
| Forge (GitHub API) | **Domain** | Not every project uses GitHub |
| Spec commands | **Domain** | Development workflow tooling |

[[scrape-is-local-capture]] formalizes this: "Scrape reads what's inside
the project (git). External data comes through connectors independently."
The declaration store IS the project. External sources are the domain.

## Sessions and Adapters — Staying in Core (For Now)

The SPEC.md originally scoped session and adapter extraction under
core-plugin-extraction. After discussion, these stay in core:

**Sessions** (`src/session.rs`, `src/commands/session/`) are the primary
interaction path. They're how users work with Patina through AI tools.
The session data (layer/sessions/ markdown) is protocol — always parsed
by core. The session commands (start, end, update, notes) are workflow
UX — but they're currently the ONLY interaction path. Extracting them
before an alternative exists would leave Patina headless.

**Adapters** (CLAUDE.md generation, Cursor rules, etc.) are tool-specific
integration surface. Clearly not protocol. But like sessions, they're
how Patina connects to the tools people actually use. Without them,
Patina has no UI.

**The rule:** Don't extract the interaction layer until there's an
alternative. Protocol CLI (scrape, scry, assay, context, believe) can
stand alone, but nobody interacts with raw protocol commands today. They
interact through sessions and adapters.

**Revisit when:** A native Patina UI exists, or the CLI can stand alone
without an AI tool driving it. At that point sessions and adapters
become extension plugins like specs.

## Forge First — Proving the Pattern

Forge extraction ([[spec-forge-plugin-extraction]]) is the first child
for a reason: it proves the entire plugin infrastructure end-to-end.

**What it proves:**
- `host/emit` works — a connector plugin can write facts to the eventlog
- `host/http` with credentials works — the plugin can call GitHub's API
  through the host boundary with injected auth
- Schema validation works — forge facts conform to the forge schema
- The manifest system works — role, capabilities, schemas all declared
- The extraction pattern works — 2,345 LOC removed from core, binary
  shrinks, behavior preserved

**If forge works as a plugin, the pattern is proven.** Every subsequent
extraction follows the same steps: identify domain code, define the WIT
interface it needs, write the plugin, remove the core code. The risk
is front-loaded in forge. Everything after is execution.

**What forge extraction looks like:**
1. Plugin-infrastructure is complete (host_emit exists, roles exist)
2. Write the forge connector plugin (WASM, mother-child world)
3. Plugin fetches GitHub data via `host/http`, emits via `host/emit`
4. Remove `src/forge/`, `src/commands/scrape/forge/` from core
5. Forge schema already exists in `.patina/schemas/forge/`
6. `patina scrape` no longer has a forge subcommand — the connector
   runs via Mother or a project-scoped task

## Scrape Simplification — Local Capture Only

Once forge is out, scrape becomes what it should be:
[[scrape-is-local-capture]].

**Current scrape does too much:**
- Reads git history (capture) — **stays**
- Parses layer/ markdown (capture + index) — **stays**
- Parses code with tree-sitter (index) — **extracted to grammar plugins**
- Fetches GitHub data (capture from external) — **extracted to connector**
- Regrounding beliefs (evolve) — **stays** (belief system is core)

**After simplification:**
- `patina scrape` = capture from git + index from layer/ + reground
  beliefs
- Code parsing = grammar plugins via pipeline world (already partially
  done)
- External data = connector plugins via mother-child or task world
- Scrape is clean, fast, and domain-agnostic

**Grounded in:** [[scrape-is-local-capture]] ("Scrape reads what's inside
the project. External data comes through connectors independently.")

## Core Plugin Extraction — The Hardest Child

[[spec-core-plugin-extraction]] extracts the spec subsystem. This is the
hardest extraction because spec commands need capabilities that don't
exist in WIT yet:

**What specs need:**
- Filesystem write access (create spec directories, write SPEC.md)
- Git operations (create tags, commits for lifecycle transitions)
- MCP tool registration (spec tools appear in AI tool interfaces)
- Layer/ file manipulation (update frontmatter, move files)

**Current WIT capabilities:** `host/layer` is read-only. There's no
`host/git`, no `host/fs-write`, no `host/mcp-register`. These would
need to be new host interfaces, designed with the same care as
`host/emit`.

**The approach:** This child spec may require its own infrastructure
work — new WIT interfaces for filesystem and git operations. That's
why it's third in sequence and acknowledged as hardest. It may need
to be split further once the scope of required WIT changes is clear.

**Scoped to specs only.** Sessions and adapters stay in core (see above).
This reduces the extraction surface and focuses on the clearest case:
spec lifecycle is development workflow tooling that not every Patina
project needs.

## Children Dependency Graph

```
┌──────────────────────────────┐
│  plugin-infrastructure       │  ← must be complete first
│  (host_emit + roles)         │
└──────────────┬───────────────┘
               │
               ↓
┌──────────────────────────────┐
│      core-extraction         │  ← this container
│        (container)           │
└──────────┬───────────────────┘
           │
     ┌─────┼──────────────┐
     ↓     ↓              ↓
┌────────┐ ┌────────────┐ ┌──────────────┐
│ forge- │ │  scrape-   │ │    core-     │
│ plugin-│ │ simplifi-  │ │   plugin-    │
│ extrac-│ │  cation    │ │  extraction  │
│ tion   │ │            │ │              │
│        │ │            │ │              │
│ FIRST  │ │  SECOND    │ │    THIRD     │
│(proves │ │(cleans     │ │  (hardest,   │
│pattern)│ │ scrape)    │ │  needs new   │
│        │ │            │ │  WIT)        │
└────────┘ └────────────┘ └──────────────┘
```

**Critical path:** plugin-infrastructure → forge-plugin-extraction →
scrape-simplification → core-plugin-extraction.

Scrape simplification depends on forge extraction (forge must be out
before scrape can drop its forge dispatch). Core plugin extraction
depends on scrape simplification only loosely — it could start in
parallel once plugin-infrastructure is done, but benefits from the
patterns proven by forge.

## What's NOT In Scope

- **No Mother changes.** Mother routing, lake management, and belief
  streams are [[spec-mother-maturation]]'s scope.
- **No new protocol verbs.** The 5 verbs are fixed. Extraction moves
  code between core and plugins; it doesn't change what the protocol does.
- **No session/adapter extraction.** These stay in core until there's
  an alternative interaction path.
- **No data architecture changes.** events.db, patina.db, the projection
  system — all stay as-is. Extraction changes WHERE code lives, not how
  data flows.

## Belief Anchors

**Identity (why the core must shrink):**
- [[patina-is-domain-agnostic-knowledge-system]] — domain-agnostic means
  no domain code in core
- [[code-is-not-core]] — code analysis is a grammar plugin, not protocol.
  A law firm doesn't need Rust syntax knowledge.
- [[beliefs-are-the-product]] — core should contain exactly what the
  belief system needs. Forge isn't that.

**Architecture (what stays, what goes):**
- [[scrape-is-local-capture]] — scrape reads git. External data comes
  through connectors. The line between protocol and domain.
- [[patina-is-knowledge-protocol]] — the protocol is capture/index/
  search/believe/evolve. Everything else is extension.
- [[wit-is-contract-wasm-is-one-runtime]] — extracted code becomes
  plugins speaking WIT interfaces. The extraction pattern.

## Key Files (Current State — What Gets Extracted)

### Forge (→ connector plugin)
- `src/forge/mod.rs` — forge types, none backend
- `src/forge/github/` — GitHub API client
- `src/forge/sync/` — sync engine, dedup, staging
- `src/commands/scrape/forge/` — forge subcommand
- `src/generated/schemas/forge.rs` — generated schema types

### Code analysis (→ grammar plugins)
- `src/commands/scrape/code/` — code scraper dispatch

### Spec subsystem (→ extension plugin)
- `src/spec.rs` — spec types and operations
- `src/commands/spec/` — CLI commands
- `src/mcp/server/spec.rs` — MCP tool interface

### Core scrapers (stay — protocol)
- `src/commands/scrape/git/` — git commit parsing
- `src/commands/scrape/layer/` — layer/ file parsing
- `src/commands/scrape/beliefs/` — belief file parsing
- `src/commands/scrape/mod.rs` — scrape orchestration, rebuild

## Open Questions

1. **How much of scrape orchestration changes?** `scrape/mod.rs` currently
   dispatches to git, layer, code, forge, beliefs. Removing code and forge
   dispatch simplifies it, but does the orchestration logic itself need
   rethinking? Or is removing the dispatch arms sufficient?

2. **Pipeline plugin invocation during scrape.** Grammar plugins already
   run via the pipeline world. After scrape-simplification, does scrape
   invoke pipeline plugins directly (for code indexing), or does that
   become a separate `patina index` step? The protocol verb split
   (capture vs index) suggests separation, but the UX of one command
   may matter more.

3. **Spec extraction WIT requirements.** What host interfaces does the
   spec subsystem actually need? A thorough audit of `src/spec.rs` and
   `src/commands/spec/` against current WIT capabilities would scope
   the infrastructure gap. This audit is the first task of
   [[spec-core-plugin-extraction]].
