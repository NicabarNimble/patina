# Design: Diff-Driven Scrape Core with AOT Plugin Cache

## KSA Alignment

This spec is foundational for [[knowledge-system-architecture]]. KSA EC1
(`scrape-is-plugin-dispatched`) requires the dispatch interface built here.
Every design decision below is made with KSA extension in mind.

### Alignment Decisions (A1–A5)

**A1: Dispatch Interface — Two Levels**

Dispatch operates at two levels:

| Level | What it routes | Key today | KSA extension |
|-------|---------------|-----------|---------------|
| **Source-kind** | Which scraper to invoke | Hardcoded in `execute_all()` | Pluggable source kinds |
| **File-type** | Which grammar handles a file | `PluginProvides.languages` | Already extensible |

SDD Phase 1 builds both levels:
- Source-kind dispatch: compute the delta, classify changed files into source
  kinds (code, layer, git-metadata, beliefs), skip source kinds with zero
  changes. Today this is `execute_all()` calling 4 functions. Phase 1 makes
  the "skip if no changes" decision explicit — the delta drives it.
- File-type dispatch: within the code scraper, load only plugins for affected
  extensions. Already works via `discover_pipeline_plugins()` → extension map.
  Phase 1 adds lazy loading (don't load plugins until the delta says we need
  them).

KSA EC1 extends source-kind dispatch: forge, spec, sessions become plugins
that register as source kinds. The dispatch interface (delta → classify →
route) stays the same. New source kinds get new classifiers.

Evidence: `scrape/mod.rs:61-84` (hardcoded 4-scraper loop),
`extract_v2.rs:442-488` (extension → plugin map).

**A2: Fact Emission — Plugins Compute, Host Writes**

Pipeline plugins are pure compute: they receive a JSON request via
`handle(request: string)` and return a JSON response. The host
deserializes and writes to DB. This is correct per [[dependable-rust]] —
the plugin boundary is an interface, not a storage layer.

SDD preserves this pattern. Grammar plugins return parsed data as JSON.
The host writes to `patina.db`. No plugin has direct DB access.

KSA EC3 (`plugins-can-emit-facts`) will add a `host_emit(event_type, data)`
function to the WIT host interface. This is additive — existing pipeline
plugins don't need it. Future source-kind plugins (forge, spec, sessions)
will call `host_emit` to write facts to `events.db`. The host validates
the schema, writes the event, and the scrape pipeline continues.

SDD does NOT add `host_emit` — that's KSA Phase 1 work. SDD's dispatch
interface is designed so `host_emit` slots in without restructuring.

Evidence: `pipeline.wit:10` (log-only import), `pipeline.rs:83-109`
(JSON in, JSON out), `extract_v2.rs:511-538` (host writes to DB).

**A3: Schema Ownership — Not SDD's Concern**

Plugin manifests already declare schema dependencies via the `schemas` field
(`PluginManifest.schemas: HashMap<String, String>`). Grammar plugins have no
schemas — they return parsed code structures, not domain facts. The schema
system is relevant to KSA's source-kind plugins (forge ships `forge.wit` +
`schema.toml`), not to SDD's grammar plugins.

SDD does not modify the schema system. KSA Phase 1 adds auto-install:
when a source-kind plugin is loaded, its schemas are installed automatically.

Evidence: `mod.rs:147` (schemas field exists), `.patina/schemas/forge/`
(manually installed today), grammar plugins have empty schemas map.

**A4: Plugin Worlds — Pipeline for Local, Mother-Child for Remote**

Two kinds of scrapers exist:

| Kind | Data source | World | Capabilities needed |
|------|------------|-------|-------------------|
| **Local** | Files on disk | Pipeline | log only (host reads files) |
| **Remote** | APIs, services | Mother-child | log, HTTP, query, measure |

SDD Phases 1-2 use pipeline world (grammar plugins parse local files).
SDD Phase 3 moves grammar plugins to Mother as warm instances — they remain
pipeline-world plugins, Mother just hosts them pre-loaded.

KSA's forge extraction needs mother-child world (HTTP for GitHub API).
No new WIT world is needed. The existing 4 worlds cover all extraction
patterns:
- Pipeline: parse local file content (grammars)
- Mother-child: fetch + parse remote data (forge, future connectors)
- Command: user-facing analysis (doctor)
- Task: one-shot actions with side effects

Evidence: `mod.rs:55-70` (world capabilities), `pipeline.wit` (log only),
`mother-child.wit` (includes HTTP), `extract_v2.rs:519` (host passes
file content to pipeline plugin).

**A5: Domain Agnosticism — Git Is Infrastructure**

Per [[if-its-patina-its-git]]: "Git is the source of record for what a
project declares." Git is infrastructure like SQLite — not domain knowledge
like Rust syntax or GitHub API semantics.

SDD's delta computation is git-based: `git diff` or mtime walk of the
working tree. This stays in core. It is domain-agnostic in the sense that
it works identically for Rust projects, email archives, Obsidian vaults,
or any content stored in git.

KSA EC12 (`core-is-domain-agnostic`) means no Rust syntax knowledge, no
GitHub API knowledge, no email parsing in core. Git diff computation is
not domain-specific — it detects WHAT changed (file paths). Plugins
determine HOW to interpret changes.

For non-git data sources (KSA data lakes), delta detection is the
source-kind plugin's responsibility. Git delta stays in core because
every Patina project IS a git repo. Future data lake plugins provide
their own change detection (API pagination cursors, sync tokens, etc.).

Evidence: `if-its-patina-its-git` belief, `scrape/mod.rs` (all scrapers
work with git working tree).

---

## Approach

### Phase 1: Diff-driven dispatch + lazy loading

Invert `execute_all()` from "for each scraper, run it" to "compute delta,
route to scrapers with work."

**Step 1: Delta computation.**
Add `compute_delta()` to `scrape/mod.rs` that returns a `ScrapeDelata`:

```rust
struct ScrapeDelta {
    /// New commits since last scrape (from events.db high-water mark)
    new_commits: Vec<String>,
    /// Changed file paths with their extensions
    changed_files: Vec<(PathBuf, Option<String>)>,  // (path, extension)
    /// Whether layer/ directory has changes
    layer_changed: bool,
    /// Whether belief files changed (or code changed → regrounding needed)
    beliefs_affected: bool,
}
```

Delta sources (checked in order):
1. `git log` since last scrape commit hash (stored in `events.db` measure)
2. `index_state` mtime comparison for code files (existing, per D1 from
   data-fast-incremental)
3. Layer file mtime check (existing pattern from layer scraper)

If all deltas are empty → return immediately. EC1: `scrape-zero-work-is-noop`.

**Step 2: Source-kind routing.**
Replace the hardcoded 4-call sequence with delta-driven dispatch:

```rust
fn execute_all() -> Result<()> {
    let delta = compute_delta()?;
    if delta.is_empty() {
        println!("Nothing changed — skipping scrape");
        return Ok(());  // EC1: < 500ms
    }

    // Route by source kind — only invoke scrapers with work
    if !delta.new_commits.is_empty() {
        scrape_git(&delta)?;           // co-changes, FTS5 for new commits only
    }
    if !delta.changed_code_files().is_empty() {
        scrape_code(&delta)?;          // only changed files, lazy plugin load
    }
    if delta.layer_changed {
        scrape_layer(&delta)?;         // only changed patterns/sessions
    }
    if delta.beliefs_affected {
        scrape_beliefs(&delta)?;       // only beliefs referencing changed paths
    }
    Ok(())
}
```

This is the dispatch interface KSA EC1 will generalize. Future source
kinds (forge plugin, google-workspace plugin) register as additional
arms. The delta → classify → route pattern stays.

**Step 3: Lazy plugin loading.**
`discover_pipeline_plugins()` currently loads ALL grammar plugins
unconditionally (`extract_v2.rs:442-488`). Change to:

```rust
fn discover_pipeline_plugins_for(
    extensions: &HashSet<String>,
) -> HashMap<String, LoadedPipelinePlugin> {
    // Read manifests (cheap — TOML parse only)
    // Only compile WASM for plugins claiming affected extensions
}
```

Extensions come from `delta.changed_code_files()`. If 1 `.rs` file changed,
only `grammar-rust` is compiled. EC4: `plugins-loaded-lazily`.

**Step 4: Incremental FTS5.**
Replace `DELETE FROM code_search; INSERT ...` with per-file upsert:

```sql
DELETE FROM code_search WHERE path = ?;
INSERT INTO code_search (path, name, kind, content) VALUES (?, ?, ?, ?);
```

Only runs for files in `delta.changed_code_files()`. EC6: `fts5-incremental`.

### Phase 2: AOT WASM module cache

After Phase 1 reduces which plugins load, Phase 2 reduces per-plugin load
cost from ~120ms to ~0.1ms.

**Approach:** Pre-compile WASM to native code on first load, cache as
`.cwasm` file.

```
~/.patina/pipeline/<name>/
├── plugin.wasm       # Source WASM component
├── plugin.toml       # Manifest
└── plugin.cwasm      # AOT-compiled native code (generated)
```

Load path:
1. If `plugin.cwasm` exists AND `cwasm.mtime > wasm.mtime`:
   → `Module::deserialize_file("plugin.cwasm")` (mmap, microseconds)
2. Else:
   → `Component::new(engine, wasm_bytes)` (Cranelift JIT, ~120ms)
   → `module.serialize()` → write `plugin.cwasm`

Cache key: wasmtime version is pinned (v41 via Cargo.lock). If wasmtime
updates, `.cwasm` files are invalid — detect via header magic or version
file. Simplest: delete all `.cwasm` on `patina` binary version change.

Implementation: modify `PipelineEngine::load_component()` to check cache
before `Component::new()`. One function change, no interface change.

Evidence: `pipeline.rs:77-79` — `Component::new(wasm_engine(), wasm)`
is the hot path. Wasmtime docs confirm `Module::deserialize_file()` is
mmap-backed and sub-millisecond.

### Phase 3: Mother as warm scrape host

**Prerequisite:** KSA Phase 1 must be at least partially complete (Mother
hosts pipeline plugins, not just mother-child plugins).

Currently the hook forks a cold `patina scrape` process (`hook/internal.rs:47`).
Mother already hosts WASM plugins warm (`daemon.rs:556-576`).

Evolution:
1. Mother learns to host pipeline plugins (not just mother-child world)
2. Hook sends `{"event": "post-commit", "delta": [...]}` to Mother over UDS
3. Mother dispatches to warm grammar plugins — no Cranelift, no mmap,
   plugins are already instantiated
4. Mother writes results to DB

The plugin WIT interface does NOT change. What changes is who hosts the
plugin (CLI process → Mother daemon) and how they're loaded (cold → warm).

This phase depends on Mother's plugin registry expanding. The design should
ensure Mother's pipeline hosting uses the same `PipelineEngine::handle()`
interface — just with pre-loaded components.

### Phase 4: Incremental belief grounding

Only reground beliefs whose grounding evidence touches changed paths.

Today: all beliefs are re-grounded every run (~1.5s for 189 beliefs).
After: query grounding paths for each belief, intersect with
`delta.changed_files`, reground only the intersection.

```sql
-- Find beliefs with grounding that references changed files
SELECT DISTINCT b.id
FROM beliefs b
JOIN belief_grounding g ON b.id = g.belief_id
WHERE g.path IN (?, ?, ...)  -- changed file paths from delta
```

This is an optimization that doesn't affect the dispatch interface.
It uses the same delta that Phase 1 computes.

---

## Commits

Phase 1 (diff-driven dispatch):
1. `refactor(scrape): compute delta before dispatch` — add `compute_delta()`,
   `ScrapeDelta` struct. No behavior change yet — compute delta, log it,
   still run all scrapers.
2. `refactor(scrape): skip scrapers with zero delta` — wire delta into
   `execute_all()`, short-circuit on empty delta (EC1).
3. `refactor(scrape): lazy pipeline plugin loading` — change
   `discover_pipeline_plugins()` to accept extension filter (EC4).
4. `refactor(scrape): incremental FTS5 updates` — replace DELETE+rebuild
   with per-file upsert for `code_search` and `commits_fts5` (EC6).

Phase 2 (AOT cache):
5. `feat(plugin): AOT WASM module cache` — add `.cwasm` serialize/deserialize
   to `PipelineEngine::load_component()` (EC5).

Phase 3 (Mother warm-host):
6. `feat(mother): host pipeline plugins warm` — extend Mother to load
   pipeline-world plugins, add UDS dispatch endpoint.
7. `feat(hook): dispatch to Mother over UDS` — hook sends delta to Mother
   instead of spawning cold CLI process (EC7).

Phase 4 (incremental beliefs):
8. `refactor(scrape): incremental belief grounding` — filter beliefs by
   changed paths before regrounding.

## Key Files

- `src/commands/scrape/mod.rs` — `execute_all()` orchestration, new
  `compute_delta()` and `ScrapeDelta`
- `src/commands/scrape/code/extract_v2.rs` — `discover_pipeline_plugins()`,
  lazy loading, file dispatch loop
- `src/plugin/internal/pipeline.rs` — `PipelineEngine::load_component()`,
  AOT cache, `discover()`
- `src/plugin/internal/mod.rs` — `PluginManifest`, `PluginProvides`
- `src/commands/hook/internal.rs` — `fork_scrape()`, future: delta to Mother
- `src/commands/mother/daemon.rs` — plugin hosting, future: pipeline dispatch
- `src/commands/scrape/beliefs/` — grounding queries, future: incremental

## Open Questions

1. **Delta persistence:** Where does `compute_delta()` store the last-seen
   commit hash? Options: `events.db` measure event (already has duration_ms),
   or a dedicated `scrape_state` table in `patina.db`. Measure event is
   simpler and already exists.

2. **Mother pipeline hosting:** Mother currently only hosts mother-child
   world plugins. Hosting pipeline plugins requires a new dispatch path in
   `daemon.rs`. Should this be a new child type or a parallel plugin
   registry? Deferred to Phase 3 design.

3. **Belief grounding paths:** The current grounding system stores paths
   in belief markdown (Applied-In section), not in a queryable DB column.
   Phase 4 may need a `belief_paths` index table. Deferred to Phase 4
   design.

4. **FTS5 incremental for commits:** `commits_fts5` is rebuilt from the
   `commits` table. Incremental requires tracking which commits are already
   in the FTS5 index. The `co_changes` upsert from data-fast-incremental
   provides a pattern (INSERT ON CONFLICT).
