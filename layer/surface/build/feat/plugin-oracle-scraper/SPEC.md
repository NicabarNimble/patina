---
type: feat
id: plugin-oracle-scraper
status: abandoned
created: 2026-02-12
blocked_by:
- plugin-command-extractions
sessions:
  origin: 20260212-083400
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

## Scope Change (2026-02-13)

Per [[plugin-ecosystem]] spec alignment session, this spec is **split**:

- **Scraper** → subsumes into **pipeline world** (`handle` dispatch with
  `{"op": "scrape", ...}`). Pipeline is the pure-compute world for host-invoked
  data processing. Scraper plugins receive bytes and return structured output.
- **Oracle** → **stays host-side**, not a plugin world. Oracles affect retrieval
  correctness, performance, and security. The internal oracle trait should be
  designed as-if-pluggable (clean inputs/outputs, no global state) so future
  Phase 6+ extraction is possible without a rewrite. For now, oracle providers
  are Rust modules in `src/retrieval/`.

## Build Steps (Revised)

1. ~~Define `wit/oracle.wit`~~ — oracle stays host-side
2. Scraper uses pipeline world (`wit/pipeline/pipeline.wit`) with `handle()` dispatch
3. ~~Refactor `retrieval/oracle.rs` for WASM~~ — keep host-side, design trait as-if-pluggable
4. Refactor `scrape code` — scraper pipeline checks for WASM pipeline plugins with `{"op": "scrape"}` capability
5. ~~Create example oracle plugin~~ — not applicable
6. Create example scraper pipeline plugin

## WIT Worlds (Revised)

| World | Entry point | Imports | Capabilities |
|-------|-------------|---------|-------------|
| ~~`oracle`~~ | ~~`query()`, `name()`~~ | ~~none~~ | ~~Stays host-side~~ |
| `pipeline` | `handle(json) -> result<string, string>` | `patina:host/log` only | Pure computation, no filesystem |

## Acceptance Criteria (Revised)

- [ ] ~~WASM oracle participates in scry fusion~~ — deferred (oracle stays host-side)
- [ ] WASM scraper (pipeline plugin) runs during `patina scrape code` for matching patterns
- [ ] Scraper pipeline plugin: pure computation via `handle()`, no host imports beyond log
- [ ] Internal oracle trait has clean inputs/outputs suitable for future pluggability

## Non-Goals

- ~~Replacing existing compiled-in oracles/scrapers~~ → Oracles not plugin-ified
- Plugin marketplace or distribution
- Real-time scraper watching
- Oracle plugins (deferred to Phase 6+)

---

## Status Log

| Date | Status | Note |
|------|--------|------|
| 2026-02-12 | design | Extracted from [[plugin-system]] Phase 4. Blocked by command extractions. |
| 2026-02-13 | amended | Per [[plugin-ecosystem]] spec alignment: scraper subsumes into pipeline world (`handle` dispatch). Oracle stays host-side — not a plugin world. Internal oracle trait designed as-if-pluggable for future Phase 6+. |
