---
type: fix
id: car-deadcode-a8-a24
status: draft
created: 2026-03-31
sessions:
  origin: 20260331-224232-852361000
references:
  - layer/core/unix-philosophy.md
  - layer/core/spec-driven-design.md
related:
  - layer/surface/build/fix/code-audit-remediation/SPEC.md
  - layer/surface/build/fix/car-architecture-a7-a25/SPEC.md
  - layer/surface/build/fix/car-dedup-a9-a21/SPEC.md
exit_criteria:
  - id: car-a8-dead-embeddings-db
    text: "Delete dead embeddings/database module and remove re-export/callers."
    checked: false
  - id: car-a10-dead-db-wrapper
    text: "Delete db/sqlite wrapper modules after migrating remaining consumers to direct rusqlite usage."
    checked: false
  - id: car-a11-dead-dev-commands
    text: "Delete dead dev command code paths that reference nonexistent files."
    checked: false
  - id: car-a12-dead-sdk-tiers
    text: "Delete obsolete sdk tier directories and remove stale references in docs/instructions."
    checked: false
  - id: car-a14-dead-query-id
    text: "Remove LAST_QUERY_ID dead global and all writes."
    checked: false
  - id: car-a15-dead-persona-check
    text: "Remove dead persona source check in scry semantic path."
    checked: false
  - id: car-a22-blanket-dead-code
    text: "Replace blanket dead_code allows with per-item annotations or remove dead items."
    checked: false
  - id: car-a23-graph-tag-stub
    text: "Delete silent no-op 'tag' graph action path so unknown actions error explicitly."
    checked: false
  - id: car-a24-dead-none-writer
    text: "Remove NoneWriter dead polymorphism path in git writer module."
    checked: false
  - id: car-deadcode-proof
    text: "`cargo check --workspace -q` and `cargo test -q --lib` pass after deletions with no replacement stubs."
    checked: false
---

# fix: Code Audit Remediation — Dead Code (A8, A10-A12, A14-A15, A22-A24)

Delete-only/removal-focused spec. No functional expansion.

## Constraints

- Prefer deletion over hiding.
- Do not introduce replacement abstractions unless required to compile.
- Keep each deletion in narrow commit slices.
