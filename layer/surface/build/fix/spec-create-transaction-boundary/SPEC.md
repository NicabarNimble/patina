---
type: fix
id: spec-create-transaction-boundary
status: draft
created: 2026-04-15
blocked_by:
  - durable-rust-unix-realignment-program
beliefs:
  - "[[spec-driven-design]]"
  - "[[dependable-rust]]"
  - "[[safety-boundaries]]"
  - "[[unix-philosophy]]"
related:
  - src/commands/spec/internal/create.rs
  - src/spec.rs
  - src/commands/spec/internal/queue.rs
  - src/commands/spec/internal/mutations.rs
  - layer/surface/reports/audit/2026-04-14-durable-rust-unix-realignment-audit.md
exit_criteria:
  - id: sctb1-invariants-explicit
    text: "Create flow invariants are explicit in spec + code docs before implementation (no implicit transaction assumptions)."
    checked: true
  - id: sctb2-fs-git-db-consistency
    text: "Successful create leaves filesystem, git history, and patterns DB in consistent draft state for the same spec id/path."
    checked: true
  - id: sctb3-git-failure-compensation
    text: "If git stage/commit fails after file materialization, create flow removes newly materialized spec files/directories (best-effort) and returns deterministic error."
    checked: true
  - id: sctb4-db-failure-compensation
    text: "If DB write fails after git commit, create flow returns deterministic repair guidance and records enough context to reconcile without silent drift."
    checked: true
  - id: sctb5-no-partial-incidental-side-effects
    text: "No partial side effects are silently kept: all known failure boundaries are either compensated or surfaced with explicit operator repair steps."
    checked: true
  - id: sctb6-failure-path-tests
    text: "Deterministic tests cover at least: git add/commit failure path and DB write failure path, asserting fail-closed outcomes and repair messaging."
    checked: true
  - id: sctb7-command-contract-stable
    text: "`patina spec create` CLI contract (flags/args/output envelope) remains backward-compatible."
    checked: true
---

# fix: spec create transaction boundary

## Problem

`create_spec_value_for_project` currently interleaves materialization, git commit, and DB update in a way that can leave partial durable side effects on failures.

## Why this is high-risk

Spec create writes durable artifacts visible to operators and collaborators (repo files + commits + local state DB). Partial failure can leave source-of-truth drift and force manual archaeology.

## Invariants (authoritative)

1. **INV-01 Success coherence**
   On success, all three surfaces agree for the new draft spec id:
   - filesystem has `layer/surface/build/<type>/<id>/SPEC.md` (and `DESIGN.md` when applicable)
   - git has the materialized files in the create commit
   - patterns DB row points at the same spec path/status

2. **INV-02 Git failure is fail-closed + compensated**
   If git stage/commit fails after files are written, create must not silently keep a half-created draft state as if create succeeded. Newly materialized files are removed best-effort and error message is deterministic.

3. **INV-03 DB failure is explicit and recoverable**
   If DB write fails after successful git commit, create must surface deterministic repair steps including exact spec id/path and reconciliation command(s). No silent success.

4. **INV-04 No contract drift at CLI boundary**
   No new flags and no response envelope changes for existing `spec create` callers.

## Target implementation shape

- Introduce a small create transaction context inside `create.rs` to track materialized paths and commit outcome.
- Keep phase boundaries explicit:
  1. validate + materialize files
  2. git stage/commit
  3. DB upsert
- Add compensation helper(s):
  - `cleanup_materialized_spec_tree(...)` for git failure path
  - deterministic repair message helper for DB failure after commit

## Non-goals

- No full rewrite of spec create architecture beyond transaction boundary hardening.
- No schema redesign of `patterns` table.
- No change to cross-project routing/authorization semantics.

## Verification

```bash
cargo check -p patina-ai
cargo test -p patina-ai commands::spec::internal::create::tests:: -- --nocapture
cargo test -p patina-ai commands::spec:: -- --nocapture
cargo run -q -- spec create fix tx-boundary-smoke --json
```
