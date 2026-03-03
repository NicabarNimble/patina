---
type: belief
id: dispatch-is-two-levels
persona: architect
facets: [architecture, plugin-system, scrape]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-03-03
revised: 2026-03-03
---

# dispatch-is-two-levels

Scrape dispatch operates at two complementary levels — source-kind routing (which scraper to invoke: code, forge, layer, beliefs) and file-type routing (which grammar plugin handles which extension within a source kind). These are not competing interfaces but layered: source-kind dispatch selects the scraper, file-type dispatch selects the plugin within it. Conflating the two leads to interface designs that try to do both and do neither well.

## Statement

Scrape dispatch operates at two complementary levels — source-kind routing (which scraper to invoke: code, forge, layer, beliefs) and file-type routing (which grammar plugin handles which extension within a source kind). These are not competing interfaces but layered: source-kind dispatch selects the scraper, file-type dispatch selects the plugin within it. Conflating the two leads to interface designs that try to do both and do neither well.

## Evidence

- [[session-20260303-101839]]: Alignment analysis of scrape-diff-driven vs knowledge-system-architecture revealed SDD says 'dispatch by file type' while KSA says 'dispatch by source kind' — seemed contradictory until code evidence showed they operate at different levels: execute_all() routes by source kind (scrape/mod.rs:61-84), discover_pipeline_plugins() routes by file extension within code scraper (extract_v2.rs:442-488) (weight: 0.9)

## Supports

- [[patina-is-domain-agnostic-knowledge-system]] — layered dispatch enables domain-agnostic core: source-kind routing is infrastructure, file-type routing is domain knowledge in plugins
- [[unix-philosophy]] — each level does one job: source-kind routing classifies the delta, file-type routing matches extensions to grammar plugins

## Attacks

<!-- Add beliefs this defeats -->

## Attacked-By

<!-- Add beliefs that challenge this -->

## Applied-In

- [[spec-scrape-diff-driven]] DESIGN.md § A1 — dispatch interface designed as two explicit levels
- [[spec-knowledge-system-architecture]] DESIGN.md § Phase 1 — KSA extends source-kind level while preserving file-type level
- `src/commands/scrape/mod.rs:61-84` — `execute_all()` is source-kind dispatch (level 1)
- `src/commands/scrape/code/extract_v2.rs:442-488` — `discover_pipeline_plugins()` is file-type dispatch (level 2)

## Revision Log

- 2026-03-03: Created — metrics computed by `patina scrape`
