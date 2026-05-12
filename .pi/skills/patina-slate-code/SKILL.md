---
name: patina-slate-code
description: Use before adding, editing, refactoring, or fixing code in the Patina repo. Ensures code changes are grounded in Slate work, user alignment, proof plans, closure evidence, and belief/spec/Allium anchors when appropriate.
---

# Patina Slate Code Gate

Bridge skill for Pi until Mother can broker child-owned skills directly.

Canonical child package: `children/slate-manager/skills/slate-code/SKILL.md`.
Version-control package: `children/slate-manager/skills/slate-version-control/SKILL.md`.

Use this skill before making source-code changes in the Patina repository.

Slate is the preferred path for Patina build/refactor/fix work. It keeps code changes attached to user intent, proof plans, closure evidence, and durable project artifacts under `layer/slate/`.

## Trigger

Load this skill when the user asks to:

- add code
- edit code
- refactor code
- fix a bug
- add CLI/API/runtime behavior
- change Mother/Child/Toy infrastructure
- modify WIT/WASI children or toys
- change tests for implementation behavior

For purely conversational answers, no Slate is required unless the conversation becomes a code-changing task.

## Required workflow before editing code

1. Read relevant project context first:
   - `AGENTS.md`
   - relevant files under `layer/`
   - relevant source files before editing
2. Check whether an active/ready Slate already covers the work.
3. If no suitable Slate exists, create one before source edits.
4. Ensure the Slate has:
   - clear `human_request`
   - `user_alignment`
   - at least one actionable `implementation_plan` item
   - a checkable `proof_plan`
   - relevant `allium_anchors` when behavior/product intent exists
   - relevant `belief_refs` when doctrine is being applied
5. Promote the Slate to `active` before changing source.
6. During implementation, add closure evidence as facts are proven.
7. Before marking complete, run the proof plan and update each criterion to `[x]`.
8. Complete only after proof is satisfied; archive only when appropriate.

## Current CLI path

Until a first-class `patina slate ...` wrapper exists, use the installed `slate-manager` child.

Local Patina install uses the guest project path:

```json
{"project":"/project"}
```

Create work:

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

Add proof / implementation items:

```bash
patina child call slate-manager 'patina:slate/control@0.1.0.set-work' '[{
  "project":"/project",
  "id":"short-kebab-id",
  "field":"proof_plan",
  "value":"[ ] Observable proof criterion."
}]'
```

Promote:

```bash
patina child call slate-manager 'patina:slate/control@0.1.0.promote-work' '[{"project":"/project","id":"short-kebab-id","force":false}]'
```

Show / check / complete:

```bash
patina child call slate-manager 'patina:slate/control@0.1.0.show-work' '[{"project":"/project","id":"short-kebab-id"}]'
patina child call slate-manager 'patina:slate/control@0.1.0.check-work' '[{"project":"/project","id":"short-kebab-id"}]'
patina child call slate-manager 'patina:slate/control@0.1.0.complete-work' '[{"project":"/project","id":"short-kebab-id","force":false}]'
```

## Slate vs `patina spec`

Slate and `patina spec` are separate islands, but for now Slate archive/version behavior should mirror spec behavior where possible.

- `patina spec` tracks behavioral specs with spec lifecycle/version-style release/archive flows.
- Slate tracks implementation work transactions under `layer/slate/work/<id>/work.toml` plus `layer/slate/events.jsonl` history.
- Slate should use spec-parity archive semantics for now: terminal work, clean tracked tree, archive commit, and recovery tag such as `slate/<id>`.
- Slate truth is the work file plus event history plus git commits/tags around the change.
- Use Allium/spec anchors when product behavior needs durable behavioral intent; use Slate for the concrete code-change transaction.

See `children/slate-manager/skills/slate-version-control/SKILL.md`.

## Do not

- Do not make non-trivial Patina code changes without an active Slate unless the user explicitly waives it.
- Do not invent Allium anchors for implementation-only details.
- Do not use legacy child manifest `[capabilities]` / `[toys]`; Patina uses `[needs].toys` and optional `[needs.scopes]`.
- Do not commit machine-specific child mounts such as local `/Users/...` read-write mounts.
