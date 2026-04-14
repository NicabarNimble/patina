---
type: feat
id: spec-cross-project-dispatch
status: active
created: 2026-04-14
sessions:
  origin: 20260414-111352-731164000
beliefs:
  - "[[spec-driven-design]]"
  - "[[adapter-pattern]]"
  - "[[safety-boundaries]]"
  - "[[unix-philosophy]]"
related:
  - src/main.rs
  - src/spec.rs
  - src/commands/spec/mod.rs
  - src/commands/spec/internal/create.rs
  - src/commands/spec/internal/mutations.rs
  - src/commands/mother/daemon.rs
  - layer/surface/build/feat/mother-rivet-integration/SPEC.md
exit_criteria:
  - id: scd1-project-selector
    text: "`patina spec` accepts top-level `--project <path|uid>` and routes non-create operations to that target project."
    checked: true
    verify: "patina spec --project <target-path> list --json"
  - id: scd2-create-target-filesystem-truth
    text: "Cross-project `spec create` writes SPEC.md (and DESIGN.md where applicable) under target project's `layer/surface/build/...`, not caller cwd."
    checked: true
    verify: "test -f <target>/layer/surface/build/feat/<id>/SPEC.md"
  - id: scd3-provenance-link
    text: "Cross-project creates auto-link source project uid into `related` as `origin-project:<uid>`."
    checked: true
    verify: "grep -n 'origin-project:' <target>/layer/surface/build/feat/<id>/SPEC.md"
  - id: scd4-session-lock-default
    text: "Cross-project creates fail closed when target project has no active session (unless override provided)."
    checked: true
    verify: "patina spec create feat blocked-test --project <target-without-session>"
  - id: scd5-force-override
    text: "`--force-cross-project` override allows operator-driven creation when session lock is intentionally bypassed."
    checked: true
    verify: "patina spec create feat override-test --project <target> --force-cross-project"
  - id: scd6-query-routing
    text: "Read flows (`list/show/check/...`) execute against the selected target project and return that project's spec state."
    checked: true
    verify: "patina spec --project <target> show <id> --json"
  - id: scd7-mutation-routing
    text: "Mutation flows (`set/promote/pause/...`) execute in the selected target project and commit there."
    checked: true
    verify: "patina spec --project <target> set <id> target q1 --json"
  - id: scd8-backward-compat
    text: "Spec commands without `--project` preserve existing single-project behavior."
    checked: true
    verify: "patina spec list --json"
---
# feat: Spec cross-project dispatch

> Allow operators to run spec lifecycle commands for another Patina project from the current project without losing project ownership boundaries.

## Problem

Spec operations historically assumed one implicit project context. In multi-project work, operators need to queue and manage specs for another project while staying in their current control context.

## Goal

Enable targeted spec routing (`--project`) with guardrails:
- lifecycle operations execute in the selected target project,
- created spec files live in the target project,
- source project is linked for cross-project create provenance,
- create path is safe by default (session lock + explicit override).

## Scope

- Extend `patina spec` command surface with top-level target project selection.
- Resolve target by path or registered project uid.
- Route non-create spec commands into the target project execution context.
- Keep create semantics with cross-project provenance + session lock.

## Non-goals

- Replacing spec file source-of-truth with centralized storage.
- Designing Slate-native spec ownership in this slice (future migration lane).

## Verification

```bash
# query route
patina spec --project /abs/path list --json

# mutation route
patina spec --project /abs/path set <id> target q1 --json

# create provenance
grep -n "origin-project:" /abs/path/layer/surface/build/feat/<id>/SPEC.md
```

## Notes

This slice keeps filesystem truth in target repos while giving Mother/CLI routing metadata to operate across project boundaries now.
