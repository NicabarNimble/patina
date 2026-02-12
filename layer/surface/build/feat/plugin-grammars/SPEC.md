---
type: feat
id: plugin-grammars
status: design
created: 2026-02-12
sessions:
  origin: 20260212-083400
blocked_by:
  - plugin-oracle-scraper  # Scraper world proves WASM on scrape hot path
blocks: []
related:
  - layer/surface/build/feat/plugin-system/SPEC.md
  - layer/surface/build/feat/plugin-oracle-scraper/SPEC.md
  - layer/surface/build/feat/plugin-command-extractions/SPEC.md
beliefs:
  - patina-is-knowledge-layer
  - coupling-is-complexity
  - compiler-enforced-safety
---

# feat: Grammar Plugins (v0.20.0)

> Load tree-sitter grammars from WASM instead of compiling them in.
> Most complex integration due to tree-sitter ABI versioning and
> scrape hot path. Built last because grammars have highest coupling
> and regression risk per [[coupling-is-complexity]].

## Problem

9 tree-sitter grammars are compiled into the binary via patina-metal
(`cc::Build` + vendored C sources). This contributes significantly to
binary size and means adding a new language requires recompiling patina.
The grammars are tightly coupled: ABI versioning (0.24 expects ABI
13-14), 8 language processors on the scrape hot path, and the
patina-metal build system.

## Origin

Extracted from [[plugin-system]] Phase 5 during session [[20260212-083400]].
The original spec was too large to contain all 5 phases. This is built
last because PluginEngine is proven by this point and the coupling risk
is highest here.

## Why Last

Session [[20260211-133159]] discovered grammars are the most coupled
existing subsystem, not the least. The original ordering assumed "no
host imports = simplest" but ignored infrastructure coupling. Per
[[coupling-is-complexity]]: simplest payload means lowest coupling,
not fewest interface requirements.

By Phase 5/v0.20.0:
- PluginEngine proven across 3 worlds (mother-child, command, oracle/scraper)
- WASM on scrape hot path proven by scraper plugins
- Bulk extraction patterns established

## Grammar Fallback

If WASM grammar not found in `~/.patina/grammars/`, fall back to
compiled-in. Zero regression for existing users. This is the transition
strategy — grammars are loaded from WASM when present, compiled-in
otherwise.

## Acceptance Criteria

- [ ] `patina scrape code` uses WASM grammar when present
- [ ] Falls back to compiled-in grammar when WASM not present
- [ ] Adding a new language is: drop a `.wasm` file, no recompile
- [ ] WASM grammar parse speed within 2x of compiled-in

## Non-Goals

- Removing compiled-in grammars immediately (fallback stays)
- Supporting non-tree-sitter parsers
- Grammar auto-download

---

## Status Log

| Date | Status | Note |
|------|--------|------|
| 2026-02-12 | design | Extracted from [[plugin-system]] Phase 5. Blocked by oracle/scraper plugins. Built last due to highest coupling risk. |
