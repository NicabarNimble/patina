---
type: refactor
id: cross-interface-mother-skills
status: draft
created: 2026-04-11
sessions:
  origin: 20260410-220235-028265000
related:
- src/mother/skills/mod.rs
- src/interface/runtime/templates.rs
- src/interface/internal/bundle.rs
- tests/mother_skills_registry.rs
- /Users/nicabar/.pi/agent/skills/
beliefs:
- '[[stale-context-is-hostile-context]]'
- '[[core-verbs-standalone-mother-additive]]'
exit_criteria:
- id: cims1-shared-skill-model
  text: Mother skill authority models shared skill packs once and maps them into interface-specific projection files for claude, gemini, opencode, and pi.
  checked: false
- id: cims2-registry-declaration
  text: Interface registry skills.include can declare shared pack names (for example patina-operator) without duplicating per-interface variants.
  checked: false
- id: cims3-projection-contract
  text: Projection emits valid interface-native artifacts for each HITL interface and preserves existing wrapper command behavior (`session-start/update/note/end`, spec flows).
  checked: false
- id: cims4-fail-closed-integrity
  text: Unknown or partially-mapped shared skills fail closed with actionable errors; no silent skips.
  checked: false
- id: cims5-pi-bridge-import
  text: A bridge path exists to ingest/normalize selected skills from `~/.pi/agent/skills/` into Mother-owned skill packs (one-time sync or explicit import command), without making runtime projection depend on external Pi paths.
  checked: false
- id: cims6-test-coverage
  text: Integration tests prove one shared skill pack projects correctly across all HITL interfaces and that fail-closed behavior triggers when mappings are missing.
  checked: false
- id: cims7-compatibility
  text: Existing built-in skills (session wrappers, spec, epistemic-beliefs) continue to project byte-compatibly unless intentionally changed by this spec.
  checked: false
- id: cims8-compile-proof
  text: cargo check --workspace -q and targeted registry/skills tests pass.
  checked: false
---
# refactor: Cross-interface Mother skill packs

> Make Mother skill authority support reusable/shared Patina skill packs that project consistently into all HITL interfaces (claude, gemini, opencode, pi), including a bridge path from existing Pi skill trees.

## Problem

Patina now has Mother-owned skill authority, but skill content is still represented as per-interface branches in `mother::skills::skill_content(interface, skill)`. Existing Pi harness skills (`~/.pi/agent/skills/*`) are not directly consumable by Patina interface projection. This creates duplication, drift risk, and inconsistent skill availability between runtimes.

## Goal

Create a canonical shared-skill-pack model under Mother authority that can:
1. Be declared once in registry metadata.
2. Project correctly into each HITL interface format.
3. Optionally import/bridge selected Pi skills into Mother-owned packs.

## Status

Draft.

## Non-Goals

- Replacing Mother authority with direct runtime reads from `~/.pi/agent/skills`.
- Unifying all interface command syntax into one file format.
- Expanding taxonomy beyond HITL in this spec.
- Refactoring session lifecycle semantics.

## Current State

- Registry declares `skills.include` per interface.
- Projection is fail-closed for unknown skills.
- Skill source authority is `src/mother/skills/mod.rs`.
- Shared concepts (session/spec/belief workflows) are duplicated across interface-specific content blocks.

## Target State

- Mother stores shared skill packs as canonical concepts + interface adapters.
- Registry can reference shared pack IDs directly.
- Projection renders interface-native artifacts from shared pack definitions.
- Selected Pi skills can be imported into Mother-owned packs through an explicit bridge step.

## Solution

1. Introduce a shared-skill-pack layer in Mother skill authority (concept + per-interface projection mapping).
2. Keep registry `skills.include` as the declaration surface, but resolve entries through shared pack IDs.
3. Add an explicit bridge/import flow for Pi skills (`~/.pi/agent/skills/*`) into Mother-managed packs.
4. Preserve fail-closed projection and current wrappers.
5. Add integration tests spanning all HITL interfaces and bridge error cases.

## Implementation Order

1. Shared pack model and resolver in `src/mother/skills/mod.rs`.
2. Projection updates in `src/interface/runtime/templates.rs`.
3. Registry metadata compatibility in `src/interface/internal/bundle.rs`.
4. Bridge/import implementation.
5. Test hardening (`tests/mother_skills_registry.rs` + new cross-interface projection tests).

## Resolved Decisions

- Runtime projection remains Mother-authoritative.
- External Pi skill trees are bridge inputs, not runtime dependencies.
- Fail-closed behavior remains mandatory.

## Verification

```bash
cargo check --workspace -q
cargo test --test mother_skills_registry
cargo test --test registry_codex_fixture
cargo test --test registry_pi_fixture
```

## Exit Criteria

Frontmatter criteria `cims1..cims8` are the source of truth.

## Build Readiness

Medium. The Mother authority seam already exists and is tested; primary risk is designing the shared-pack mapping without breaking interface-specific projection expectations.
