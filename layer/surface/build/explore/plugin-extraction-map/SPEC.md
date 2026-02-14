---
type: explore
id: plugin-extraction-map
status: active
created: 2026-02-14
sessions:
  origin: 20260214-104350
beliefs:
- graceful-extraction
- separate-worlds-for-isolation
- plugin-is-agent-plus-skill
- wasi-sandboxed-filesystem
- two-layer-capability-grants
references:
- layer/core/patina-identity.md
- layer/core/dependable-rust.md
- layer/core/unix-philosophy.md
- layer/core/adapter-pattern.md
---

# explore: Plugin Extraction Map

> Systematic assessment of every module the identity document marks as
> "protocol tooling" (extractable), grounded in actual code coupling,
> WIT interface readiness, and format stability. Determines extraction
> order and identifies blockers.

## Authority

All classifications derive from `layer/core/patina-identity.md` — the
Protocol Test (section "The Protocol Test") and the extraction table
(section "Protocol Tooling"). [[graceful-extraction]] governs the
migration pattern: plugin-first dispatch with compiled fallback.

## Method

For each module: read all source files, count `use crate::` / `use patina::`
imports, classify each dependency as public API (extractable surface) or
internal coupling (extraction blocker), assess format stability, and map
to the appropriate plugin world.

---

## 1. Coupling Analysis

### Legend

| Score | Meaning |
|-------|---------|
| LOW | Zero or near-zero internal crate imports. Self-contained. |
| MEDIUM | Uses only public APIs (`patina::release`, `patina::spec`). Extractable with host functions. |
| HIGH | Reaches into `crate::` internals (retrieval engine, git module, other commands). Needs WIT additions or refactoring before extraction. |

### 1.1 yolo — Devcontainer generation

**Coupling: LOW (0 internal imports)**

| File | Internal imports |
|------|-----------------|
| `src/commands/yolo/mod.rs` | None |
| `src/commands/yolo/features.rs` | None |
| `src/commands/yolo/generator.rs` | None |
| `src/commands/yolo/profile.rs` | None |
| `src/commands/yolo/scanner.rs` | None |

Yolo is entirely self-contained. It scans the current directory for
language/tool markers, maps to devcontainer features, and writes
`.devcontainer/` files. No database, no eventlog, no protocol access.

**Target world:** Task (mutates filesystem — writes `.devcontainer/`)
**Format stability:** STABLE (devcontainer.json is an external spec)
**Extraction blockers:** None. Ready today.

### 1.2 upgrade — Version check

**Coupling: LOW (0 internal imports)**

| File | Internal imports |
|------|-----------------|
| `src/commands/upgrade.rs` | None (`env!("CARGO_PKG_VERSION")` only) |

Self-contained GitHub API check. Uses `reqwest` for HTTP and `chrono`
for date formatting. No database, no eventlog, no layer access.

**Target world:** Command (check-only) or Task (if future download/install)
**Format stability:** STABLE (GitHub releases API + semver comparison)
**Extraction blockers:** Needs `host/http` — available in task world,
NOT in command world. If upgrade stays read-only (check + print), command
world works via `env!()` version. If it downloads/installs, needs task world.

### 1.3 version — Version display + tracking

**Coupling: MEDIUM (2 public API imports)**

| File | Internal imports |
|------|-----------------|
| `src/commands/version/mod.rs` | None |
| `src/commands/version/internal.rs` | `patina::release::{BumpType, ReleaseStrategy}` |

Uses `ReleaseStrategy::from_project()` (public API) and direct
`rusqlite::Connection` to query ready specs from `.patina/local/data/patina.db`.
Also shells out to `git` and `dagger` for component versions.

**Target world:** Command (read-only display)
**Format stability:** STABLE (Cargo.toml version field, semver, JSON output)
**Extraction blockers:**
- DB access for ready specs → must go through `host/query` (available)
- `ReleaseStrategy::from_project()` → needs host function or re-implementation
- External tool version checks (git, dagger) → needs `host/exec` or delegate to host

### 1.4 report — Project state reports

**Coupling: MEDIUM-HIGH (3 internal imports)**

| File | Internal imports |
|------|-----------------|
| `src/commands/report/mod.rs` | None |
| `src/commands/report/internal.rs` | `crate::commands::scry::{scry, ScryOptions}`, `crate::commands::repo::get_db_path` |

Composes from scry queries + direct DB queries + filesystem checks.
The scry dependency can be replaced by `host/query` (kind="scry").
The direct DB queries (summary metrics, largest modules, RAG health)
are the hard part — they access raw SQLite tables not exposed via query.

**Target world:** Command (read-only, composes information)
**Format stability:** MOSTLY STABLE (markdown/JSON output). But depends
on scry result format and DB schema, both of which could evolve.
**Extraction blockers:**
- Scry → replaceable via `host/query` (kind="scry")
- Raw DB queries (index_state, function_facts, scrape_meta, eventlog counts) →
  need new `host/query` kind or host/metrics interface
- `crate::commands::repo::get_db_path` → need host function for repo DB paths

### 1.5 spec — Spec lifecycle + release delegation

**Coupling: MEDIUM (2 public API imports + DB + git)**

| File | Internal imports |
|------|-----------------|
| `src/commands/spec/mod.rs` | None |
| `src/commands/spec/internal.rs` | `patina::release::{BumpType, ReleaseStrategy}`, `patina::spec::{parse_spec_file, serialize_spec_file}` |

Uses only public APIs from the `patina` lib crate. But has deep operational
coupling: direct `rusqlite::Connection` for spec queries, direct
`std::process::Command` for git tag/rm/commit, and delegates to `release`
for version management.

**Target world:** Command (reads/writes stable markdown format)
**Format stability:** STABILIZING (YAML frontmatter with serde roundtrip,
just got spec-complete-archives feature in v0.21.1)
**Extraction blockers:**
- Git operations (tag, rm, commit, status) → needs `host/git` WIT interface
- DB access for spec queries → needs `host/query` (kind="spec") or
  expose spec table through assay
- Release delegation → release must be extractable or accessible as host function
- Filesystem write (updating spec frontmatter) → needs `host/layer` write support

### 1.6 release — Version strategy dispatch

**Coupling: HIGH (2 internal crate imports, 7+ git functions)**

| File | Internal imports |
|------|-----------------|
| `src/release/mod.rs` | None (pure types + interface) |
| `src/release/internal.rs` | `crate::git` (7 functions), `crate::project::is_versioning_enabled` |

Deep coupling to `crate::git` module — uses `status_porcelain`,
`has_upstream`, `commits_behind_upstream`, `is_diverged`, `tag_exists`,
`current_branch`, `create_tag`. Also reads/writes Cargo.toml directly.

**Target world:** Command (but needs git + filesystem write — problematic)
**Format stability:** STABLE (Cargo.toml is standard, git tags are standard)
**Extraction blockers:**
- `crate::git` — 7 functions, deeply intertwined. Needs comprehensive
  `host/git` WIT interface exposing all these operations.
- `crate::project::is_versioning_enabled` — needs host function
- Cargo.toml write — needs filesystem write capability
- This is the module that blocks spec extraction (spec delegates to release)

### 1.7 eval + bench — Retrieval quality measurement

**Coupling: HIGH (5+ internal imports across multiple subsystems)**

| File | Internal imports |
|------|-----------------|
| `src/commands/eval/mod.rs` | `crate::retrieval::{FusedResult, QueryEngine, RetrievalConfig}`, `patina::eventlog` |
| `src/commands/eval/internal/assay_eval.rs` | `crate::commands::assay::{assay_search, SearchOptions}` |
| `src/commands/eval/internal/scry_eval.rs` | `crate::commands::oxidize`, `patina::embeddings::create_embedder`, `crate::commands::assay` |
| `src/commands/eval/internal/combined_eval.rs` | `crate::retrieval::QueryEngine`, `crate::commands::assay` |
| `src/commands/bench/internal.rs` | `crate::retrieval::{QueryEngine, QueryOptions, RetrievalConfig}`, `patina::project` |

Deepest coupling of any extractable module. Creates `QueryEngine` instances
with custom `RetrievalConfig`, directly accesses `FusedResult` internals,
creates embedders, and runs oxidize. These are protocol core internals —
eval literally tests the protocol engine.

**Target world:** Command (power user tooling)
**Format stability:** UNSTABLE (retrieval config, query engine internals,
embeddings API all actively evolving)
**Extraction blockers:**
- `QueryEngine` + `RetrievalConfig` — would need to expose full retrieval
  pipeline configuration through WIT, or accept that eval can only test
  what `host/query` exposes (losing ablation testing)
- `create_embedder` — directly creates ONNX embedding models
- `crate::commands::oxidize` — invokes the indexing pipeline
- Power of eval comes from reaching into internals. Extracting it strips
  its primary value proposition.

---

## 2. Modules NOT in the Extraction Table

Applied the Protocol Test from `patina-identity.md` to every command module
not listed in the extraction table.

### 2.1 session — Development session tracking

**Protocol Test result:** Protocol tooling (extractable)

1. Is it a protocol operation? Borderline — sessions feed into capture
   (observations → eventlog) and evolve (distillation → beliefs).
2. Does it use the protocol? YES — reads/writes layer data, creates
   git tags, writes events.
3. Can Patina function without it? YES — `patina scrape && patina scry`
   works without sessions.

**Current coupling:** LOW-MEDIUM (`patina::git` only)
**Note:** The identity doc lists session in the "Evolution Path" block
(`Protocol tooling: spec, session, release, eval, report, yolo`) but
omits it from the extraction table. This appears to be an oversight —
session should be added to the table.

**Target world:** Command or Task (creates git tags, writes markdown)
**Format stability:** STABLE (markdown sessions, git tags)
**Extraction blockers:** Git operations (tag create), filesystem writes

### 2.2 launch — Open project in AI adapter

**Protocol Test result:** Protocol infrastructure (stays)

3. Does it provide infrastructure? YES — it's the entry point that
   starts Mother and opens AI adapters. Deeply coupled to adapters,
   paths, project, workspace, and git modules (5 internal imports).

→ Stays in the binary. Not extractable.

### 2.3 persona — Cross-project user knowledge

**Protocol Test result:** Protocol core

1. Is it a protocol operation? YES — capture + believe + evolve at
   the user level. This IS the cross-project knowledge system.

→ Stays in the binary. Core protocol.

### 2.4 dev — Developer utilities

**Protocol Test result:** None of the above

4. It's internal developer tooling for maintaining Patina itself
   (bump_version, release, sync_adapters, update_fixtures, validate).
   Not user-facing.

→ Not a plugin candidate. Developer tooling, stays as-is.

---

## 3. Extraction Order

Per [[graceful-extraction]]: formats must stabilize before extraction.
Per the identity doc: "bundle now, extract later" — the boundary moves
outward over time, tooling first, infrastructure last.

### Tier 1: Ready Now (zero internal coupling)

| # | Module | World | Blockers | Effort |
|---|--------|-------|----------|--------|
| 1 | **yolo** | task | None. Filesystem write via WASI. | Small — straightforward port of scanner/generator to guest API. |
| 2 | **upgrade** | command (check) / task (install) | Needs HTTP for GitHub API check. Task world has `host/http`. | Small — mostly reqwest calls that map to `host/http`. |

### Tier 2: Needs `host/query` Exposure (medium coupling)

| # | Module | World | Blockers | Effort |
|---|--------|-------|----------|--------|
| 3 | **version** | command | DB access → convert to `host/query`. `ReleaseStrategy` → re-implement or host function. | Medium — refactor DB queries to use host/query. |
| 4 | **report** | command | Raw DB queries → need new query kinds or `host/metrics` interface. | Medium — most work is defining new query surface. |

### Tier 3: Needs `host/git` WIT Interface

| # | Module | World | Blockers | Effort |
|---|--------|-------|----------|--------|
| 5 | **session** | command/task | Git tag creation, markdown file writes. | Medium — needs `host/git` + filesystem write. |
| 6 | **spec** | command | Git operations (tag/rm/commit), release delegation, DB queries. | Large — blocked by release coupling. Extract release first or decouple. |
| 7 | **release** | command | 7 `crate::git` functions, Cargo.toml write. Most tightly coupled extractable module. | Large — requires comprehensive `host/git` interface. |

### Tier 4: Likely Stays Compiled

| # | Module | World | Blockers | Effort |
|---|--------|-------|----------|--------|
| 8 | **eval + bench** | command | Deep into retrieval engine internals. Extracting loses ablation power. | Very large — would need to expose full retrieval pipeline through WIT. Diminishing returns. |

---

## 4. What Blocks Extraction?

### 4.1 Missing WIT Interfaces

| Interface | Needed by | What it does |
|-----------|-----------|-------------|
| `host/git` | spec, release, session | Tag create/exists, status porcelain, commit, rm, branch info |
| `host/filesystem` (write) | yolo, spec | Write files outside the WASI sandbox (or map work dirs) |
| `host/metrics` | report, version | Raw DB statistics (file counts, event counts, function counts) |
| `host/exec` | release | Run external commands (cargo, git) |

**Note:** `host/query` (scry, context, assay) already exists and covers
report's scry dependency and version's ready-spec queries.

### 4.2 Format Instability

| Module | Issue |
|--------|-------|
| eval + bench | `QueryEngine`, `RetrievalConfig`, `FusedResult` internals actively evolving |
| report | Depends on scry result format and DB schema |

### 4.3 Architectural Coupling

| Pattern | Modules | Issue |
|---------|---------|-------|
| spec → release → git | spec, release | Spec delegates to release for version bumps. Release uses 7 git functions. Extracting spec requires extracting or decoupling release first. |
| eval → retrieval engine | eval, bench | Eval's value is testing internals. Extraction would limit it to black-box testing via `host/query`. |

---

## 5. Where Does plugin-template-gallery Fit?

[[plugin-template-gallery]] (`patina plugin init`) is NOT an extraction
of an existing module. It's **extraction infrastructure** — the tool that
makes all future extractions faster.

### Role in the Extraction Story

```
Today:     Creating a plugin = 5 manual steps, reconstruct from docs
After PTG: Creating a plugin = `patina plugin init review-bot --world task`
```

Every module in Tiers 1-3 above will eventually become a plugin project.
Each needs a `Cargo.toml` (cdylib), `plugin.toml` (manifest), and
`src/lib.rs` (trait impl). plugin-template-gallery automates this
boilerplate.

### Priority Assessment

plugin-template-gallery should ship **before** Tier 1 extractions
(yolo, upgrade), because:

1. **It validates the guest API crates** — templates depend on
   `patina-task-api`, `patina-command-api`, etc. Scaffolding tests that
   these crates work correctly for all 4 worlds.
2. **It documents the extraction pattern** — templates ARE the reference
   implementation for how to build a plugin in each world.
3. **It reduces extraction effort** — instead of hand-constructing
   `Cargo.toml` + `plugin.toml` + `lib.rs` for each extraction, you
   scaffold and focus on porting logic.
4. **It's a small, well-scoped spec** — 4-5 commits, no runtime changes.
   Pure scaffolding with embedded templates.

### Recommended Sequence

```
1. plugin-template-gallery  → Ship `patina plugin init`
2. yolo extraction          → First Tier 1 extraction, uses task world
3. upgrade extraction       → Second Tier 1 extraction, validates command/task
4. host/query improvements  → Enable Tier 2 (version, report)
5. host/git WIT interface   → Enable Tier 3 (session, spec, release)
```

---

## 6. Updated Extraction Table

The identity document's extraction table should be updated to include
session and note eval+bench's likely permanent residence:

| Module | What it does | Extraction path | Coupling | Priority |
|--------|-------------|-----------------|----------|----------|
| `yolo` | Devcontainer generation | Task plugin (zero coupling) | LOW | Tier 1 |
| `upgrade` | Version check | Command/Task plugin (zero coupling) | LOW | Tier 1 |
| `version` | Version display | Command plugin (needs host/query) | MEDIUM | Tier 2 |
| `report` | Project reports | Command plugin (needs host/query + metrics) | MEDIUM-HIGH | Tier 2 |
| `session` | Session tracking | Command/Task plugin (needs host/git) | LOW-MEDIUM | Tier 3 |
| `spec` | Spec lifecycle | Command plugin (blocked by release) | MEDIUM | Tier 3 |
| `release` | Version strategy | Command plugin (needs host/git) | HIGH | Tier 3 |
| `doctor` | Health checks | **Extracted (v0.17.0)** | — | Done |
| `eval` + `bench` | Retrieval quality | **Likely stays compiled** — value is in internal access | HIGH | Tier 4 |

---

## 7. Open Questions

1. **Should eval+bench stay compiled permanently?** Its value is ablation
   testing of retrieval internals. Extracting it limits testing to the
   `host/query` surface. If we accept that tradeoff, it becomes Tier 2.
   If not, it stays in the binary as "protocol quality tooling."

2. **Should session be added to the identity doc's extraction table?**
   The evolution path lists it as protocol tooling, but the table omits
   it. Recommend adding it — session tracking can work entirely through
   host functions once `host/git` exists.

3. **Is `host/git` worth building?** Three Tier 3 modules (spec, release,
   session) need it. That's significant ROI. But it's a large WIT surface
   (7+ functions from `crate::git` alone). Alternative: expose a smaller
   `host/exec` that lets plugins run allowlisted commands.

4. **Should release stay coupled to spec?** Today `spec status complete`
   delegates to `release` for version bumps. If release extracts, spec
   must either extract with it or call through a host function. Decoupling
   them (spec just marks status, release runs separately) might be cleaner.

## Status Log

| Date | Status | Note |
|------|--------|------|
| 2026-02-14 | active | Initial exploration. Read all 7 extractable modules + 4 additional candidates. Produced coupling scores, world mappings, format stability assessments, blocker inventory, and recommended extraction order. |
