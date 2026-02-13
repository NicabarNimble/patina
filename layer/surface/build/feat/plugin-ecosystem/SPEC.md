---
type: feat
id: plugin-ecosystem
status: design
created: 2026-02-13
sessions:
  origin: 20260213-055346
blocked_by: []
blocks: []
related:
  - layer/surface/build/feat/plugin-system/SPEC.md
  - layer/surface/build/feat/plugin-command-extractions/SPEC.md
  - layer/surface/build/feat/plugin-oracle-scraper/SPEC.md
  - layer/surface/build/feat/plugin-grammars/SPEC.md
beliefs:
  - patina-is-knowledge-protocol
  - plugin-is-agent-plus-skill
  - separate-worlds-for-isolation
  - two-layer-capability-grants
  - patina-is-knowledge-layer
  - skills-for-structured-output
  - mother-is-the-daemon
references:
  - "Obsidian community plugin system (2000+ plugins, TypeScript, unsandboxed)"
  - "Zed extension system (wasmtime, WIT, wasm32-wasip1, capability-gated)"
  - "Session 20260213-055346: Zed/Obsidian/Patina comparative analysis"
---

# feat: Plugin Ecosystem

> Obsidian-level accessibility. WASM-level safety. LLM-authored plugins.
> A plugin is a bundle of agent (WASM) + skill (prompt) + manifest (capabilities).
> Four worlds by calling convention: pipeline (host-invoked pure compute),
> command (user-invoked intelligence), task (user-invoked action),
> mother-child (daemon continuous action). Three zones as taxonomy:
> Pipeline, Intelligence, Action. This is a design document — it frames
> the vision and identifies the work. Implementation specs follow.

## Problem

Patina has a working plugin system (v0.17.0) — wasmtime, WIT Component Model,
two worlds (mother-child, command), three first-party plugins, capability grants,
toy allowlists. The runtime works. But the ecosystem doesn't exist:

1. **No community story** — plugins are hand-built by us, manually installed
2. **No LLM authoring path** — despite LLMs being the primary consumer of Patina
3. **No skill registration** — skills are baked into adapter templates at compile
   time; plugins can't ship skills
4. **No knowledge access** — plugins can log and read project config, but can't
   search beliefs, scry, assay, or context
5. **No install command** — manual `.wasm` + `.toml` file placement
6. **No template** — no way for an LLM to bootstrap a plugin project

The existing Phase 3-5 specs ([[plugin-command-extractions]], [[plugin-oracle-scraper]],
[[plugin-grammars]]) focus on extracting existing code into WASM. This spec focuses
on making the plugin system usable by others — the ecosystem around the runtime.

## Core Insight: Patina Is a Knowledge Protocol

Per [[patina-is-knowledge-protocol]]: Patina is to development knowledge what
git is to version control. The protocol has five verbs:

```
CAPTURE → INDEX → SEARCH → BELIEVE → EVOLVE
```

These produce outputs — artifacts, results, decisions, health signals, deltas.
The protocol does NOT include "action." Like git doesn't have "deploy" as a
verb, Patina doesn't have "act." Extensions may act on protocol outputs, but
action is a side-effect of extensions, not a protocol phase.

The core — capture (scrape), index (oxidize), search (scry/assay), believe
(epistemic layer), evolve (sessions/patterns) — should work standalone and
LLM-agnostic. Everything else belongs in the plugin ecosystem.

This reframes the entire extraction roadmap. Phases 3-5 ([[plugin-command-extractions]],
[[plugin-oracle-scraper]], [[plugin-grammars]]) are not binary size optimization —
they are **protocol distillation**. Each extraction trims the core toward its
protocol essence while the ecosystem grows to handle the rest.

### Governance Principle

> **If changing it would break every plugin, it's protocol.
> If changing it only affects one use case, it's a plugin.**

Corollary: if something can be A/B tested per project without breaking
artifacts, lean plugin. If it affects artifact semantics (layer format,
belief schema, session schema), it's core.

## Core Insight: LLMs Change the Plugin Equation

The traditional barrier argument ("Rust+WASM is too hard for community plugins")
is outdated. LLMs can author Rust WASM plugins trivially. The design question
shifts from "make it easy for humans to write" to:

**Make it easy for LLMs to generate, users to install, and the sandbox to enforce.**

This is the Obsidian lesson applied to the LLM era: Obsidian succeeded because
the path from "I want X" to "X works" is short. Patina can achieve the same
with a different surface — instead of "write JavaScript," it's "describe what
you want" → LLM generates Rust → WASM sandbox enforces safety → user approves
capabilities, not code.

Together these insights define the design space: a **stable knowledge protocol**
with a **safe, LLM-friendly extension surface**.

---

## The Bundle: Agent + Skill + Manifest

Per [[plugin-is-agent-plus-skill]], a Patina plugin is a bundle of up to three parts:

| Part | What | Runs where | Required? |
|------|------|-----------|-----------|
| **Agent** | WASM binary | System (daemon or CLI) | Yes |
| **Skill** | Prompt template (markdown) | LLM session (adapter) | Optional |
| **Manifest** | Capability declarations | Install time (approval) | Yes |

**The agent** is the sensor/executor — deterministic code running in the WASM
sandbox. It detects conditions, computes results, emits toys.

**The skill** is the playbook — a prompt template that tells the LLM how to
interpret agent output and what actions to recommend. Skills are the LLM-facing
API to the plugin.

**The manifest** declares what the bundle needs — host functions, toy commands,
query access. Users approve the manifest at install time.

### Why all three?

| Without skill | Agent detects "belief X is stale" → user sees raw output, doesn't know what to do |
|---------------|-----------------------------------------------------------------------------------|
| Without agent | Skill tells LLM "check belief freshness" → LLM has no data source, can't check |
| **With both** | Agent detects staleness → skill tells LLM how to investigate and update → user gets actionable guidance |

Not all plugins need all three parts. Pipeline plugins (swap an embedding model)
may need only agent + manifest. Pure analysis tools may need only a command
agent. The bundle is a packaging concept, not a rigid requirement.

### Skill Registration

Today skills are compile-time embedded in adapter templates (`resources/claude/skills/`).
Plugins need to register skills at install time:

```
~/.patina/plugins/belief-checker/
├── belief-checker.wasm      # Agent
├── plugin.toml              # Manifest
└── skills/
    └── check-beliefs.md     # Skill (adapter-agnostic or per-adapter)
```

When `patina adapter add claude` runs (or on plugin install), registered skills
get injected into the adapter's skill discovery path. The adapter's existing
skill loading mechanism picks them up.

**Open question:** Are skills adapter-agnostic (one markdown works for all LLMs)
or adapter-specific (separate files for Claude, Gemini, OpenCode)? Initial
direction: adapter-agnostic markdown with structured sections the adapter
can interpret.

---

## Four Worlds = Four Execution Contracts

The 10-scenario walkthrough (session [[20260213-055346]]) revealed that the
real distinction between plugin types is **calling convention** — who invokes
the plugin, what its lifecycle is, and what capabilities the contract provides.

Worlds are execution contracts, not capability bundles. Each is a different
runtime lifecycle. Capabilities (query, http, toys) are host interfaces
gated by the manifest within a world.

| World | Invoked by | Lifecycle | Protocol boundary |
|-------|-----------|-----------|-------------------|
| `pipeline` | Host (during scrape/index) | Per-call | Extends Capture + Index |
| `command` | User (CLI) | One-shot | Extends Search + Believe (read) |
| `task` | User (CLI) | One-shot | Acts on Search/Believe outputs |
| `mother-child` | Daemon (heartbeat) | Long-lived | Monitors Evolve, acts on signals |

Note: task and mother-child don't extend the protocol — they extend
*outward from it*. The protocol produces outputs (results, decisions,
signals). These worlds act on those outputs. Like CI acts on git commits.

Each world is a genuinely different execution contract. Scenarios that
break cleanly under one world don't fit in the others:

- Grammar plugin (`parse(bytes) -> tree`) is host-invoked pure compute —
  can't be a `command` because `run(args) -> exit_code` is wrong shape.
- Belief auditor is read-only analysis — shouldn't have toys.
- PR reviewer needs both query AND toys, but runs on-demand — can't be
  mother-child (requires daemon) or command (no toys).
- Cloudflare connector is continuous monitoring — must be mother-child.

### Pipeline World: Host-Invoked Pure Compute

**Key design principle:** Push all side effects into the host. Pipeline
plugins are pure functions — bytes in, results out. No filesystem, no
network, no model loading. The host does all I/O and passes data through
the WASM boundary.

```wit
world pipeline {
    import patina:host/log@0.1.0;

    export init: func();
    export name: func() -> string;

    // Grammar plugins
    export parse: func(source: list<u8>, language: string) -> result<string, string>;

    // Chunking plugins
    export chunk: func(source: list<u8>, language: string) -> result<list<string>, string>;
}
```

The host checks `[provides]` in the manifest to know which exports to call.
Each plugin implements its subset. The world's WIT grows additively as new
pipeline types are added — this is safe because `log` is the only import,
so isolation stays tight regardless of which exports exist.

**Why this works:** A grammar plugin literally cannot exfiltrate because it
has no imports to call. `wit_bindgen` on the guest side only generates
bindings for what the guest world imports. No `query()`, no `http()`, no
filesystem — not enforced at runtime, enforced at compile time.

**Embedding model note:** ONNX inference via `ort` requires native C++
libraries that can't run inside WASM. Per [[patina-identity]], ONNX runtime
is "Foundation" — it stays in core. Embedding "plugins" would provide
model metadata, tokenizer logic, and pre/post processing (like E5's
`"query: "` prefix) as pure computation. The host always runs inference.

### Command World: User-Invoked Intelligence (existing)

Read-only analysis tools. Can query the knowledge base but cannot spawn
processes or make network calls.

```wit
world command {
    import patina:host/log@0.1.0;
    import patina:host/query@0.1.0;  // NEW
    import patina:host/layer@0.1.0;

    export init: func();
    export name: func() -> string;
    export description: func() -> string;
    export run: func(args: list<string>) -> s32;
}
```

**Psychologically safe:** When a user runs `patina audit`, they expect
read-only analysis. Commands don't act, they inform. The absence of toys
makes this expectation explicit at the world level.

### Task World: User-Invoked Action (new)

On-demand plugins that can both analyze AND act. Runs once, returns
analysis plus a list of toys for the host to execute.

```wit
world task {
    import patina:host/log@0.1.0;
    import patina:host/query@0.1.0;
    import patina:host/layer@0.1.0;
    import patina:host/types@0.1.0;
    import patina:host/http@0.1.0;   // NEW — domain-allowlisted

    export init: func();
    export name: func() -> string;
    export description: func() -> string;
    export run: func(args: list<string>) -> s32;
    export toys: func() -> list<toy>;
}
```

After `run()` returns, the host checks `toys()` and executes them (with
capability gating). The plugin says "here's my analysis AND here's what
to do about it." The host decides whether to actually run the toys.

**Covers the PR-reviewer gap:** needs query (beliefs, context) + toys
(`gh pr review`) but runs on-demand, not on heartbeat. Doesn't require
the daemon.

### Mother-Child World: Daemon Continuous Action (existing)

Long-lived agents with heartbeat lifecycle. Monitor, detect, react.

```wit
world mother-child {
    import patina:host/log@0.1.0;
    import patina:host/query@0.1.0;  // NEW
    import patina:host/types@0.1.0;
    import patina:host/http@0.1.0;   // NEW — domain-allowlisted

    export init: func();
    export name: func() -> string;
    export on-load: func() -> result<_, string>;
    export on-unload: func();
    export health: func() -> child-health;
    export handle: func(action: string, payload: string) -> result<string, string>;
    export tick: func() -> list<toy>;
}
```

---

## Three Zones (Taxonomy, Not Architecture)

Zones are a **user-facing taxonomy** — they describe what a plugin is *for*,
not how it runs. The four worlds describe *how* it runs. A zone maps to one
or more worlds:

### Zone 1: Pipeline (customize how Patina processes knowledge)

| Plugin type | What it does | World | Host provides |
|-------------|-------------|-------|--------------|
| Grammar (Zig, Gleam, Cairo) | `parse(bytes) -> tree` | pipeline | Source bytes |
| Custom chunking | `chunk(bytes, lang) -> chunks` | pipeline | Source bytes |
| Tokenizer/prefix | Pre/post-process for embedding model | pipeline | Text |
| Embedding config | Model metadata + dimensions | command | N/A (config only) |

These map to [[plugin-oracle-scraper]] and [[plugin-grammars]] specs.
Pipeline zone plugins are the simplest — minimal capabilities, pure compute.

### Zone 2: Intelligence (query and analyze knowledge)

| Plugin type | What it does | World |
|-------------|-------------|-------|
| Belief auditor | Cross-reference beliefs against evidence | command |
| Pattern recommender | Suggest beliefs from similar projects | command |
| Knowledge export | Format for Obsidian/Notion/etc. | command |
| Changelog generator | Session + git history → changelog | command |
| Onboarding guide | Generate new-dev guide from patterns | command |

**Requires the `query` host interface.** Without it, command plugins can
only read project config. With it, they search beliefs, scry semantic
results, and run assay queries.

### Zone 3: Action (do things with knowledge)

| Plugin type | What it does | World |
|-------------|-------------|-------|
| PR reviewer | Analyze diff against beliefs, post review | task |
| One-shot deploy | Check readiness, run wrangler deploy | task |
| Cloudflare connector | Monitor CI, auto-deploy | mother-child |
| GitHub connector | Create issues from degraded beliefs | mother-child |
| Slack notifier | Alert on belief health changes | mother-child |
| Test health monitor | Track pass/fail rates, detect flaky tests | mother-child |
| Cross-project sync | Watch ref repos for relevant changes | mother-child |

Action plugins split between `task` (on-demand) and `mother-child`
(continuous). Both have toys and query access. The difference is lifecycle:
task runs once and exits; mother-child ticks forever.

**Each action plugin benefits from shipping with a skill** — the agent
detects/acts, the skill tells the LLM how to interpret results and guide
the user. This is the bundle model in action.

---

## Design Gaps (Build Targets)

Gaps are organized into three categories: host interfaces (capabilities
plugins can use), worlds (execution contracts), and ecosystem tooling
(install, template, UX). These are different layers — interfaces live
inside worlds, tooling wraps around them.

### Host Interfaces (capabilities within worlds)

These are `patina:host/*` WIT interfaces that plugins import. Each is
independently capability-gated in the manifest. A world *defines* which
interfaces are available; the manifest *controls* which are active.

#### Interface: Query (`patina:host/query@0.1.0`)

**Priority: High | Effort: Small**

The MCP server (`src/mcp/server.rs`) already exposes scry, assay, and context
as JSON-in/string-out over JSON-RPC. The query host interface wraps the same
dispatch for WASM plugins.

```wit
interface query {
    /// Query the knowledge base.
    /// kind: "scry" | "assay" | "context" | "beliefs"
    /// params: JSON string matching the MCP tool input schema
    /// Returns formatted results or error
    query: func(kind: string, params: string) -> result<string, string>;
}
```

**Manifest gating — kinds and scope:**
```toml
[capabilities]
host_query = ["scry", "context"]           # Which query kinds
query_scope = "current_project"            # current_project | allowed_repos | all_repos
# query_budget = { max_rows = 100 }       # Optional: prevent runaway queries
```

Query scope is a first-class capability. A mother-child plugin searching
`all_repos` on every heartbeat tick is very different from one searching
`current_project`. Make this explicit in the manifest so users see it
at install time.

**Available in worlds:** command, task, mother-child.
**Not in:** pipeline (pure compute — no host queries by design).

**Implementation:** Host-side dispatch reuses MCP server's JSON parsing logic.
Guest API crates re-export `query()` function.

#### Interface: HTTP (`patina:host/http@0.1.0`)

**Priority: High | Effort: Small**

Webhooks (Slack, GitHub) need HTTP access. Raw `curl` toys are dangerous —
command-level toy allowlists don't gate arguments. A host-provided HTTP
interface with domain allowlisting is cleaner and safer.

```wit
interface http {
    /// POST to an allowed domain. Host enforces domain allowlist.
    /// Returns response body or error.
    http-post: func(url: string, body: string, content-type: string) -> result<string, string>;

    /// GET from an allowed domain.
    http-get: func(url: string) -> result<string, string>;
}
```

**Manifest gating:**
```toml
[capabilities]
host_http = ["hooks.slack.com", "api.github.com"]
```

**Available in worlds:** task, mother-child.
**Not in:** pipeline (pure compute), command (read-only — commands inform,
they don't reach out).

The plugin never sees `curl`. The host handles TLS, connection pooling, and
domain enforcement. The toy system stays for local processes (git, cargo,
wrangler). HTTP is a separate capability.

#### Summary: Interface × World Matrix

| Interface | pipeline | command | task | mother-child |
|-----------|----------|---------|------|-------------|
| `log` | ✓ | ✓ | ✓ | ✓ |
| `layer` | — | ✓ | ✓ | ✓ |
| `query` | — | ✓ | ✓ | ✓ |
| `types` | — | — | ✓ | ✓ |
| `http` | — | — | ✓ | ✓ |
| toys | — | — | ✓ | ✓ |

Each ✓ means the world's WIT imports the interface. Each — means the
guest crate won't even have bindings for it (compile-time isolation).
Within a ✓, the manifest further gates which specific capabilities are
active (e.g., `host_query = ["scry"]` not `["scry", "assay", "beliefs"]`).

---

### Worlds (execution contracts)

These are new WIT worlds to add alongside the existing `command` and
`mother-child`.

#### World: Task (new)

**Priority: High | Effort: Medium**

On-demand action plugins. Same shape as command (`run(args) -> exit_code`)
but with toys and HTTP access. Covers scenarios that need both intelligence
and action but not a running daemon.

**Requires:**
- New WIT file: `wit/task/task.wit`
- New guest API crate: `patina-task-api`
- New engine variant in `src/plugin/internal/` (or extend CommandEngine)
- `patina plugin new --world task` template support

**Manifest example:**
```toml
[plugin]
name = "pr-reviewer"
world = "task"
[capabilities]
host_log = true
host_query = ["beliefs", "context"]
host_http = ["api.github.com"]
[capabilities.toys]
commands = ["gh"]
[provides]
commands = ["review-pr"]
```

#### World: Pipeline (new)

**Priority: Medium | Effort: Medium**

Host-invoked pure-compute plugins. Replaces the planned `oracle`, `scraper`,
and `grammar` worlds with a single world. All side effects pushed into the
host — the plugin is a pure function.

**Requires:**
- New WIT file: `wit/pipeline/pipeline.wit`
- New guest API crate: `patina-pipeline-api`
- Host-side integration with scrape and query engines (host calls plugin
  exports during its own operations)
- `patina plugin new --world pipeline` template support

**Key constraint:** `log` is the only import. No query, no layer, no HTTP,
no toys. `wit_bindgen` on the guest side won't generate bindings for
capabilities the world doesn't import — compile-time isolation, not
runtime enforcement.

**Subsumes:** [[plugin-oracle-scraper]] scraper world, [[plugin-grammars]]
grammar world. The oracle world may still be needed for plugins that provide
alternative search backends (requires further design — see Open Questions).

---

### Ecosystem Tooling (install, template, UX)

These are CLI features and developer experience that wrap around the
worlds and interfaces.

#### Tooling: One-Command Install

**Priority: High | Effort: Medium**

```bash
# Local crate (LLM just generated this)
patina plugin install ./my-plugin/

# GitHub repo
patina plugin install github.com/user/patina-plugin-foo

# Pre-built WASM
patina plugin install ./plugin.wasm --manifest ./plugin.toml
```

**Install flow:**

1. **Resolve source** — local dir, GitHub URL, or pre-built WASM
2. **Build if needed** — `cargo build --target wasm32-wasip2` for Rust sources
3. **Parse manifest** — read `plugin.toml`, validate required fields
4. **Validate WASM** — check exports match declared world, check API version
5. **Show capabilities** — display what the plugin needs (Gap 7)
6. **User approval** — approve or deny capabilities
7. **Place files** — copy to correct dir based on world:
   - `pipeline` → `~/.patina/pipeline/`
   - `command` → `~/.patina/plugins/`
   - `task` → `~/.patina/plugins/`
   - `mother-child` → `~/.patina/children/`
8. **Register skills** — copy skill files to discoverable location
9. **Update registry** — write to `~/.patina/plugin-cache.toml`

**Builds on existing infrastructure:**
- `PluginManifest::from_path()` — manifest parsing
- `paths::plugin::*` — path construction
- `repo` command's `registry.yaml` — registry pattern
- `reqwest::blocking` — HTTP for GitHub sources

**Source vs binary decision:** Support both. Source-first for LLM workflow
(generate → build → install). Pre-built for sharing (trust the sandbox).

#### Tooling: Plugin Template

**Priority: Medium | Effort: Small**

A `patina plugin new <name> --world <world>` command that scaffolds a
minimal plugin project:

```
my-plugin/
├── Cargo.toml          # [lib] crate-type = ["cdylib"]
├── plugin.toml         # manifest with capabilities
├── src/
│   └── lib.rs          # trait impl + register macro (~30 lines)
└── skills/             # optional skill directory
    └── my-skill.md     # optional skill template
```

**Four templates, one per world:**
- `--world pipeline` → `patina-pipeline-api` dep, `parse`/`chunk` exports
- `--world command` → `patina-command-api` dep, `run(args)` export
- `--world task` → `patina-task-api` dep, `run(args)` + `toys()` exports
- `--world mother-child` → `patina-plugin-api` dep, `handle`/`tick`/`health`

**Requirements for LLM-friendliness:**
- Guest API crates must be on crates.io or installable via git dependency
- Each trait is tiny (~30 lines for a complete plugin)
- `register_*!` macros hide all WIT bindgen boilerplate
- Template includes comments explaining each capability option

**Future:** `patina plugin generate "description"` — Patina prompts an LLM
to generate the plugin from a natural language description. Intent → plugin,
zero Rust knowledge required.

#### Tooling: Capability Approval UX

**Priority: Medium | Effort: Small**

Install-time capability display and approval:

```
$ patina plugin install ./pr-reviewer/

  Plugin: pr-reviewer v0.1.0
  World:  task

  Capabilities requested:
    ✓ host_log     — write to log
    ✓ host_query   — search: beliefs, context
    ✓ host_http    — POST to: api.github.com
    ✓ toys         — commands: gh
    ✗ host_layer   — not requested

  Approve? [Y/n]
```

**Grants persisted in** `~/.patina/plugin-grants.toml`:
```toml
[pr-reviewer]
host_log = true
host_query = ["beliefs", "context"]
host_http = ["api.github.com"]
toys = ["gh"]
approved_at = "2026-02-13T10:00:00Z"
```

User is not re-prompted unless capabilities change on update.

**Zed comparison:** Zed stores grants in user settings, no install-time prompt.
Patina's install-time prompt is friendlier for non-technical users who
won't edit TOML settings files.

---

## Relationship to Existing Specs

This spec is a **vision document** — it frames the "what and why" of the plugin
ecosystem. The existing specs are the "how" for specific phases:

| Spec | Role | Status | Impact of 4-world model |
|------|------|--------|------------------------|
| [[plugin-system]] | Runtime (wasmtime, WIT, 2 worlds) | complete | Foundation stays. Task + pipeline worlds are additive. |
| [[plugin-command-extractions]] | Extract yolo, eval, bench, etc. | design | Extractions become command or task world plugins. |
| [[plugin-oracle-scraper]] | Extensible oracle + scraper | design | Scraper subsumes into pipeline world. Oracle TBD — may need own world if search backends require host imports beyond log. |
| [[plugin-grammars]] | Tree-sitter from WASM | design | Subsumes into pipeline world (`parse` export). |
| **This spec** | Ecosystem (4 worlds, install, template, query, skills) | design | All zones |

The seven gaps in this spec are **independent of the extraction specs** — they
can be built in parallel. The query interface, install command, template, and
capability UX don't require extracting yolo or grammars first.

**Suggested build order:**

*Host interfaces first (they unlock plugin usefulness):*
1. `patina:host/query` — unlocks Intelligence and Action zone plugins
2. `patina:host/http` — unlocks webhook/API action plugins safely

*Worlds next (new execution contracts):*
3. `task` world — covers on-demand action gap (PR reviewer, one-shot deploy)
4. `pipeline` world — enables grammar/chunking community plugins

*Ecosystem tooling last (wraps around the above):*
5. Capability approval UX — required before any community plugin use
6. Plugin template + `patina plugin new` — enables LLM authoring
7. `patina plugin install` — enables distribution
8. Skill registration — closes the agent+skill loop

---

## Extension Points Outside the 4 Worlds

### LLM Adapters (host-side, not WASM)

Adapters (Claude, Gemini, OpenCode) are how LLMs consume the protocol.
They handle auth, API transport, prompt orchestration, rate limits, and
secrets. They are NOT candidates for WASM sandboxing — they need full
host access by nature.

Adapters are a **5th extension point** but live outside the 4-world system:
- Currently compiled-in (`src/adapters/`)
- Skills (the prompt half of the bundle) are adapter-augmentation, not a
  separate world
- If adapters ever become pluggable, they'd be host-side plugins (dylib or
  trait objects), not WASM

The 4 worlds handle system-side extensions. Adapters handle LLM-side
extensions. These are orthogonal.

---

## What We Don't Build (Yet)

- **Plugin marketplace/registry** — ref repos and local install first
- **Hot reloading** — restart to load. KISS.
- **Plugin dependencies** — plugins don't depend on other plugins
- **Cross-adapter skill translation** — adapter-agnostic skills first,
  translation if that doesn't work
- **`patina plugin generate`** — the LLM-generation command. Design the
  template and install path first; generation is a layer on top.
- **Plugin auto-update** — manual for now
- **WASM-sandboxed adapters** — adapters stay host-side (see above)

## Open Questions

1. **Skill format:** Adapter-agnostic markdown or per-adapter variants?
   Initial bet: agnostic, with structured sections adapters can parse.

2. **Query interface scope:** Should `query` also support writes (create
   belief, add session note) or stay read-only? Initial bet: read-only.
   Writes go through the CLI or a separate `mutate` interface.

3. **Guest API distribution:** Publish `patina-command-api`,
   `patina-plugin-api`, `patina-task-api`, and `patina-pipeline-api` to
   crates.io, or use git dependencies? Crates.io is friendlier for LLM
   generation but requires version management for 4 crates.

4. **Project-scoped plugins:** Should plugins install to project `.patina/plugins/`
   in addition to user `~/.patina/plugins/`? Useful for project-specific
   tooling but complicates discovery.

5. **Oracle world fate:** Does oracle (search backend swap) subsume into
   pipeline, or does it need its own world? A search backend may need host
   imports (database access, index files) that violate pipeline's pure-compute
   constraint. If so, it stays as a 5th world.

6. **Task vs command boundary:** Should command ever get optional toys (blurring
   the line), or is the hard split between "inform" (command) and "act" (task)
   worth the extra world? Current bet: hard split, worth it for UX clarity.

---

## Status Log

| Date | Status | Note |
|------|--------|------|
| 2026-02-13 | design | Created from Zed/Obsidian comparative analysis session [[20260213-055346]]. Frames three-zone model, bundle concept, four design gaps. Belief [[plugin-is-agent-plus-skill]] captured. |
| 2026-02-13 | amended | 10-scenario walkthrough validated 4-world model: pipeline (host-invoked pure compute), command (user-invoked intelligence), task (user-invoked action), mother-child (daemon continuous action). Calling convention is the real distinction. Pipeline defined as pure compute — all side effects pushed into host. Task world added to cover on-demand action gap (PR reviewer, one-shot deploy). HTTP host interface added for webhook safety (domain allowlisting replaces raw curl toys). Design gaps expanded from 4 to 7. Zones retained as user-facing taxonomy, not architecture. |
| 2026-02-13 | amended | External review refinements. **(1)** Action removed from protocol spine — protocol verbs are capture/index/search/believe/evolve only. Task and mother-child act on protocol *outputs*, not as protocol phases. **(2)** Query scope added as first-class capability: `query_scope = current_project \| allowed_repos \| all_repos` + optional `query_budget`. **(3)** Governance principle elevated: "if changing it would break every plugin, it's protocol." **(4)** Adapters explicitly placed outside 4-world system as host-side extension point (auth, APIs, secrets require full host access). **(5)** Worlds reframed as execution contracts, not capability bundles. Belief [[patina-is-knowledge-protocol]] updated. |
