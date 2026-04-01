---
type: fix
id: car-cleanup-non-a
status: draft
created: 2026-03-31
sessions:
  origin: 20260331-224232-852361000
references:
  - layer/core/unix-philosophy.md
  - layer/core/spec-driven-design.md
  - layer/core/session-capture.md
related:
  - layer/surface/build/fix/code-audit-remediation/SPEC.md
exit_criteria:
  - id: car-deprecated-serve
    text: "Remove empty deprecated serve module path."
    checked: false
  - id: car-deprecated-flags
    text: "Remove deprecated scry flags (--legacy, --full)."
    checked: false
  - id: car-upgrade-placeholder
    text: "Gate placeholder upgrade flow behind dev feature so default build cannot hit mock URL/fallback path."
    checked: false
  - id: car-build-md-version
    text: "Update layer/core/build.md version tracking to current reality."
    checked: false
  - id: car-agents-md-accuracy
    text: "Fix AGENTS.md runtime/surface references to actual directory structure."
    checked: false
  - id: car-core-tools-decision
    text: "Remove premature core_tools re-export module if unused/canonically redundant."
    checked: false
  - id: car-stale-specs-archived
    text: "Archive completed/abandoned stale specs through official spec lifecycle commands."
    checked: false
  - id: car-cleanup-proof
    text: "Compile and command help checks pass; docs and spec inventory reflect cleanup outcomes."
    checked: false
---

# fix: Code Audit Remediation — Cleanup (non-A)

Cleanup/deprecation/documentation hygiene only.

## Constraints

- No architecture moves.
- No behavior changes outside explicit deprecation removals.
- Use official spec lifecycle workflow for archival operations.
