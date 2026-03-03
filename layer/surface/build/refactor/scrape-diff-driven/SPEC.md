---
type: refactor
id: scrape-diff-driven
status: active
created: 2026-03-03
sessions:
  origin: 20260303-090741
related:
- data-fast-incremental
- knowledge-system-architecture
beliefs:
- correctness-by-construction-not-convention
- patina-is-domain-agnostic-knowledge-system
exit_criteria:
- id: scrape-zero-work-is-noop
  text: '`patina scrape` with no new commits and no changed files completes in < 500ms'
  checked: true
- id: scrape-single-commit-under-2s
  text: incremental scrape after a single commit completes in < 2s (the EC1 data-fast-incremental deferred)
  checked: true
- id: diff-drives-dispatch
  text: scrape computes the delta (changed files) once, dispatches only to extractors that handle those file types
  checked: true
- id: plugins-loaded-lazily
  text: WASM grammar plugins are only loaded when the diff contains files of their claimed language — 0 changed .rs files means grammar-rust is not loaded
  checked: true
- id: aot-module-cache
  text: WASM modules are serialized to .cwasm after first compile; subsequent loads use deserialize_file (mmap, no Cranelift)
  checked: true
- id: fts5-incremental
  text: code_search FTS5 index updates only rows for changed files — no DELETE + full rebuild when < 100% of files changed
  checked: true
- id: mother-scrape-dispatch
  text: hook can optionally send diff event to Mother over UDS; Mother dispatches to warm plugins — cold start is zero
  checked: false
---
# refactor: Diff-Driven Scrape Core with AOT Plugin Cache

> `patina scrape` today runs 4 scrapers sequentially, each independently
> discovering what changed. After [[data-fast-incremental]] delivered 60%
> improvement (16s → 6.4s), the remaining cost is fixed overhead: WASM
> plugin loading (~2s), FTS5 full rebuilds (~1s), belief grounding (~1.5s),
> git tag/tracked-file reindex (~1s). All of this runs unconditionally,
> even when there is zero work to do. The correct amount of work for
> "nothing changed" is zero.

## Problem

Scrape's control flow is **scraper-driven**: for each scraper, run it.
Each scraper independently figures out what changed. This means:

1. **WASM cold start on every invocation.** 17 grammar plugins are
   compiled via Cranelift JIT (~120ms each, ~2s total) even when 0 files
   need parsing. `discover_pipeline_plugins()` runs before the file walk.

2. **FTS5 rebuilds are unconditional.** `code_search` and `commits_fts5`
   do DELETE + full rebuild every run (~1s combined). SQLite FTS5 supports
   incremental INSERT/DELETE by rowid, but the code doesn't use it.

3. **No shared delta.** The git scraper knows the diff (it parsed the
   commits), but that information doesn't flow to the code scraper or
   belief scraper. Each scraper independently discovers there's nothing
   to do — after doing expensive setup.

4. **Belief grounding is unconditional.** All 189 beliefs are re-grounded
   every run (~1.5s) even when 0 beliefs changed. Only beliefs referencing
   changed files need regrounding.

The architectural root: scrape doesn't compute the delta first. It can't
tell subsystems "here's what changed, process only this."

## Solution

### Phase 1: Diff-driven dispatch + lazy loading

Invert scrape's control flow from "for each scraper, run it" to:

```
commit → diff → [affected files by type]
  → load only plugins that handle affected types
  → parse only changed files
  → update only changed FTS5 rows
  → update only affected co-change pairs
  → reground only beliefs referencing changed paths
```

The delta is computed once (git diff or mtime walk) and threaded through
all subsystems. Zero changed files = zero work = millisecond exit.

### Phase 2: AOT WASM module cache

Wasmtime supports `Module::serialize()` → `Module::deserialize_file()`.
Compilation is ~120ms per module. Deserialization via mmap is microseconds.

On first load: compile + serialize to `~/.patina/pipeline/<name>/plugin.cwasm`.
On subsequent loads: `deserialize_file()` if `.cwasm` mtime > `.wasm` mtime.
Cache key includes wasmtime version (pinned to v41).

This turns per-plugin cost from ~120ms to ~0.1ms. Combined with lazy
loading (Phase 1), the common case loads 1 cached plugin for ~0.1ms
instead of 17 uncached plugins for ~2s.

### Phase 3: Mother as warm scrape host

The hook currently spawns a cold CLI process: `patina scrape`. Mother
already runs as a daemon with UDS socket at `~/.patina/run/serve.sock`
and hosts WASM plugins (mother-child world) with warm instances.

The evolution:
- Hook sends `{"event": "post-commit", "diff": [...]}` to Mother over UDS
- Mother holds grammar plugins warm in memory (already instantiated)
- Mother dispatches to warm plugins, writes results to DB
- Cold start is zero — the process is already running

This requires bridging the pipeline plugin world (per-file, stateless)
with Mother's hosting model (persistent, daemon-resident). The plugin
interface doesn't change — just who hosts it.

### Phase 4: Incremental belief grounding

Only reground beliefs that reference paths in the diff. The belief
scraper already has path-based grounding queries — filter to beliefs
whose grounding evidence touches changed files.

## Non-Goals

- **Parallel scraping.** Running extractors concurrently adds complexity.
  Diff-driven dispatch makes the sequential path fast enough.
- **Replacing WASM.** The plugin boundary is right. The cold start is
  an unconditional-loading problem, not a WASM problem. AOT cache and
  Mother warm-host solve it within the WASM architecture.
- **New plugin worlds.** This spec uses existing pipeline plugin interface.
  [[knowledge-system-architecture]] handles the broader plugin extraction.

## Relationship to Other Specs

- **[[data-fast-incremental]]**: Delivered the incremental algorithms
  (co-change upsert, mtime skip, hooks). This spec addresses the
  remaining fixed overhead that those optimizations exposed.
- **[[knowledge-system-architecture]]**: KSA EC1 (`scrape-is-plugin-dispatched`)
  requires the dispatch interface built here. Phase 1 of this spec builds
  delta-driven dispatch at two levels: source-kind routing (which scraper to
  invoke) and file-type routing (which grammar plugin to load). KSA Phase 1
  generalizes source-kind routing to support plugin-registered source kinds.
  Note: Phase 3 (Mother warm-host) requires KSA to expand Mother's plugin
  hosting beyond mother-child world — this is a mutual dependency.
- **[[data-architecture-v2]]**: Parent architecture. This spec is Phase D
  continuation — performance that was outside data-fast-incremental's scope.

## Advisors (from session 20260303-090741)

**Gjengset:** "You're doing work proportional to project size, not change
size. That's O(n) when it should be O(delta)." AOT cache is a solved
problem in the WASM ecosystem.

**Steenberg:** "You don't have 4 kinds of work. You have one event — a
commit — and it produces a diff. One pipeline, one pass."

**Kelley:** "The correct amount of work for 'nothing changed' is zero.
You're loading 17 WASM plugins to parse zero files. Every one of those
is a bug." Mother as warm host eliminates cold start entirely.
