---
type: refactor
id: interface-skill-ownership-split
status: active
created: 2026-04-21
related:
  - src/interface/internal/bundle.rs
  - src/interface/runtime/templates.rs
  - src/interface/internal/bootstrap.rs
  - src/mother/skills/mod.rs
  - src/interface/internal/surface.rs
  - src/interface/runtime/mod.rs
  - resources/claude/skills/epistemic-beliefs/
  - resources/opencode/epistemic-beliefs.md
  - resources/gemini/epistemic-beliefs.toml
references:
  - layer/core/values/patina-identity.md
  - layer/core/values/spec-driven-design.md
  - layer/core/values/session-capture.md
  - layer/core/values/safety-boundaries.md
beliefs:
  - '[[stale-context-is-hostile-context]]'
  - '[[durability-lives-outside-interface-process]]'
  - '[[core-verbs-standalone-mother-additive]]'
exit_criteria:
  - id: isos1-ownership-contract
    text: "Skill ownership is explicit: global skills are interface-native; Patina workflow skills are project-owned."
    checked: true
  - id: isos2-projection-precedence
    text: "Projection merge order is deterministic and documented: global interface baseline -> project Patina skills -> project override files (last wins)."
    checked: true
  - id: isos3-pi-belief-parity
    text: "PI epistemic-beliefs behavior is proactive and parity-aligned with Claude/OpenCode intent (asks user to capture beliefs when strong design signals appear)."
    checked: true
  - id: isos4-patina-skills-project-root
    text: "Patina-owned skills live in project state and are projected from project-owned sources, not only global embedded bundle state."
    checked: true
  - id: isos5-refresh-upgrade-flow
    text: "`patina ai refresh` (or equivalent setup refresh path) updates managed project Patina skills, preserves user edits via managed blocks/backups, and records interface ops."
    checked: true
  - id: isos6-hitl-compat-all
    text: "Claude/OpenCode/Gemini/PI all remain launchable HITL interfaces with no regression in session/spec command wrappers."
    checked: true
  - id: isos7-managed-surface-safety
    text: "Managed path constraints and backup/takeover behavior remain fail-closed and auditable for skill projection changes."
    checked: true
  - id: isos8-tests
    text: "Automated tests cover ownership split, projection precedence, PI belief prompt parity, and refresh idempotency."
    checked: true
---
# refactor: Interface Skill Ownership Split (PI-first HITL polish)

> Make skill ownership unambiguous: global for pure interface behavior, project-owned for Patina workflows. Start with PI behavior parity, then apply the same contract across all HITL interfaces.

## Problem

Current skill projection mixes two categories:

1. Interface-native guidance (global/runtime behavior), and
2. Patina workflow guidance (session/spec/epistemic-beliefs).

Because ownership is blurred, behavior drifts by interface. In practice, Claude tends to proactively suggest belief capture while PI/OpenCode/Gemini are weaker and more procedural.

This creates an inconsistent HITL experience and weakens trust in project truth workflows.

## Goal

Define and enforce a simple ownership model:

- **Global skills**: pure HITL interface behavior, shared at user/runtime level.
- **Project skills**: Patina workflow behavior (session/spec/belief), owned by project and refreshed through project update flow.

Primary implementation focus is **PI HITL behavior**, while touching all required surfaces to keep cross-interface consistency.

## Non-Goals

- Implementing shared/cross-project belief federation in this spec.
- Redesigning Mother belief storage model in this spec.
- Replacing interface registry architecture wholesale.

## Value Anchors (layer/core)

- **Patina Identity**: keep core focused on protocol workflows (session/spec/belief) while treating interface-native behavior as separate global concern.
- **Spec-Driven Design**: ownership split is explicit and test-locked; no implicit behavior drift.
- **Session Capture**: project-owned Patina skills must remain low-friction and deterministic across interfaces.
- **Safety Boundaries**: projection updates stay inside managed project/interface paths and preserve fail-closed backup/takeover behavior.

## Ownership Contract (normative)

### A) Global interface skills

Global skills are allowed only for interface/runtime-native concerns:
- tool detection/launch notes,
- interface UX affordances,
- non-Patina vendor specifics.

These live in Mother-managed interface package state.

### B) Project Patina skills

Patina workflow skills are project-owned:
- session workflow commands,
- spec workflow commands,
- epistemic belief capture workflow,
- Patina policy language for this repo/project.

These must project from project-managed sources and update with project refresh flow.

### C) Projection precedence

Deterministic merge order:
1. global interface baseline,
2. project Patina skills,
3. project local overrides.

Last writer wins. No ambiguous duplicate ownership.

## PI-first behavior requirement

PI must receive proactive epistemic behavior parity with Claude intent:
- detect design principles / repeated decisions,
- ask user confirmation before creation,
- use deterministic `create-belief.sh` flow,
- recommend `patina scrape` after capture.

This is behavior parity at policy level, not byte-for-byte template identity.

## Implementation Order

1. **Ownership split wiring**
   - Separate global vs project Patina skill catalogs in projection path.
2. **PI parity first**
   - Upgrade PI `epistemic-beliefs` prompt/command behavior to proactive parity.
3. **Cross-interface alignment**
   - Ensure Claude/OpenCode/Gemini map to same ownership model and merge precedence.
4. **Refresh/update mechanics**
   - Ensure project refresh updates Patina-owned skill payloads with managed backups.
5. **Tests + docs**
   - Lock behavior with integration tests and update operator docs.

## Direct Code Targets

- `src/mother/skills/mod.rs`
- `src/interface/runtime/templates.rs`
- `src/interface/internal/bootstrap.rs`
- `src/interface/internal/bundle.rs`
- `src/interface/internal/surface.rs`
- `resources/opencode/epistemic-beliefs.md`
- `resources/gemini/epistemic-beliefs.toml`
- `resources/claude/skills/epistemic-beliefs/SKILL.md` (parity reference)

## Verification

```bash
patina spec check interface-skill-ownership-split --json
cargo check --workspace -q
cargo test -q --workspace
```

Required scenario checks:
- PI suggests belief capture on strong design-language patterns.
- PI/Claude/OpenCode/Gemini all expose working session/spec/belief wrappers after projection refresh.
- Re-running refresh is idempotent and preserves user unmanaged content with backups.

## Exit Criteria

Frontmatter `isos1..isos8` are source of truth.

## Build Readiness

High: most required plumbing exists; this is primarily ownership boundary enforcement + prompt/projection alignment.
