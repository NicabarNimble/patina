---
type: feat
id: plugin-oracle-scraper
status: design
created: 2026-02-12
sessions:
  origin: 20260212-083400
blocked_by:
  - plugin-command-extractions  # Command extractions prove bulk extraction pattern
blocks: []
related:
  - layer/surface/build/feat/plugin-system/SPEC.md
  - layer/surface/build/feat/plugin-command-extractions/SPEC.md
  - layer/surface/build/feat/plugin-grammars/SPEC.md
  - layer/surface/build/explore/wit-interfaces/SPEC.md
beliefs:
  - patina-is-knowledge-layer
  - separate-worlds-for-isolation
  - wasi-sandboxed-filesystem
  - compiler-enforced-safety
---

# feat: Oracle & Scraper Plugins (v0.19.0)

> Make the serve and capture pipelines extensible. Third-party oracles
> and scrapers can be loaded as WASM plugins. New WIT worlds with
> capability isolation per [[separate-worlds-for-isolation]].

## Problem

The oracle and scraper pipelines are compiled-in only. Adding a new
oracle or scraper means modifying core code and recompiling patina.
Third-party extensions are impossible. The WIT interfaces for both
are already sketched in [[wit-interfaces]].

## Origin

Extracted from [[plugin-system]] Phase 4 during session [[20260212-083400]].
The original spec was too large to contain all 5 phases. This spec owns
extensible serve/capture; [[plugin-system]] owns the runtime.

## Build Steps

1. Define `wit/oracle.wit` — `patina:oracle@0.1.0` world (from [[wit-interfaces]])
2. Define `wit/scraper.wit` — `patina:scraper@0.1.0` world (from [[wit-interfaces]])
3. Refactor `retrieval/oracle.rs` — oracle fusion queries both compiled-in and WASM oracles
4. Refactor `scrape code` — scraper pipeline checks for WASM scrapers matching file extension
5. Create example oracle plugin
6. Create example scraper plugin

## WIT Worlds

| World | Exports | Imports | Capabilities |
|-------|---------|---------|-------------|
| `oracle` | `query()`, `name()`, `is-available()` | none | Pure computation |
| `scraper` | `scrape-file()`, `patterns()` | `wasi:filesystem` (read-only) | Filesystem read |

## Acceptance Criteria

- [ ] WASM oracle participates in scry fusion alongside compiled-in oracles
- [ ] WASM scraper runs during `patina scrape code` for matching file patterns
- [ ] Oracle plugin: pure computation, no capabilities required
- [ ] Scraper plugin: `wasi:filesystem` read-only, sandboxed to project directory

## Non-Goals

- Replacing existing compiled-in oracles/scrapers (extend, not replace)
- Plugin marketplace or distribution
- Real-time scraper watching

---

## Status Log

| Date | Status | Note |
|------|--------|------|
| 2026-02-12 | design | Extracted from [[plugin-system]] Phase 4. Blocked by command extractions. |
