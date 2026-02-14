---
type: explore
id: plugin-extraction-map
status: complete
created: 2026-02-14
sessions:
  origin: 20260214-104350
  completed: 20260214-110957
beliefs:
- graceful-extraction
- separate-worlds-for-isolation
- plugin-is-agent-plus-skill
- wasi-sandboxed-filesystem
- two-layer-capability-grants
- duplicate-before-extract
- reads-via-host-writes-via-intents
- decouple-before-extract
- eval-stays-compiled
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

## 8. Deep Dive: The host/git Question

### The Precise Inventory

Every `crate::git` call and raw `Command::new("git")` across the three
Tier 3 modules, classified as read (safe, informational) or write (mutates
repo state).

#### Read Operations (18 unique)

| Operation | session | spec | release | What it returns |
|-----------|:-------:|:----:|:-------:|-----------------|
| `is_git_repo()` | Y | - | - | bool |
| `current_branch()` | Y | - | Y | string |
| `is_clean()` | Y | - | - | bool |
| `branch_exists(name)` | Y | - | - | bool |
| `head_sha()` | Y | - | - | string (full SHA) |
| `short_sha()` | Y | - | - | string (7-char SHA) |
| `tag_exists(name)` | Y | Y | Y | bool |
| `has_upstream()` | - | - | Y | bool |
| `commits_behind_upstream()` | - | - | Y | usize |
| `is_diverged()` | - | - | Y | bool |
| `status_porcelain()` | Y | Y | Y | string (full status) |
| `commits_since_count(sha)` | Y | - | - | usize |
| `last_commit_relative_time()` | Y | - | - | string |
| `last_commit_message()` | Y | - | - | string |
| `diff_stat_summary()` | Y | - | - | string |
| `log_oneline(n)` | Y | - | - | string |
| `files_changed_since(ref)` | Y | - | - | list\<string\> |
| `status_count()` | Y | - | - | usize |

Session dominates reads (16/18). It's an observer — it watches git
state to report session metrics. Release reads are safeguard checks (5/18).
Spec reads are minimal (2/18, just tag_exists + is_tree_clean).

#### Write Operations (7 unique)

| Operation | session | spec | release | What it does |
|-----------|:-------:|:----:|:-------:|-------------|
| `create_tag(name, msg)` | Y | - | Y | Annotated tag on HEAD |
| `create_tag(name, msg, ref)` | - | Y | - | Annotated tag on specific ref |
| `checkout(branch)` | Y | - | - | Switch branch |
| `checkout_new_branch(name, from)` | Y | - | - | Create + switch |
| `git rm -rf <path>` | - | Y | Y | Remove tracked files |
| `git add <paths>` | - | Y | Y | Stage files |
| `git commit -m <msg>` | - | Y | Y | Create commit |

Session writes are boundary markers (2: create_tag) and branch setup
(2: checkout). Spec and release write the same 4 operations (tag, rm,
add, commit) — their write surfaces are nearly identical because spec
delegates to release.

### Five Design Options

#### Option A: Comprehensive host/git WIT Interface

```
interface git {
    // Reads
    current-branch: func() -> result<string, string>;
    tag-exists: func(name: string) -> result<bool, string>;
    head-sha: func() -> result<string, string>;
    status-porcelain: func() -> result<string, string>;
    has-upstream: func() -> result<bool, string>;
    commits-behind-upstream: func() -> result<u32, string>;
    is-diverged: func() -> result<bool, string>;
    commits-since-count: func(sha: string) -> result<u32, string>;
    log-oneline: func(count: u32) -> result<string, string>;
    files-changed-since: func(ref: string) -> result<list<string>, string>;
    // ... 8 more reads

    // Writes
    create-tag: func(name: string, message: string, ref: option<string>)
        -> result<_, string>;
    checkout: func(branch: string) -> result<_, string>;
    checkout-new-branch: func(name: string, from: string)
        -> result<_, string>;
    add-paths: func(paths: list<string>) -> result<_, string>;
    rm-paths: func(paths: list<string>, force: bool) -> result<_, string>;
    commit: func(message: string) -> result<_, string>;
}
```

**Assessment:** 24+ functions. Huge WIT surface. Every new git need = WIT
change + host impl change + re-publish guest API crate. Tight coupling
between plugin capabilities and host git surface. Violates unix-philosophy
("one tool, one job") — this is building a git client inside WIT.

#### Option B: host/exec Allowlisted Shell Execution

```
interface exec {
    run: func(command: string, args: list<string>)
        -> result<exec-result, string>;
    record exec-result {
        exit-code: s32,
        stdout: string,
        stderr: string,
    }
}
```

With manifest-declared allowed commands:
```toml
[capabilities]
host_exec = ["git"]
```

**Assessment:** Small interface, maximum flexibility. But terrible for
security — even "just git" includes `git push --force`, `git reset --hard`,
`git clean -fdx`. The manifest allowlist is too coarse. Would need
sub-command allowlisting (`git.tag`, `git.commit` but not `git.push`)
which recreates Option A's complexity in TOML instead of WIT.

#### Option C: Intent-Based (Return Actions, Host Executes)

Plugins return *what they want to happen*, host decides *how and whether*.
This is the toy pattern already proven in the task world.

```
interface git-intents {
    record git-intent {
        kind: git-intent-kind,
        args: list<string>,
    }
    enum git-intent-kind {
        create-tag,     // args: [name, message] or [name, message, ref]
        add-paths,      // args: [path1, path2, ...]
        rm-paths,       // args: [path1, path2, ...]
        commit,         // args: [message]
        checkout,       // args: [branch]
        create-branch,  // args: [name, from]
    }
}

// Plugin run() returns intents alongside exit code
record run-result {
    exit-code: s32,
    git-intents: list<git-intent>,
}
```

**Assessment:** Clean separation of concerns. Host validates, audits,
and executes. Plugin can't do destructive operations the host doesn't
allow. BUT: plugin can't make decisions mid-execution based on git state.
"If tag exists, bail" requires the plugin to query tag_exists first,
then conditionally emit the create-tag intent. This means **reads must
still be synchronous host calls** while writes are deferred intents.

#### Option D: Don't Extract These Modules

The identity doc says "bundle now, extract later" and "the boundary
moves outward over time." Maybe the boundary shouldn't move here.

**Argument for staying compiled:** Spec, release, and session are not
just "using the protocol" — they are **operating the development
lifecycle**. They create commits. They create tags. They move files in
and out of the git index. This is the version control substrate itself.

The Protocol Test asks "Can Patina function without it?" Yes, technically.
But these modules are the governance layer — the spec-driven development
lifecycle that makes Patina's knowledge layer trustworthy. Extracting
them into sandboxed WASM means the most critical workflow operations
run with the least host access.

**Argument for eventual extraction:** The whole point of plugins is that
the binary gets smaller and the protocol core gets harder. Community
members might want different spec lifecycles, release strategies, or
session workflows. Extraction enables customization.

#### Option E: Split Read/Write (Recommended)

Combine the best of Options A and C:

**Reads: Synchronous host calls** — small, typed, safe.
**Writes: Returned as intents** — validated and executed by host.

```
// host/git-read — synchronous, safe, typed
interface git-read {
    current-branch: func() -> result<string, string>;
    tag-exists: func(name: string) -> result<bool, string>;
    head-sha: func() -> result<string, string>;
    status-porcelain: func() -> result<string, string>;
    has-upstream: func() -> result<bool, string>;
    is-diverged: func() -> result<bool, string>;
    commits-since: func(ref: string) -> result<u32, string>;
    log-oneline: func(count: u32) -> result<string, string>;
    files-changed-since: func(ref: string) -> result<list<string>, string>;
}
```

```
// Writes are returned as intents from run()
record git-action {
    kind: git-action-kind,
    args: list<string>,
}
enum git-action-kind {
    create-tag,     // [name, message] or [name, message, ref]
    add-paths,      // [path1, ...]
    rm-paths,       // [path1, ...]
    commit,         // [message]
    checkout,       // [branch]
    create-branch,  // [name, from]
}
```

**Assessment:**
- Reads are an 8-9 function interface — manageable, typed, testable.
- Writes follow the proven toy pattern — plugin declares intent, host
  executes with full validation. The host can reject, audit, or modify.
- Plugin logic works: "query tag_exists → if exists, return exit code 1;
  if not, return create-tag intent" — decision logic stays in the plugin,
  destructive execution stays in the host.
- Matches [[two-layer-capability-grants]]: manifest declares what the
  plugin wants (git-read + git write kinds), host decides what to allow.
- Matches [[wasi-sandboxed-filesystem]]: reads go through host functions,
  writes are mediated intents.

### Which Modules Fit Which Pattern?

| Module | Reads needed | Writes needed | Option E fit |
|--------|:----------:|:------------:|:------------:|
| session | 16 reads | 2 (tag, branch) | Excellent — mostly observer + boundary markers |
| release | 5 reads (safeguards) | 4 (tag, add, rm, commit) | Good — safeguard reads → write intents |
| spec | 2 reads | 5 (tag, add, rm, commit) | Good — but delegates to release |

### The spec→release Coupling Problem

Even with Option E, there's a structural issue: `spec status complete`
calls `release.preflight()` → `release.execute()`. In a plugin world:

1. **Both extracted:** spec-plugin calls... what? It can't call
   release-plugin. Plugins don't call each other. The host would need
   to orchestrate: spec returns "release-intent", host runs release logic.
2. **Spec extracted, release stays:** spec-plugin returns a "release"
   intent, host dispatches to compiled release code. Clean.
3. **Both stay compiled:** Status quo. Works fine.
4. **Decouple them:** `patina spec status <id> complete` just marks status.
   `patina release` is a separate command that reads completions and
   bumps versions. Cleanest separation but changes the user workflow.

**Recommendation: Option 4 (decouple) before extraction.**

Today's coupling is a convenience, not a necessity. `spec status complete`
doing version bump + git tag + archive is an all-in-one workflow.
Decomposing it into `spec status complete` (mark status) +
`patina release` (detect completions, bump, tag) + `patina spec archive`
(which already exists) follows unix-philosophy: each command does one job.
This decoupling is worth doing regardless of plugin extraction.

### Timeline Assessment

```
                   Now        After PTG    After Tier 1    Long-term
                   ────        ─────────    ──────────      ─────────
host/git-read:     n/a         n/a          Design          Ship
git-action intents: n/a         n/a          Design          Ship
spec→release:      coupled     coupled      decouple spec   extract both
session:           compiled    compiled     compiled        extract w/ git-read
```

The host/git work is NOT on the critical path. The sequence is:
1. Ship PTG (plugin-template-gallery)
2. Extract yolo + upgrade (Tier 1, no git needed)
3. Decouple spec from release (refactor, no plugin work)
4. Design host/git-read WIT + git-action intents
5. Extract session (heaviest git-read user, lightest writer)
6. Extract spec, then release

Steps 1-3 happen first. Steps 4-6 are long-term.

## 9. WIT Convention Audit: Option E vs Existing Interfaces

Vetted the proposed `host/git-read` interface and `git-action` intents
against the three existing WIT files.

### Naming Conventions

Existing host interfaces use kebab-case: `log-level`, `child-health`,
`health-status`, `http-response`, `http-post`, `http-get`, `find-project-root`,
`count-layer-files`, `check-adapter-version`. Option E's proposed names
(`current-branch`, `tag-exists`, `head-sha`, `status-porcelain`,
`has-upstream`, `is-diverged`, `commits-since`, `log-oneline`,
`files-changed-since`) follow this convention exactly. No corrections needed.

### Interface Placement

Existing pattern: one interface per concern, all in `patina:host@0.1.0`:
- `log` — logging
- `types` — shared records/enums outside any world
- `layer` — read-only project data
- `query` — capability-gated search
- `http` — domain-allowlisted network

A `git-read` interface fits naturally as another `patina:host` interface.
It would live in `host.wit` alongside the others. The name `git-read`
follows the existing pattern of scoping by capability (read-only, safe).

### Where git-actions Belong

Existing intent pattern: `toy` record in `patina:host/types@0.1.0`.
Toys are returned from `tick()` (mother-child) and `toys()` (task).
A `git-action` record should follow the same pattern:

- **Define** `git-action` record + `git-action-kind` enum in
  `patina:host/types@0.1.0` (alongside `toy`, `child-health`)
- **Return** from a new export: either extend `run()` to return a
  composite result, or add a `git-actions: func() -> list<git-action>`
  export parallel to `toys()`

The second approach (dedicated export) is cleaner — it preserves
`run()` returning a simple `s32` and follows the proven `toys()` pattern.
Host calls `run()`, then calls `git-actions()`, validates against
manifest capabilities, and executes.

### World Import/Export Changes

| World | Current imports | Would add |
|-------|---------------|-----------|
| command | log, layer, query | + `git-read` import (for session, spec, release reads) |
| task | log, types, layer, query, http | + `git-read` import |
| command | exports: init, name, description, run | + `git-actions` export |
| task | exports: init, name, description, run, toys | + `git-actions` export |

**Key observation:** Command world currently has NO action mechanism.
It returns only `s32` from `run()`. Adding `git-actions()` to command
world would be the first command-world intent pattern. This is
architecturally significant — it means command plugins can have
side-effects mediated by the host. This is the right design (host
validates + executes), but it changes command world's semantic from
"inform only" to "inform + mediated action."

**Alternative:** Move git-writing modules (spec, release) to task world
instead. Task world already has the `toys()` intent pattern. Git-actions
would compose naturally alongside toys. This preserves command world as
read-only.

**Recommendation:** Use task world for all Tier 3 extractions. Session,
spec, and release all write (git tags, commits, file removals). The
task world's existing `toys()` pattern extends cleanly to
`git-actions()`. Keep command world pure read-only per its docstring
("one-shot execution"). This aligns with [[separate-worlds-for-isolation]].

---

## 10. spec→release Decoupling: Code-Level Validation

### Current Coupling (verified against source)

`src/commands/spec/internal.rs` imports:
```
use patina::release::{BumpType, ReleaseStrategy};
use patina::spec::{parse_spec_file, serialize_spec_file};
```

The coupling is concentrated in `update_spec_status()` (line 361).
When `new_status == "complete"`:

1. `ReleaseStrategy::from_project(Path::new("."))` — detects Cargo/External/None
2. `BumpType::from_spec_type(&frontmatter.r#type)` — feat→Minor, fix→Patch
3. `strategy.preflight(bump, &file_path)` — safeguard checks (git status, tags, etc.)
4. `prepared.execute(title, &file_path, archive_dir)` — bump Cargo.toml, git add, git commit, git tag

`release/internal.rs` in turn uses:
- `crate::git` (5 functions in safeguard checks): `status_porcelain`,
  `has_upstream`, `commits_behind_upstream`, `is_diverged`, `tag_exists`,
  `current_branch`
- `crate::git::create_tag` (1 function in execute)
- `crate::project::is_versioning_enabled` (1 function in detection)
- Direct `Command::new("git")` for: `git rm -rf`, `git add`, `git commit`

### Additional Finding: spec's Private Git Duplicates

`spec/internal.rs` has its own private `tag_exists()` (line 731) and
`is_tree_clean()` (line 741) that duplicate `patina::git::tag_exists`
and `patina::git::is_clean`. These were presumably written before the
shared git module existed or to avoid an import. A small tech debt item
independent of extraction.

### What Changes if spec Stops Invoking release

Today's flow (`patina spec status <id> complete`):
```
spec::update_spec_status()
  → validate status
  → update spec file + DB
  → ReleaseStrategy::from_project()
  → BumpType::from_spec_type()
  → strategy.preflight()      ← safeguard checks
  → prepared.execute()         ← Cargo.toml bump + commit + tag
  → archive_spec_inner()       ← git rm + commit + spec tag
```

Decoupled flow (`patina spec status` + `patina release`):
```
patina spec status <id> complete
  → validate status
  → update spec file + DB
  → [if should_archive] archive_spec_inner()
  → print: "Spec marked complete. Run 'patina release' to bump version."

patina release
  → find completed, unreleased specs
  → ReleaseStrategy::from_project()
  → determine aggregate bump (highest of all completed spec types)
  → preflight() → execute()
```

### Refactoring Scope

| Change | Files | Effort |
|--------|-------|--------|
| Remove `patina::release` imports from spec/internal.rs | 1 file | Small |
| Remove the `if new_status == "complete"` release delegation block (lines 428-457) | 1 file | Small |
| Add `patina release` subcommand (scan for completed unreleased specs) | New command or extend release/mod.rs | Medium |
| Consolidate spec's private `tag_exists`/`is_tree_clean` to use `patina::git` | 1 file, cleanup | Small |
| Archive flow stays in spec (it's spec lifecycle, not release) | No change | — |

Total: ~1 session of work. The release command needs a "scan for completed
specs that haven't been released" query, which is a new `patina release`
subcommand. This is a natural fit — release owns versioning strategy,
spec owns spec lifecycle.

### Verdict

This decoupling is **worth doing as a standalone refactor spec**, independent
of plugin extraction. Reasons:

1. **unix-philosophy compliance** — spec does one job (spec lifecycle),
   release does one job (version management)
2. **Unblocks both modules for independent extraction** — spec becomes
   Tier 2 (needs host/query + host/git for archiving, not host/release),
   release becomes independently extractable
3. **Better user model** — explicit `patina release` is clearer than the
   hidden side-effect of `spec status complete` bumping versions
4. **Small, bounded change** — mostly deleting code from spec/internal.rs
   and adding a query to the release command

---

## 11. Coupling Analysis Corrections

Re-read all extractable modules against current source (commit ff7d270).
Changes from the initial analysis:

### 11.1 spec Coupling — Corrected

Initial claim: "2 public API imports + DB + git"
Verified: **2 public API imports + 1 DB import + 5 raw git commands**

- `patina::release::{BumpType, ReleaseStrategy}` — public API, extraction blocker
- `patina::spec::{parse_spec_file, serialize_spec_file}` — lib crate public API, NOT a blocker (this is spec's own types re-exported through the lib)
- `rusqlite::Connection` — direct DB for spec queries and status updates
- 5 raw `Command::new("git")` calls: `git tag -a`, `git rm -rf`, `git add`, `git commit -m`, plus 2 private helpers (`tag_exists`, `is_tree_clean`)

The coupling is **slightly higher** than reported because of the raw git
commands (counted as "direct git" but not broken out as individual
`Command::new` calls). The private `tag_exists`/`is_tree_clean` duplicates
add 2 more git operations not counted in the Section 8 inventory.

**Updated git operation count for spec:**
- Reads: `tag_exists` (×3 call sites), `is_tree_clean` (×1) → 2 unique reads
- Writes: `git tag -a -m ref` (×1), `git rm -rf` (×2 paths), `git add` (×0, delegated to release), `git commit -m` (×1) → 3 unique writes
- Plus all git operations delegated through `release` (7 via `crate::git`)

### 11.2 session Coupling — Confirmed Accurate

Initial claim: "LOW-MEDIUM (`patina::git` only)"
Verified: **Exactly 1 import: `use patina::git;`**

Session uses 16 git read functions and 2 write functions through the
public `patina::git` module. No raw `Command::new("git")` calls. No DB
access. No imports from other command modules. This is the cleanest
Tier 3 module — a pure observer with boundary markers.

### 11.3 release Coupling — Confirmed Accurate

Initial claim: "HIGH (2 internal crate imports, 7+ git functions)"
Verified: `crate::git` (6 functions in safeguards + 1 `create_tag` in execute),
`crate::project::is_versioning_enabled` (1), plus 3 raw
`Command::new("git")` for `rm`, `add`, `commit`.

Total: 7 `crate::git` + 1 `crate::project` + 3 raw git = 11 git-touching
operations. HIGH coupling confirmed.

### 11.4 No Other Coupling Changes

yolo (0 imports), upgrade (0 imports), version (2 public API imports),
report (3 internal imports), eval+bench (5+ internal imports) — all
unchanged since initial analysis. No recent commits touched these modules.

---

## 12. Belief Cross-Check

### Existing Beliefs — Validation

| Belief | Relevant? | Validated? |
|--------|:---------:|:----------:|
| [[graceful-extraction]] | YES | Confirmed — doctor extraction proves the pattern. Feature-gated fallback works. |
| [[separate-worlds-for-isolation]] | YES | Confirmed — and strengthened by the finding that Tier 3 modules should use task world (not command) because they write. |
| [[two-layer-capability-grants]] | YES | Confirmed — manifest declares git-read + git-action-kinds, host validates. |
| [[plugin-is-agent-plus-skill]] | PARTIAL | Applies to ecosystem plugins more than extraction of existing commands. Extracted commands ARE the agent; skill templates are optional. |
| [[wasi-sandboxed-filesystem]] | YES | Confirmed — extracted modules can't access git directly, must go through host functions. |
| [[duplicate-before-extract]] | YES | Relevant counter-guidance: spec's private `tag_exists`/`is_tree_clean` are duplicates, but since there are now 3 consumers (spec, release, session all use tag_exists), extraction to shared `patina::git` is justified by the rule of three. |

### New Beliefs to Capture

Three beliefs emerged from this exploration:

**1. reads-via-host-writes-via-intents**
Plugin reads are synchronous host calls (safe, typed, immediate return).
Plugin writes are returned as intents (host validates, audits, executes).
This split keeps decision logic in the plugin while keeping destructive
execution in the host. Proven by the toy pattern (mother-child + task
worlds), now recommended for git operations.

**2. decouple-before-extract**
Coupled modules must be decoupled before independent extraction.
spec→release coupling means extracting one requires extracting or
hosting the other. Decoupling first (each command does one job) makes
both independently extractable and follows unix-philosophy. General
principle: tightly coupled modules that want different extraction
timelines must be decoupled first.

**3. eval-stays-compiled**
eval+bench should stay compiled because their value is in ablation
testing of retrieval internals (QueryEngine, RetrievalConfig,
FusedResult, create_embedder). Extracting them limits testing to the
host/query surface, losing the ability to test retrieval component
combinations. This is "protocol quality tooling" — it tests the
protocol engine, not just uses it.

---

## 13. Identity Document Updates

### Recommended Changes to `layer/core/patina-identity.md`

**1. Add `session` to the extraction table** (section "Protocol Tooling")

Session is listed in the Evolution Path block as protocol tooling
(`spec, session, release, eval, report, yolo`) but omitted from the
extraction table. The Protocol Test confirms it's extractable:
- Uses the protocol (writes layer data, creates git tags, writes events)
- Patina functions without it (`patina scrape && patina scry` works)
- LOW-MEDIUM coupling (only `patina::git`)

Add row:
```
| `session` | Development session tracking | Task plugin (needs host/git for tags) |
```

**2. Annotate eval+bench as "likely stays compiled"**

Current extraction table says: `Command plugin (power user tooling)`.
Should be updated to reflect the analysis:
```
| `eval` + `bench` | Retrieval quality measurement | Likely stays compiled — value is ablation testing of retrieval internals |
```

**3. Do NOT reference host/git-read + git-action pattern**

The identity document is about *what Patina is*, not *how plugins work*.
The host/git design is implementation detail that belongs in the
plugin-extraction-map explore spec (here) or a future host/git-read
WIT spec. Adding implementation details to the identity doc would
violate its purpose.

**4. Add `upgrade` to the Evolution Path block**

The Evolution Path lists `spec, session, release, eval, report, yolo`
as protocol tooling. `upgrade` is missing. It's in the extraction table
but not the narrative block. Add it for consistency.

---

## 14. Closure Assessment

### Is the Exploration Complete?

**YES — the exploration can close.** All questions from the initial
exploration (Sections 1-8) have been validated, corrected, and deepened.
The remaining work is action items, not analysis.

### What This Explore Produced

1. Complete coupling analysis of 11 modules (7 from identity table + 4 candidates)
2. Four-tier extraction order with verified coupling scores
3. Host/git WIT design (Option E) vetted against existing WIT conventions
4. spec→release decoupling plan with line-level code references
5. Three new beliefs: reads-via-host-writes-via-intents, decouple-before-extract, eval-stays-compiled
6. Identity doc update recommendations (4 items)
7. Clear WIT convention finding: Tier 3 modules should use task world (not command) due to write operations

### Child Specs: What's Warranted and What's Premature

| Potential Child Spec | Warranted? | Reason |
|---------------------|:----------:|--------|
| [[spec-release-decouple]] (refactor) | **YES** | Well-scoped, bounded, code-level plan exists (Section 10). Worth doing regardless of extraction timeline. Follows unix-philosophy. |
| [[host-git-read]] (WIT design) | **NOT YET** | Depends on Tier 1 extractions + PTG shipping first. Design is documented here (Sections 8-9) and can be promoted to a spec when the prerequisite work completes. Writing a spec now would be speculative — the WIT may need to evolve as Tier 1/2 extractions teach us lessons. |
| [[identity-doc-update]] | **NO** — inline | The 4 identity doc changes are small (add 2 table rows, update 1 row text, add 1 word to a list). These should be done as a direct commit, not a spec. Specs authorize non-trivial changes. |

### Recommended Next Actions (not specs, just work items)

1. **Capture 3 beliefs** (reads-via-host-writes-via-intents, decouple-before-extract, eval-stays-compiled)
2. **Update identity doc** (4 small changes per Section 13)
3. **Write [[spec-release-decouple]] refactor spec** when ready to schedule the work
4. **Clean up spec's git duplicates** (consolidate private tag_exists/is_tree_clean → patina::git) — small tech debt, can piggyback on any spec/ change

---

## Status Log

| Date | Status | Note |
|------|--------|------|
| 2026-02-14 | active | Initial exploration. Read all 7 extractable modules + 4 additional candidates. Produced coupling scores, world mappings, format stability assessments, blocker inventory, and recommended extraction order. |
| 2026-02-14 | active | Deep dive into host/git question. Inventoried all 25 git operations across Tier 3 modules (18 reads, 7 writes). Evaluated 5 design options. Recommended Option E (split read/write: synchronous reads + intent-based writes). Identified spec→release decoupling as prerequisite for extraction. |
| 2026-02-14 | complete | Validated all analysis against current source. Vetted Option E against WIT conventions (fits cleanly). Documented spec→release decoupling at code level. Found spec has private git duplicates (tech debt). Determined Tier 3 modules should use task world (not command) due to writes. Captured 3 new beliefs. Identified 1 warranted child spec ([[spec-release-decouple]]) and assessed 1 as premature ([[host-git-read]]). Exploration complete.

| Date | Status | Note |
|------|--------|------|
| 2026-02-14 | active | Initial exploration. Read all 7 extractable modules + 4 additional candidates. Produced coupling scores, world mappings, format stability assessments, blocker inventory, and recommended extraction order. |
| 2026-02-14 | active | Deep dive into host/git question. Inventoried all 25 git operations across Tier 3 modules (18 reads, 7 writes). Evaluated 5 design options. Recommended Option E (split read/write: synchronous reads + intent-based writes). Identified spec→release decoupling as prerequisite for extraction. |
