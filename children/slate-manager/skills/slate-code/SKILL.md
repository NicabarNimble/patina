---
name: slate-code
description: Slate child skill for Patina build/refactor/fix code changes. Use before editing Patina source so work is tied to a Slate item, proof plan, closure evidence, and relevant beliefs/Allium anchors.
---

# Slate Code Skill

Use this child skill before non-trivial Patina code changes.

Slate is the preferred work transaction for build/refactor/fix changes. It captures user alignment, implementation plan, proof plan, closure evidence, belief harvest, and version/archive boundaries.

## Applies when

- adding, editing, refactoring, or fixing code
- adding CLI/API/runtime behavior
- changing Mother/Child/Toy infrastructure
- modifying WIT/WASI children or toys
- changing tests for implementation behavior

Pure conversation does not need Slate unless it turns into code-changing work.

## Workflow

1. Ask Mother for active children and skills when that surface exists.
2. Create or reuse a Slate work item before source edits.
3. Ensure the work item has:
   - `human_request`
   - `user_alignment`
   - `implementation_plan`
   - checkable `proof_plan`
   - relevant `allium_anchors` for behavioral/product intent
   - relevant `belief_refs` for doctrine
4. Promote to `active` before implementation.
5. Add closure evidence as facts are proven.
6. Complete only when proof plan is checked and belief harvest is resolved.
7. Archive using Slate version-control semantics after checkpoint/commit boundaries are safe.

## Current command bridge

Until `patina slate ...` and `patina mother skills ...` exist, call the child directly.

Local project mount convention:

```json
{"project":"/project"}
```

Create:

```bash
patina child call slate-manager 'patina:slate/control@0.1.0.create-work' '[{
  "project":"/project",
  "id":"short-kebab-id",
  "title":"Short human title",
  "kind":"build",
  "human-request":"What the user asked for.",
  "allium-anchors":[],
  "user-alignment":"Why this matches the user's request and constraints."
}]'
```

Set plan/proof/evidence:

```bash
patina child call slate-manager 'patina:slate/control@0.1.0.set-work' '[{
  "project":"/project",
  "id":"short-kebab-id",
  "field":"proof_plan",
  "value":"[ ] Observable proof criterion."
}]'
```

Promote/check/complete:

```bash
patina child call slate-manager 'patina:slate/control@0.1.0.promote-work' '[{"project":"/project","id":"short-kebab-id","force":false}]'
patina child call slate-manager 'patina:slate/control@0.1.0.check-work' '[{"project":"/project","id":"short-kebab-id"}]'
patina child call slate-manager 'patina:slate/control@0.1.0.complete-work' '[{"project":"/project","id":"short-kebab-id","force":false}]'
```

## Relationship to spec

Slate and `patina spec` remain separate islands, but for now Slate version/archive behavior should mirror spec behavior where possible. See `../slate-version-control/SKILL.md`.
