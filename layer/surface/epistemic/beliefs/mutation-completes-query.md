---
type: belief
id: mutation-completes-query
persona: architect
facets: [architecture, spec-system, workflow]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-02-23
revised: 2026-02-23
---

# mutation-completes-query

Query commands without mutation commands are dead infrastructure — the mutation side completes the loop

## Statement

Query commands without mutation commands are dead infrastructure — the mutation side completes the loop

## Evidence

- [[session-20260223-092355]]: [[session-20260223-092355]] - spec blocked and spec ready queries exist in src/commands/spec/internal.rs but show empty results because no spec block/spec unblock commands exist to populate blocked_by YAML fields. Infrastructure sits unused without the write path. (weight: 0.9)

## Supports

- [[spec-first]] — specs need operational tooling to be effective, not just schema

## Attacks

<!-- Add beliefs this defeats -->

## Attacked-By

<!-- Add beliefs that challenge this -->

## Applied-In

- `src/commands/spec/internal.rs` — `get_blocked_specs()` and `get_ready_specs()` query `spec_deps` table but no `spec block` command exists to populate it. Result: queries return empty, infrastructure is dead.

## Revision Log

- 2026-02-23: Created — metrics computed by `patina scrape`
