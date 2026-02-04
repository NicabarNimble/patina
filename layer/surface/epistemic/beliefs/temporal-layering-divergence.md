---
type: belief
id: temporal-layering-divergence
persona: architect
facets: [architecture, technical-debt, evolution]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-02-04
revised: 2026-02-04
---

# temporal-layering-divergence

Features built in sequence without retiring the old path create parallel implementations that diverge — the accumulated debt grows faster than the features themselves

## Statement

Features built in sequence without retiring the old path create parallel implementations that diverge — the accumulated debt grows faster than the features themselves

## Evidence

- [[session-20260204-103550]]: [[analysis-three-servers.md]] - CLI (Nov 25) → serve wrapper (Dec 3) → MCP+QueryEngine (Dec 12) → --hybrid bridge (Dec 16): each layer added a parallel search path instead of replacing the old one, resulting in three independent implementations with duplicated formatting, logging, persona bolting, and result types (weight: 0.95)

## Supports

- [[mcp-is-shim-cli-is-product]] — D0 unification driven by recognizing CLI should own the pipeline, not three parallel paths
- [[duplicate-before-extract]] — the inverse: sometimes duplication is intentional before extracting the right abstraction. This belief captures when duplication is *accidental* from temporal layering

## Attacks

<!-- Add beliefs this defeats -->

## Attacked-By

<!-- Add beliefs that challenge this -->

## Applied-In

- [[d0-unified-search/SPEC.md]] — D0 exists specifically to fix this: unify CLI/MCP/serve into one QueryEngine pipeline
- `src/commands/serve/internal.rs:handle_scry()` — the most damaged artifact: started as thin CLI wrapper, accumulated both paths in an `if hybrid` branch
- `src/commands/scry/mod.rs:execute()` — CLI path that was never migrated to QueryEngine (kept alive by `--hybrid` opt-in)

## Revision Log

- 2026-02-04: Created — metrics computed by `patina scrape`
