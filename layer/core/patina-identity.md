---
id: patina-identity
layer: core
status: active
created: 2026-02-11
tags: [identity, architecture, core-principle, plugin-boundary]
references: [dependable-rust, unix-philosophy, adapter-pattern, spec-driven-design]
---

# Patina Identity

**Purpose:** Define what Patina is and isn't, where the core boundary lives, and when something belongs in the binary versus a plugin. Every architectural decision flows from this document.

---

## Core Principle

Patina is a **knowledge substrate for AI-assisted development**. It captures, indexes, and serves project context — patterns, beliefs, sessions, code structure, commit history — so that AI agents can make informed decisions about your codebase. Patina is to AI agents what git is to editors: invisible infrastructure that makes the tool above it smarter.

**The binary is the pipeline. The layer is the product.**

## What Patina IS

Patina does six things. Everything in the binary serves one of these:

### 1. Capture — Extract knowledge from development artifacts

Code structure, git history, layer files, forge data, beliefs. The `scrape` pipeline reads your project and writes to SQLite + eventlog. This is the intake.

**Modules:** `scrape` (15K lines), `scanner`, `eventlog`, `forge`, `git`

### 2. Index — Build searchable representations

Embeddings from ONNX models, FTS5 indexes, structural graphs, temporal co-change matrices. The `oxidize` pipeline transforms raw scrape data into queryable form.

**Modules:** `oxidize`, `embeddings`, `models`, `db`

### 3. Serve — Answer questions about your project

Semantic search (scry), factual search (assay), pattern delivery (context), belief grounding. This is what AI agents consume via MCP or CLI.

**Modules:** `scry`, `assay`, `context`, `retrieval`, `mcp`

### 4. Govern — Track decisions and enforce process

Specs authorize work. Sessions capture discussion. Beliefs capture principles. Versions track releases. The governance pipeline ensures decisions have provenance.

**Modules:** `spec`, `session`, `belief`, `release`, `version`

### 5. Connect — cross-project awareness and plugin orchestration

Mother is the daemon that connects everything. Cross-project routing, model caching, relationship graph, and future plugin management. Mother IS how Patina scales beyond a single project.

**Modules:** `mother` (daemon, graph, children), `adapters` (LLM entry points), `workspace`

### 6. Protect — security infrastructure

Secrets management, scanner, encryption. Security is core infrastructure that grows with the system, not an optional add-on.

**Modules:** `secrets`, `scanner`

### Foundation

These serve all six functions:

- **`layer`** — filesystem structure for knowledge storage (core/surface/dust/sessions)
- **`paths`** — single source of truth for all path construction (no I/O)
- **`project`** — unified config management (.patina/config.toml)
- **`models`** — embedding models exist so Patina can work. No models, no scry.
- **`migration`** — path migration between versions (idempotent)

## What Patina IS NOT

### Not a build system

Patina doesn't compile code, run tests, or deploy artifacts. It *understands* build artifacts (Cargo.toml, package.json) to auto-detect project type, but it never executes builds. `ReleaseStrategy::External` prints advisory messages — it doesn't run `npm publish`.

### Not a deployment tool

No CI/CD, no container orchestration, no cloud integration. `yolo` generates devcontainer configs (1,600 lines) — this is the strongest candidate for extraction. It creates environments; Patina captures knowledge.

### Not an LLM runtime

Adapters are config generators, not inference engines. `ClaudeAdapter` writes `CLAUDE.md` and copies session scripts. It doesn't call the Claude API, manage tokens, or route prompts. The MCP server exposes Patina's *knowledge* tools to LLMs — it doesn't host LLM capabilities.

### Not a task tracker

Session tracking and spec lifecycle are governance, not project management. Patina doesn't have sprints, assignees, or Kanban boards. The frozen `patina-work` spec (beads-like work tracking) belongs in a plugin, not core.

### Not a general-purpose database

SQLite is an implementation detail. Patina doesn't expose SQL, manage schemas for external consumers, or serve as a data warehouse. The eventlog is an append-only knowledge substrate, not an application database.

## The Core/Plugin Boundary

The boundary follows a simple test: **does it serve a core function (capture, index, serve, govern, connect, protect)?** If yes, it's core. If it's an optional enhancement that Patina can function without, it's a plugin.

### Definitely Core — stays in the binary

| Module | Pillar | Why core |
|--------|--------|----------|
| `scrape` (all) | Capture | The entire pipeline — engine + scrapers (code, git, layer, forge, beliefs). Scrape is how Patina gets data. |
| `oxidize` | Index | Embedding pipeline. Transforms raw scrape data into queryable form. |
| `scry` + `retrieval` | Serve | Query engine + oracle fusion. The serve surface for AI agents. |
| `assay` | Serve | Structural/factual queries (FTS5, imports, call graph). |
| `context` | Serve | Pattern + belief delivery to agents. |
| `session` | Govern | Session lifecycle. |
| `spec` | Govern | Spec lifecycle + release delegation. |
| `release` | Govern | Strategy dispatch (Cargo/External/None). |
| `belief` | Govern | Belief audit + metrics. |
| `layer` | Foundation | Knowledge storage structure. |
| `paths` | Foundation | Path construction (no I/O). |
| `project` | Foundation | Config management. |
| `eventlog` | Foundation | Append-only truth store. |
| `db` | Foundation | SQLite abstraction. |
| `embeddings` | Foundation | Embedding engine trait + ONNX runtime. |
| `models` | Foundation | Embedding models exist so Patina can work. No models, no embeddings, no scry. |
| `mcp` | Serve | Protocol bridge (JSON-RPC over stdio). Thin shim — CLI is the product. |
| `init` | Foundation | Project skeleton setup. |
| `mother` | Connective | The daemon that connects everything — cross-project routing, plugin management, caching, graph. Mother IS how Patina scales beyond a single project. Future: runs adapters, manages plugins. |
| `adapters` | Entry | How users and agents enter Patina. Today: config generators. Future: Mother-managed runtime integration. |
| `secrets` | Security | Local-first encryption (age + Keychain). Security is core infrastructure that will grow, not optional. |

### Definitely Plugin — extract as plugin system matures

`doctor` was the first extraction (v0.17.0) — it ships as a WASM plugin with compiled fallback, proving the pattern works. Next candidates:

| Module | Lines | Why plugin | Status |
|--------|-------|------------|--------|
| `yolo` | 1,613 | Devcontainer generation isn't knowledge. Strongest extraction candidate. | Pending |
| `eval` + `bench` | 3,229 | Quality measurement for power users. Not core to knowledge serving. | Pending |
| `report` | ~605 | Report generation. Composed from core tools — classic plugin. | Pending |
| `doctor` | 278 | Health checks. Useful but not knowledge infrastructure. | **Extracted (v0.17.0)** |
| `upgrade` | 162 | Version check. Utility, not pillar. | Pending |

## The Plugin Test

Before adding ANY new module to the binary, apply this test:

### 1. Does it serve a core function?

Capture, index, serve, govern, connect, or protect. If no → it's a plugin. Full stop.

### 2. Can Patina function without it?

If yes → it's a plugin. `patina scrape && patina scry "how does auth work?"` must work without eval, yolo, or doctor installed.

### 3. Does it introduce a new external dependency?

If yes → strong signal for plugin. Every new dependency in `Cargo.toml` increases binary size and attack surface. The 52MB binary exists because everything is compiled in.

### 4. Would a different project want different behavior?

If yes → it's a plugin. Forge behavior differs per platform (GitHub vs Gitea vs GitLab). Oracle behavior differs per domain. These are extension points, not core.

### When in doubt: bundle now, extract later

The plugin system is live (v0.17.0) — wasmtime + WIT Component Model with two worlds (child world for daemon children, command world for CLI commands). Doctor was the first extraction, proving the pattern: WASM-first with compiled fallback via feature gate (`bundled-doctor`). The MotherChild trait and existing traits (LLMAdapter, ForgeReader, Oracle) are the plugin interfaces — some dispatch to compiled code, some to WASM, with the boundary moving outward over time.

## Architectural Invariants

These are non-negotiable properties of the system:

### 1. Rust-first runtime

No Python subprocess dependencies. No Node.js. No shell scripts at runtime. Embeddings run through ONNX Runtime via `ort` crate. Cross-platform: same vector space on Mac/Linux/Windows.

**Why:** AI agents execute in contexts without guaranteed runtimes. The binary must be self-contained.

### 2. Local-first data

All knowledge lives on disk. SQLite databases, markdown files, TOML configs. No cloud dependencies for core operation. Secrets use `age` encryption + macOS Keychain — no external vault services.

**Why:** Project knowledge is sensitive. Dependencies on external services create availability risks that compound in agentic contexts.

### 3. Eventlog is truth

The append-only eventlog is the canonical data source. All SQLite tables (code_fts, commits_fts, moments, etc.) are materialized views — derived, rebuildable. `patina rebuild` recreates everything from eventlog + layer/ + git.

**Why:** Rebuildable data is portable. Clone the repo, run rebuild, get full knowledge. No state migration, no schema versioning hell.

### 4. Layer is git-tracked knowledge

`layer/` is the knowledge product — checked into git, versioned, reviewable. `.patina/` is derived local state — gitignored, rebuildable. Never store irreplaceable data in `.patina/`.

**Why:** Knowledge that isn't in git doesn't survive `rm -rf .patina/`. Git is the durability layer.

### 5. Compiler-enforced safety

In agentic contexts, the compiler is the only reliable review gate. Prefer enums over strings, typestate over documentation, exhaustive match over convention. See [[compiler-enforced-safety]].

**Why:** No PR review, no team lead, no QA gate. If bad code compiles, it ships.

### 6. Sync-first execution

Synchronous, blocking code by default. No async infection. Use `std::thread::scope` for parallelism when needed, `rayon` for CPU-bound batch work. See [[sync-first]].

**Why:** Async adds complexity without benefit for inherently sequential workloads (file I/O, SQLite, CLI commands). Contained async only if WASM host I/O demands it.

### 7. MCP is shim, CLI is product

The MCP server is a discovery mechanism for LLM agents. It wraps CLI logic, never implements its own. Every MCP tool must have a CLI equivalent. See [[mcp-is-shim-cli-is-product]].

**Why:** Users debug with CLI. Agents discover with MCP. Same code path, different transport.

### 8. Specs authorize action

Every non-trivial change is authorized by a spec. Sessions discuss, specs decide, code executes. See [[spec-driven-design]].

**Why:** In agentic development, unauthorized scope creep is the default failure mode. Specs are the guardrail.

## The Evolution Path

Patina's architecture has a deliberate evolution path from monolith to platform:

```
Today:          enum dispatch → compiled code
                ReleaseStrategy::Cargo → internal.rs
                ForgeReader → github/internal.rs
                Oracle → semantic.rs

Plugin system:  enum dispatch → WIT interface → WASM module
                ReleaseStrategy → release.wit → release-cargo.wasm
                ForgeReader → forge.wit → forge-github.wasm
                Oracle → oracle.wit → oracle-semantic.wasm
```

The traits exist and WIT interfaces are shipping. v0.17.0 delivered two WIT worlds: `child` (daemon children — models, repos) and `command` (CLI commands — doctor). The refactor from enum to trait to WIT is mechanical — one extension point at a time, never a rewrite.

**Key constraint:** wasmtime + WIT Component Model. Not Extism. Separate WIT worlds per plugin type for capability isolation. Two-layer capability grants (manifest + host). See [[patina-platform]].

## Common Mistakes

### 1. Adding features that don't serve the six core functions

```
Bad:  "Let's add a code formatter to patina"
      → That's a build tool, not knowledge infrastructure

Good: "Let's add a scraper that indexes formatting rules"
      → That captures knowledge about the project's style
```

### 2. Building systems when you need tools

```
Bad:  "Let's build a task management system in patina"
      → That's patina-work plugin territory

Good: "Let's add belief grounding to assay"
      → That enhances the serve pillar with existing infrastructure
```

### 3. Coupling plugins to core internals

```
Bad:  forge reads from eventlog internals directly
Good: forge writes to eventlog via public API, assay reads via FTS5
```

### 4. Adding external dependencies for one-time operations

```
Bad:  Add `glob` crate for one call in release safeguards
Good: Use `toml::Value` (already in tree) for config parsing
      See [[use-whats-in-the-tree]]
```

### 5. Making the binary bigger instead of the layer richer

```
Bad:  Compile tree-sitter grammars for 30 languages into the binary
Good: Load grammars as WASM plugins from ~/.patina/grammars/

Bad:  Add LLM inference to the binary
Good: Generate context files that LLM tools consume
```

### 6. Confusing the layer with the binary

```
The layer (layer/) is the knowledge product — portable, git-tracked, human-readable
The binary (patina) is the pipeline — captures, indexes, serves, governs
The cache (.patina/) is derived state — rebuildable, gitignored, machine-readable

Don't store knowledge in the binary (hardcoded patterns)
Don't store pipeline logic in the layer (executable scripts)
Don't treat cache as durable (it's derived from layer + git)
```

## References

- [Dependable Rust](./dependable-rust.md) — Black-box module pattern (mod.rs + internal.rs)
- [Unix Philosophy](./unix-philosophy.md) — One tool, one job, done well
- [Adapter Pattern](./adapter-pattern.md) — Trait-based external system integration
- [Spec-Driven Design](./spec-driven-design.md) — Specs authorize action
- [Session Capture](./session-capture.md) — Friction-free knowledge capture
- [[compiler-enforced-safety]] — Type-level enforcement in agentic contexts
- [[transparent-complexity]] — Every code path visible to the compiler
- [[work-triages-specs]] — Let the build determine what matters
- [[patina-is-knowledge-layer]] — Git-style substrate, not LLM tool competitor
- [[sync-first]] — No async infection
- [[mcp-is-shim-cli-is-product]] — CLI is the real product
