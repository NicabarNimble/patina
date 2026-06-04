---
id: patina-identity
layer: core
status: active
created: 2026-02-11
revised: 2026-06-03
tags: [identity, architecture, core-principle, child-boundary, protocol]
references: [
  dependable-rust,
  unix-philosophy,
  adapter-pattern,
  spec-driven-design,
  core-primitives-are-not-children,
  core-verbs-standalone-mother-additive,
  patina-is-knowledge-protocol,
  patina-is-knowledge-layer,
  eventlog-is-truth,
  children-have-agency-toys-are-capabilities
]
---

# Patina Identity

**Purpose:** Define what Patina is, what stays in the protocol core, and what belongs in Mother, a child, a toy contract, or an external tool.

## Core Value

Patina is a **knowledge protocol for AI-assisted development**.

Five verbs define the protocol:

1. **Capture** development artifacts into durable local knowledge.
2. **Index** that knowledge into searchable projections.
3. **Search** project knowledge without pretending to reason.
4. **Believe** by recording project-scoped, evidence-backed assertions.
5. **Evolve** by letting sessions, specs, beliefs, and patterns mature over time.

Remove any verb and Patina stops being Patina.

**The CLI is the protocol surface. The binary is the protocol engine. The layer is the product. Mother is additive infrastructure.**

## Proofs

- **Standalone protocol proof:** `patina scrape`, `patina oxidize`, `patina scry`, `patina assay`, `patina context`, and `patina belief` are first-class CLI commands. They do not require a running Mother daemon for baseline local execution. See [[core-verbs-standalone-mother-additive]].
- **Layer proof:** `layer/` is git-tracked knowledge: core values, surface beliefs, sessions, specs, and evidence. `.patina/` is local derived state. See [[patina-is-knowledge-layer]].
- **Eventlog proof:** The append-only eventlog is the canonical substrate; SQLite tables are rebuildable projections. See [[eventlog-is-truth]].
- **Boundary proof:** current child manifests use `child.toml`, `[child]`, `kind`, and `[needs].toys`. Legacy `[capabilities]`, top-level `[toys]`, and plugin-era manifest vocabulary are not canonical.
- **Mother proof:** Mother owns cross-project routing, child orchestration, grants, readiness, secrets/session coordination, and observability. That makes Patina scale, but does not define the five protocol verbs.
- **Interface proof:** Claude, OpenCode, Gemini, and similar runtimes are external guests. They may receive Patina context through generated files, helper scripts, CLI flows, or configured adapters; they are not Patina's runtime.

## Current Vocabulary

- **Core** means the five protocol verbs and the minimum foundation needed to execute them.
- **Mother** means the local control plane around Patina: routing, project coordination, children, grants, and observability.
- **Child** means a sandboxed WASI/WASM component with a `child.toml` manifest.
- **Kind** means the runtime category declared by `child.kind`.
- **Toy** means a WIT-contracted capability granted to a child, such as filesystem, git, logging, measurement, state, or messaging.
- **World** is reserved for WIT component composition contexts, not runtime manifest vocabulary.
- **Plugin** is legacy compatibility vocabulary. Use `child` unless naming an explicit compatibility path or historical artifact.

## Protocol Boundaries

### Capture

`scrape` extracts local project knowledge: code structure, git history, layer files, schemas, connector facts, sessions, specs, and beliefs. It writes durable events and rebuildable projections.

Core modules include `scrape`, `scanner`, `eventlog`, `schema`, and `git`.

### Index

`oxidize` builds searchable representations: embeddings, FTS5 indexes, graphs, and other projections over captured knowledge.

Core modules include `oxidize`, `embeddings`, `models`, and retrieval storage.

### Search

`scry`, `assay`, and `context` retrieve relevant project knowledge. Search ranks and explains evidence; it does not plan, decide, or impersonate an agent.

Core modules include `scry`, `assay`, `context`, and `retrieval`.

### Believe

Beliefs are project-scoped assertions with evidence, supports, attacks, and decay. They are not global truth.

Core modules include `belief`, `layer`, and epistemic belief scraping.

### Evolve

The layer changes over time: sessions distill into beliefs, specs authorize work, beliefs gain or lose support, and stale knowledge moves out of the active surface.

Core surfaces include `layer/`, belief lifecycle data, session files, and spec-derived evidence. Session and spec commands are tooling over the protocol, not protocol verbs themselves.

## Foundation

These support the verbs and stay close to core:

- `paths`: path construction only; no hidden I/O policy.
- `project`: project configuration and identity.
- `models`: local embedding model management.
- `migration`: idempotent layout/version migration.
- `init`: project skeleton setup.
- `rebuild`: proof that local projections are derived from durable sources.

## Infrastructure

These support Patina at scale but are not the protocol itself:

- `mother`: daemon, routing, project registry, child orchestration, grants, and views.
- `interface`: external guest runtime setup for Claude, OpenCode, Gemini, and similar tools.
- `secrets`: vault, key handling, and secret grants.
- `child`: WASM component runtime, manifests, and typed calls.
- `toys`: WIT capability contracts used by children.
- `connect`: external service connection management.

## Extraction Rule

Keep the protocol standalone. Move behavior outward when it can live behind stable contracts.

- If it is one of the five verbs, keep it core.
- If it extends a verb, prefer a child or strategy boundary.
- If it coordinates host access, secrets, grants, or cross-project state, it belongs near Mother.
- If it is just a useful tool, keep it outside the binary or make it a child with explicit toys.
- If removing it does not break `patina scrape && patina scry "how does auth work?"`, it is not protocol core.

## Architectural Invariants

1. **Rust-first runtime:** no Python, Node, or shell-script runtime dependencies in core protocol paths.
2. **Local-first data:** core operation uses local files, local SQLite, local models, and git-tracked layer knowledge.
3. **Eventlog is truth:** derived tables can be rebuilt from eventlog, layer, and local sources.
4. **Layer is product:** `layer/` is reviewable knowledge; `.patina/` is rebuildable local state.
5. **Compiler-enforced safety:** prefer enums, typed boundaries, and checked contracts over stringly conventions.
6. **Sync-first execution:** blocking code by default; bounded parallelism when needed; async only where host integration requires it.
7. **CLI-first execution:** MCP and interface integrations are wrappers or discovery aids, not independent protocol implementations.
8. **Specs authorize action:** non-trivial changes need explicit behavioral intent.

## Non-Goals

- Not a build system: Patina may understand build files, but does not own compiling, testing, or deploying.
- Not a deployment tool: no CI/CD, cloud orchestration, or release hosting.
- Not an LLM runtime: external AI tools are guests; Patina supplies knowledge.
- Not a task tracker: specs and sessions govern knowledge and action, not sprints or Kanban.
- Not a general-purpose database: SQLite is implementation machinery, not the product.

## Common Mistakes

```text
Bad:  add a formatter to Patina
Good: capture and retrieve formatting conventions as project knowledge

Bad:  make Mother required for local scrape/scry
Good: keep Mother additive; core verbs stay CLI-standalone

Bad:  call new WASM extensions plugins
Good: call them children, declare kind, and grant toys explicitly

Bad:  let a child read eventlog internals directly
Good: expose typed facts and capabilities through public contracts

Bad:  compile every parser into the binary
Good: keep scrape as orchestrator and load grammar/pipeline children through WIT
```

## References

- [Dependable Rust](./dependable-rust.md)
- [Unix Philosophy](./unix-philosophy.md)
- [Adapter Pattern](./adapter-pattern.md)
- [Spec-Driven Design](./spec-driven-design.md)
- [Session Capture](./session-capture.md)
- [[core-primitives-are-not-children]]
- [[core-verbs-standalone-mother-additive]]
- [[patina-is-knowledge-protocol]]
- [[patina-is-knowledge-layer]]
- [[eventlog-is-truth]]
- [[children-have-agency-toys-are-capabilities]]
- [[mcp-is-discovery-cli-is-execution]]
