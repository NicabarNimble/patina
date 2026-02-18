---
id: patina-identity
layer: core
status: active
created: 2026-02-11
revised: 2026-02-13
tags: [identity, architecture, core-principle, plugin-boundary, protocol]
references: [dependable-rust, unix-philosophy, adapter-pattern, spec-driven-design]
---

# Patina Identity

**Purpose:** Define what Patina is and isn't, where the protocol core lives, and when something belongs in the binary versus a plugin. Every architectural decision flows from this document.

---

## Core Principle

Patina is a **knowledge protocol for AI-assisted development**. Five verbs define the protocol: **capture, index, search, believe, evolve**. Everything else is tooling built on the protocol or infrastructure that supports it.

**The binary is the pipeline. The layer is the product. The protocol is the contract.**

## The Protocol

Patina's irreducible core. These five operations define what Patina *is*. Remove any one and it stops being Patina.

### 1. Capture — Extract knowledge from development artifacts

Code structure, git history, layer files, forge data. The `scrape` pipeline reads your project and writes to SQLite + eventlog. This is the intake.

**Modules:** `scrape` (15K lines), `scanner`, `eventlog`, `forge`, `git`

### 2. Index — Build searchable representations

Embeddings from ONNX models, FTS5 indexes, structural graphs, temporal co-change matrices. The `oxidize` pipeline transforms raw scrape data into queryable form.

**Modules:** `oxidize`, `embeddings`, `models`, `db`

### 3. Search — Retrieve relevant knowledge about your project

Semantic search (scry), factual search (assay), pattern delivery (context), belief grounding. Search is retrieval, not reasoning — Patina finds and ranks, it doesn't plan or decide. AI agents consume search results via MCP or CLI.

**Modules:** `scry`, `assay`, `context`, `retrieval`, `mcp`

### 4. Believe — Capture and evolve project principles

Beliefs are project-scoped, evidence-backed assertions — not global truth. They capture decisions, patterns, and principles with supports/attacks relationships. Beliefs ground the protocol in project reality.

**Modules:** `belief`, `layer` (epistemic beliefs in layer/surface/epistemic/)

### 5. Evolve — Knowledge accumulates and matures

Patterns move through core → surface → dust. Sessions distill into beliefs. Beliefs gain or lose entrenchment through evidence. The layer is a living document.

**Modules:** `layer` (lifecycle), `session` (distillation)

### Foundation

These serve all five protocol operations:

- **`layer`** — filesystem structure for knowledge storage (core/surface/dust/sessions)
- **`paths`** — single source of truth for all path construction (no I/O)
- **`project`** — unified config management (.patina/config.toml)
- **`models`** — embedding models exist so Patina can work. No models, no scry.
- **`migration`** — path migration between versions (idempotent)
- **`init`** — project skeleton setup

## Protocol Tooling — uses the protocol, extractable over time

These modules use the protocol but aren't the protocol itself. Today they're compiled-in. As the plugin system matures and formats stabilize, they move to plugins. Nothing is lost — features move and the core hardens.

| Module | What it does | Extraction path |
|--------|-------------|-----------------|
| `spec` | Spec lifecycle + release delegation | Command plugin (reads/writes stable markdown format) |
| `release` | Version strategy dispatch | Command plugin (reads Cargo.toml, creates tags) |
| `session` | Development session tracking | Task plugin (needs host/git for tags) |
| `version` | Version display + tracking | Command plugin |
| `report` | Project state reports | Command plugin (composes from scry/assay/context) |
| `eval` + `bench` | Retrieval quality measurement | Likely stays compiled — value is ablation testing of retrieval internals |
| `doctor` | Health checks | **Extracted (v0.17.0)** — first plugin, proves the pattern |
| `yolo` | Devcontainer generation | Task plugin (mutates filesystem) |
| `upgrade` | Version check | Command plugin (task if it downloads/replaces binary) |

### Extraction principle

Per [[graceful-extraction]]: keep the compiled version as a feature-gated fallback. Plugin-first dispatch with compiled fallback means the system works regardless of plugin availability. The compiled path is only removed after the plugin path is proven stable.

## Protocol Infrastructure — supports the protocol, stays longest

These modules provide infrastructure the protocol needs to operate at scale. They stay in the binary longest because they need full host access or provide cross-cutting concerns.

| Module | What it does | Why it stays |
|--------|-------------|-------------|
| `mother` | Cross-project daemon, routing, plugin management | The router that connects everything. Mother IS how Patina scales. |
| `adapters` | LLM entry points (Claude, Gemini, OpenCode) | Need full host access (auth, APIs, secrets). Not sandboxable. |
| `secrets` | Age encryption + Keychain | Security infrastructure. Plugins consume secrets; they don't manage them. |
| `plugin` | WASM engine (wasmtime, WIT, 4 worlds) | The extraction mechanism itself. |

## What Patina IS NOT

### Not a build system

Patina doesn't compile code, run tests, or deploy artifacts. It *understands* build artifacts (Cargo.toml, package.json) to auto-detect project type, but it never executes builds.

### Not a deployment tool

No CI/CD, no container orchestration, no cloud integration.

### Not an LLM runtime

Adapters are config generators, not inference engines. The MCP server exposes Patina's *knowledge* tools to LLMs — it doesn't host LLM capabilities.

### Not a task tracker

Session tracking and spec lifecycle are governance tooling, not project management. No sprints, assignees, or Kanban boards.

### Not a general-purpose database

SQLite is an implementation detail. The eventlog is an append-only knowledge substrate, not an application database.

## The Protocol Test

Before adding ANY new module to the binary, apply this test:

### 1. Is it a protocol operation?

Capture, index, search, believe, or evolve. If yes → protocol core. If no → continue.

### 2. Does it use the protocol?

If it reads/writes layer data, queries scry/assay/context, or manages beliefs → it's protocol tooling. Compile it in today, plan for extraction.

### 3. Does it provide infrastructure the protocol needs?

Cross-project routing, security, plugin hosting → protocol infrastructure. Stays in the binary.

### 4. None of the above?

It's a plugin. Don't add it to the binary. Build it as a WASM plugin from day one.

### 5. Can Patina function without it?

`patina scrape && patina scry "how does auth work?"` must work without eval, yolo, doctor, or report installed. If removing it breaks the protocol → it's core. If not → plugin or tooling.

### When in doubt: bundle now, extract later

The plugin system is live (v0.17.0) with four planned worlds: pipeline (pure compute), command (inform), task (act), mother-child (daemon). Doctor was the first extraction. The boundary moves outward over time — tooling first, infrastructure last.

## Architectural Invariants

Non-negotiable properties of the system:

### 1. Rust-first runtime

No Python subprocess dependencies. No Node.js. No shell scripts at runtime. Embeddings run through ONNX Runtime via `ort` crate. Cross-platform: same vector space on Mac/Linux/Windows, but Patina itself targets macOS and Linux (symlinks, Unix sockets, and filesystem invariants are POSIX-only assumptions).

### 2. Local-first data

All knowledge lives on disk. SQLite databases, markdown files, TOML configs. No cloud dependencies for core operation.

### 3. Eventlog is truth

The append-only eventlog is the canonical data source. All SQLite tables are materialized views — derived, rebuildable. `patina rebuild` recreates everything from eventlog + layer/ + git.

### 4. Layer is git-tracked knowledge

`layer/` is the knowledge product — checked into git, versioned, reviewable. `.patina/` is derived local state — gitignored, rebuildable.

### 5. Compiler-enforced safety

In agentic contexts, the compiler is the only reliable review gate. Prefer enums over strings, typestate over documentation. See [[compiler-enforced-safety]].

### 6. Sync-first execution

Synchronous, blocking code by default. Parallelism is explicit and bounded (`std::thread::scope`, `rayon`); async only when host integration requires it. See [[sync-first]].

### 7. MCP is shim, CLI is product

The MCP server wraps CLI logic, never implements its own. See [[mcp-is-shim-cli-is-product]].

### 8. Specs authorize action

Every non-trivial change is authorized by a spec. See [[spec-driven-design]].

## The Evolution Path

```
Protocol core:    capture, index, search, believe, evolve
                  → Always in the binary. Hardened by extraction.

Protocol tooling: spec, session, release, eval, report, yolo, upgrade
                  → Compiled today. Plugin tomorrow. Formats stabilize first.

Protocol infra:   mother, adapters, secrets, plugin engine
                  → Stays in the binary longest. Full host access required.

Plugin ecosystem: 4 worlds (pipeline, command, task, mother-child)
                  → Community extends Patina without touching core.
```

The plugin system (v0.17.0+) enables this evolution. Each extraction hardens the protocol core by proving it doesn't need the extracted module to function. The binary gets smaller and more stable. The ecosystem gets richer.

## Common Mistakes

### 1. Adding features that don't serve the protocol

```
Bad:  "Let's add a code formatter to patina"
Good: "Let's add a scraper that indexes formatting rules"
```

### 2. Building systems when you need tools

```
Bad:  "Let's build a task management system in patina"
Good: "Let's add belief grounding to assay"
```

### 3. Coupling plugins to core internals

```
Bad:  forge reads from eventlog internals directly
Good: forge writes to eventlog via public API, assay reads via FTS5
```

### 4. Making the binary bigger instead of the layer richer

```
Bad:  Compile tree-sitter grammars for 30 languages into the binary
Good: Load grammars as WASM plugins from ~/.patina/pipeline/
```

### 5. Confusing the layer with the binary

```
The layer (layer/) is the knowledge product
The binary (patina) is the protocol engine
The cache (.patina/) is derived state
```

## References

- [Dependable Rust](./dependable-rust.md) — Black-box module pattern
- [Unix Philosophy](./unix-philosophy.md) — One tool, one job, done well
- [Adapter Pattern](./adapter-pattern.md) — Trait-based external system integration
- [Spec-Driven Design](./spec-driven-design.md) — Specs authorize action
- [Session Capture](./session-capture.md) — Friction-free knowledge capture
- [[compiler-enforced-safety]] — Type-level enforcement in agentic contexts
- [[patina-is-knowledge-protocol]] — Protocol distillation principle
- [[patina-is-knowledge-layer]] — Git-style substrate, not LLM tool competitor
- [[graceful-extraction]] — Plugin-first with compiled fallback
- [[sync-first]] — No async infection
- [[mcp-is-shim-cli-is-product]] — CLI is the real product
