---
type: feat
id: plugin-command-extractions
status: design
created: 2026-02-12
sessions:
  origin: 20260212-083400
blocked_by:
  - plugin-system  # Phase 2 (command world) must ship first
blocks: []
related:
  - layer/surface/build/feat/plugin-system/SPEC.md
  - layer/surface/build/feat/plugin-grammars/SPEC.md
  - layer/surface/build/feat/plugin-oracle-scraper/SPEC.md
beliefs:
  - patina-is-knowledge-layer
  - patina-identity
  - dependable-rust
  - compiler-enforced-safety
---

# feat: Plugin Command Extractions (v0.18.0)

> Extract yolo, eval+bench, report, upgrade from the binary into WASM
> command plugins. These are the "Definitely Plugin" modules from
> [[patina-identity]]. Requires the `command` world from Phase 2 of
> [[plugin-system]].

## Problem

The binary is 52MB with every feature compiled in. [[patina-identity]]
identifies 5 modules as "Definitely Plugin" — they don't serve core
functions (capture, index, serve, govern, connect, protect) and Patina
can function without them. Phase 2 of [[plugin-system]] proves the
`command` world with doctor. This spec is the bulk extraction.

## Origin

Extracted from [[plugin-system]] Phase 3 during session [[20260212-083400]].
The original spec was too large to contain all 5 phases. This spec owns
the extraction work; [[plugin-system]] owns the runtime and first
extractions (Phases 1-2).

## Plugins to Extract

| Plugin | Lines | World | Capabilities | Notes |
|--------|-------|-------|-------------|-------|
| `patina-yolo` | 1,613 | **task** | host_layer (read), toys (cargo) | Spawns cargo build — needs toys, task world |
| `patina-eval` | 2,476 | command | host_database (read), host_layer (read) | Read-only analysis |
| `patina-bench` | 753 | command | host_database (read) | Read-only analysis |
| `patina-report` | ~400 | command | host_layer (read), host_database (read) | Read-only report generation |
| `patina-upgrade` | 162 | **task** | host_http (github.com) | Downloads binary, mutates system — needs http + task world |

## Acceptance Criteria

- [ ] All 5 plugins work identically as WASM
- [ ] Binary size reduced measurably (target: <40MB from 52MB)
- [ ] `patina plugin list` shows all default plugins
- [ ] Removing a plugin.wasm file gracefully degrades (command not found, not crash)

## Non-Goals

- New features in extracted modules (extract first, improve later)
- Plugin auto-update
- Plugin dependencies between extracted modules

---

## Status Log

| Date | Status | Note |
|------|--------|------|
| 2026-02-12 | design | Extracted from [[plugin-system]] Phase 3. Blocked by Phase 2 command world. |
| 2026-02-13 | amended | Per [[plugin-ecosystem]] spec alignment: yolo and upgrade reclassified as **task** world plugins (they mutate the system via toys/http). Requires task world to exist before extraction. eval, bench, report remain command world. |
