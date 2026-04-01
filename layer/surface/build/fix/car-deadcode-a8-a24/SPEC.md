---
type: fix
id: car-deadcode-a8-a24
status: ready
created: 2026-03-31
sessions:
  origin: 20260331-224232-852361000
related:
- layer/surface/build/fix/code-audit-remediation/SPEC.md
- layer/surface/build/fix/car-architecture-a7-a25/SPEC.md
- layer/surface/build/fix/car-dedup-a9-a21/SPEC.md
- src/embeddings/database.rs
- src/embeddings/mod.rs
- src/db/sqlite.rs
- src/db/mod.rs
- src/commands/scrape/code/database.rs
- src/commands/dev/bump_version.rs
- src/commands/dev/sync_adapters.rs
- src/commands/dev/validate.rs
- sdk/patina-sdk-core/
- sdk/patina-sdk-data/
- sdk/patina-sdk-agent/
- src/commands/scry/internal/logging.rs
- src/commands/scry/internal/semantic.rs
- src/child/toy_host/v2.rs
- src/child/internal/host_support.rs
- src/beliefs/graph_host.rs
- src/git/writer.rs
- AGENTS.md
beliefs:
- '[[canonical-module-bypass-compounds]]'
- '[[wasi-is-foundation-not-option]]'
references:
- layer/core/unix-philosophy.md
- layer/core/spec-driven-design.md
- layer/surface/build/feat/child-construction-canon/SPEC.md
exit_criteria:
- id: car-a8-dead-embeddings-db
  text: Delete dead embeddings/database module. Children emit facts via toys, not direct DB writes — this module is from a superseded approach.
  checked: true
- id: car-a10-dead-db-wrapper
  text: Delete db/sqlite wrapper modules. Per [[canonical-module-bypass-compounds]], 50+ call sites bypass it. Migrate remaining consumer to direct rusqlite.
  checked: true
- id: car-a11-dead-dev-commands
  text: Delete dead dev command code paths that reference nonexistent files (src/adapters/claude.rs, .patina/version_manifest.json).
  checked: true
- id: car-a12-dead-sdk-tiers
  text: Delete sdk/patina-sdk-{core,data,agent}/ from disk. Per [[child-construction-canon]], the child/toy model uses manifest + runtime enforcement, not compile-time SDK tiers. AGENTS.md updated to reflect umbrella SDK as canonical surface.
  checked: true
- id: car-a14-dead-query-id
  text: Remove LAST_QUERY_ID dead global and all writes.
  checked: true
- id: car-a15-dead-persona-check
  text: Remove dead persona source check in scry semantic path.
  checked: true
- id: car-a22-blanket-dead-code
  text: 'Replace blanket #![allow(dead_code)] in toy_host/v2.rs and host_support.rs with per-item annotations. Distinguish: functions consumed by WASM bindgen or part of the granted toybox get #[allow(dead_code)]. Functions with zero toy consumers get deleted.'
  checked: true
- id: car-a23-graph-tag-stub
  text: Delete silent no-op 'tag' graph action so unknown actions error explicitly.
  checked: true
- id: car-a24-dead-none-writer
  text: Delete NoneWriter. Keep ForgeWriter trait — additional forge backends (Gitea) are a plausible future direction. But NoneWriter has no callers.
  checked: true
- id: car-deadcode-proof
  text: '`cargo check --workspace -q` and `cargo test -q --lib` pass after deletions with no replacement stubs.'
  checked: true
---

# fix: Code Audit Remediation — Dead Code (A8, A10-A12, A14-A15, A22-A24)

Delete-only/removal-focused spec. No functional expansion.

## Context

Patina's architecture: protocol core (native CLI verbs), Mother (daemon), children (WASM legos), toys (sandbox capability grants). Dead code in this context means: code from superseded approaches (A8 pre-oxidize embeddings, A10 multi-backend db), code referencing files that don't exist (A11), code from an SDK architecture that [[child-construction-canon]] replaced (A12), and unused stubs/globals (A14, A15, A23, A24).

A22 (blanket dead_code allows on toy host files) requires care: these files implement the toy host interface that Mother provides to children. Some functions may appear "dead" from Rust's perspective but are called via WASM bindgen or are part of the toybox surface that no current child uses yet. Only delete functions that are genuinely unreachable, not just currently unused.

## Dependencies

- **A8 before A10**: A8 deletes `embeddings/database.rs`, one of A10's 2 consumers of `db/sqlite.rs`.
- All other gates are independent.

## Constraints

- Prefer deletion over hiding.
- Do not introduce replacement abstractions unless required to compile.
- Keep each deletion in narrow commit slices.
- A22: distinguish "dead to Rust" from "live to WASM/toybox." Toys that are part of the granted capability surface but not yet used by a shipped child are NOT dead code.
