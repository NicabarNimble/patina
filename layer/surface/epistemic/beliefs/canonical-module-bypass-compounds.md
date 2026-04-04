---
type: belief
id: canonical-module-bypass-compounds
status: active
confidence: high
entrenchment: medium
facets: [architecture, maintenance, drift]
session_origin: session-20260331-224232-852361000
created: 2026-03-31
---
# Canonical Module Bypass Compounds

When a canonical module exists but callers bypass it, duplication grows silently and the canonical module becomes a lie. This predicts where future bugs will cluster.

## Evidence

In [[session-20260331-224232-852361000]], the full code audit found two clear instances:
- `src/paths.rs` declares itself the single source of truth for path resolution, but 10+ sites across the codebase hardcode `.patina/` paths without importing from it. `src/project/internal.rs:312-334` reimplements the entire path hierarchy.
- `src/db/sqlite.rs` provides a `SqliteDatabase` wrapper with only 2 consumers, while 50+ call sites use `rusqlite::Connection::open()` directly. The abstraction is effectively dead.

Both cases share the same pattern: a canonical module was created, some code adopted it, but most code bypassed it. Over time the bypass became the norm and the canonical module became misleading documentation.

## Test

When creating a canonical module (single source of truth, abstraction layer, shared utility): grep for bypass patterns within one release. If bypass exceeds adoption, either enforce the canonical path or remove the pretense.

## Connects

- [[dependable-rust]] — black-box modules only work if callers actually use them
- [[patina-identity]] — infrastructure that isn't used is noise, not infrastructure
