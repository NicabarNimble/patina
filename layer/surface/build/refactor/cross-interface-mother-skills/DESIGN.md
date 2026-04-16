# Design: refactor: Cross-interface Mother skill packs

## Why This Design

Mother already owns interface skill projection authority, but content is still
encoded as interface-specific branches. That duplicates the same intent
(session/spec/belief workflows) across claude/gemini/opencode/pi and prevents
direct reuse of existing Pi skills without copy/paste.

This design introduces a shared pack model under Mother authority, then renders
pack content per interface format at projection time.

## Build Target

Deliver a no-contract-break refactor that adds:

1. **Shared skill packs** (single logical skill, multi-interface renderers)
2. **Registry declaration by shared pack id** via `skills.include`
3. **Fail-closed projection** for unknown or incomplete mappings
4. **Pi bridge import** into Mother-owned storage (no runtime dependency on
   `~/.pi/agent/skills` during projection)

Compatibility target:
- existing wrappers (`session-start/update/note/end`) remain unchanged
- existing built-ins (`spec`, `epistemic-beliefs`) keep current behavior unless
  intentionally evolved in this spec

## Architecture Sketch

### A) Shared pack model in Mother skills authority

Introduce a logical layer above `skill_content(interface, skill)`:

- `SharedSkillPackId` (string-backed ids, e.g. `patina-operator`)
- `SharedSkillPack` (canonical pack metadata + per-interface render plan)
- resolver: `(interface, pack_id) -> SkillContent`

This keeps one conceptual pack id while still projecting native file layouts
(`.md`, `.toml`, script paths) per interface.

### B) Registry include semantics

`skills.include` remains the interface declaration surface, but entries are
resolved as shared pack ids first.

Compatibility mode:
- existing ids (`session-start`, `spec`, `epistemic-beliefs`) continue to work
- shared pack ids can compose these primitives (or supersede them)

### C) Projection pipeline

`templates::copy_to_project()` and seed install paths resolve each include entry
through Mother skills authority and project files exactly as today.

Hard rule: resolution remains **fail-closed**:
- unknown pack id -> error
- missing interface mapping for known pack -> error
- no silent skip/fallback

### D) Pi bridge import (ingest then own)

Add an explicit bridge flow that reads selected skill trees from:
`~/.pi/agent/skills/*`

and normalizes/copies them into Mother-owned storage (proposed:
`~/.patina/skills/imported/<pack-id>/...`).

Projection uses only Mother-owned content after import. External Pi paths are
input-only, never runtime projection dependencies.

## Resolved Decisions

- Mother remains runtime authority for projected skills.
- Shared pack ids are canonical at registry declaration time.
- Runtime projection must stay fail-closed.
- Pi skill trees are bridge inputs, not direct runtime sources.
- Existing wrapper workflow stays additive and unchanged.

## Commits
1. `refactor(skills): add shared-pack resolver model under mother::skills` —
   introduce pack ids and interface-aware rendering API.
2. `refactor(templates): resolve skills.include entries through shared packs` —
   wire projection/install paths to new resolver.
3. `refactor(registry): allow shared pack ids in builtin skills.include` —
   declare at least one shared pack (`patina-operator`) across all HITL
   interfaces while preserving compatibility entries.
4. `feat(skills): add patina-operator shared pack mappings` — initial
   cross-interface pack proving markdown/toml/script path adaptations.
5. `feat(skills): add explicit pi-skill bridge import` — ingest selected packs
   from `~/.pi/agent/skills` into Mother-owned storage.
6. `test(skills): cross-interface projection + fail-closed coverage` — prove
   successful projection across all HITL interfaces and deterministic failures.

## Direct Code Targets

- `src/mother/skills/mod.rs`
  - shared pack types + resolver
  - known-skill catalog update
  - bridge-import loader hooks for Mother-owned imported packs
- `src/interface/runtime/templates.rs`
  - include resolution path in `copy_to_project`
  - seed install path resolution in `install_interface_templates`
- `src/interface/internal/bundle.rs`
  - builtin registry `skills.include` declarations for shared packs
- `src/paths.rs`
  - Mother-owned imported-skill storage path helpers (if new storage added)
- `src/commands/interface/manage.rs` (or equivalent command surface)
  - explicit bridge import entry point
- `tests/mother_skills_registry.rs`
  - shared-pack projection and fail-closed tests
- `tests/registry_pi_fixture.rs` / new projection tests
  - prove cross-interface behavior with fixture-backed registry metadata

## Verification Plan

1. `cargo check --workspace -q`
2. `cargo test --test mother_skills_registry`
3. `cargo test --test registry_pi_fixture`
4. Add/execute shared-pack projection tests for all HITL interfaces
5. Manual smoke:
   - `patina ai setup`
   - verify projected shared pack artifacts exist in `.claude/.gemini/.opencode/.pi`
6. Bridge smoke:
   - run import flow from `~/.pi/agent/skills`
   - verify imported pack lands under Mother-owned storage
   - re-run projection without accessing Pi path

## Build Readiness

Medium. The authority seam is already in place and fail-closed behavior exists.
Primary risk is introducing shared-pack indirection without changing projected
artifact semantics.

## Open Questions

1. Should imported pack storage live under `~/.patina/skills/` or
   `~/.patina/interfaces/skills/`?
2. Should `patina-operator` be always projected to all HITL interfaces or remain
   explicit in `skills.include`?
3. Do we keep legacy per-skill ids permanently, or stage deprecation once shared
   packs are proven stable?
