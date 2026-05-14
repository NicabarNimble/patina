---
name: patina-mother-system
description: Top-level Patina/Mother skill router. Use for Patina repository work to discover active Mother children, child-provided skills, and preferred command surfaces before choosing lower-level skills such as Slate.
---

# Patina / Mother Skill System

This is the top-level project skill entrypoint. `AGENTS.md` should point here rather than embedding per-child workflow policy.

## Model

Mother should be the skill broker for Patina work:

1. Ask Mother what children are active/available.
2. Ask Mother what skill packages each child exposes.
3. Load the child skill needed for the task.
4. Use the child command/help surface for exact operations.

Desired shape:

```text
patina mother skills
patina mother skills list
patina mother skills show <child>
patina mother skills help <child> [skill-or-command]
```

Conceptually this should feel like `--help` for active children:

```text
Mother: slate-manager is active.
Mother: slate-manager provides skills:
  - patina-slate-code: code-change work transactions
  - patina-slate-handoff: resume/packet/handoff workflows
Mother: use `patina mother skills help slate-manager patina-slate-code`.
```

## Current bridge

The full Mother-owned skill registry is not implemented yet. Until then, use project-local skill files as a bridge.

Available child skill packages currently checked in:

- `children/slate-manager/skills/slate-code/SKILL.md` — Slate-first workflow for non-trivial Patina code changes.
- `children/slate-manager/skills/slate-version-control/SKILL.md` — spec-parity archive/version-control rules for Slate work.

Temporary Pi bridge:

- `.pi/skills/patina-slate-code/SKILL.md` — routes Pi toward the child-owned Slate skill package until Mother exposes first-class child skill discovery.

## Slate child convention

Slate is the preferred path for Patina build/refactor/fix work, but that rule belongs to the Slate child skill package, not directly in `AGENTS.md`.

Slate skill packages should expose:

- when they apply
- what Slate work item to create/reuse
- command examples
- lifecycle gates
- proof/closure expectations
- relationship to specs, Allium anchors, beliefs, sessions, and git

Slate and `patina spec` remain separate islands, but current version/archive behavior should stay spec-parity:

- specs version behavioral intent and release/archive flows
- Slate tracks implementation work transactions and proof trails
- for now Slate archive should mirror spec archive: terminal gate, clean tracked tree, archive-removal commit, and recovery tag such as `slate/<id>`
- git/session artifacts provide version boundaries for Slate work

## Reference repo research flow

When the user mentions Mother/HITL/source reference repos such as PI, OpenCode, Gemini, Claude-adjacent tooling, or other registered external repos:

1. Start with discovery tools before raw file reads:
   - `patina scry "<question>" --repo <repo-name>` when you already know the repo key.
   - `patina assay` / `patina assay --repo <repo-name>` for structured repo context when available.
2. If the cached source location is needed, locate it with partial repo-name lookup:
   - `patina repo list <partial-name>`
   - examples: `patina repo list gemini`, `patina repo list opencode`, `patina repo list pi`, `patina repo list flu`
3. Use `patina repo list <partial-name> --json` when an agent/tool needs stable repo metadata and cache paths.
4. Read focused files from the returned path only after the scry/assay pass or when the user explicitly asks for direct source inspection.

## Agent procedure

When asked to change Patina code:

1. Read this skill.
2. Check whether Mother has a first-class skill registry command yet.
3. If not, load the relevant child bridge skill directly, usually `.pi/skills/patina-slate-code/SKILL.md`.
4. Continue with that child skill's workflow.

Do not add child-specific long-form policy to `AGENTS.md`; keep it here or in child skill packages.
