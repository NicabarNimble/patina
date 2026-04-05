# Design: Interface Vocabulary Hard Cut

## Why This Design

This lane is a semantic hard-cut, not a cosmetic rename pass. The architecture
and project language already treat `interface` as canonical, but active code,
CLI surfaces, and storage still carry legacy vocabulary. That mismatch creates
policy drift and increases operational ambiguity during the upcoming interface
redesign work.

The design intentionally migrates vocabulary before behavior redesign so the
next architecture phase starts from one stable naming model.

## Build Target

Achieve zero active usage of the legacy adapter term across runtime code,
CLI/help, payload contracts, and persisted schema, while preserving migration
safety for existing project/global state.

## Design Principles

- Interface is the only active term in code and user-facing runtime surfaces.
- Historical references may remain only in archived artifacts.
- Compatibility is migration-time only, not permanent dual vocabulary.
- Storage/schema migration is part of this lane (not deferred).
- Proof is repository-scan + compile/test + command-flow verification.

## Baseline Inventory (From Current Code)

1. CLI and command modules
   - active module and help text still use legacy command family naming
   - routing and aliases still wired from `src/main.rs`

2. Session/API payload contracts
   - result payload fields and in-memory structs still include legacy names
   - active-session lookup/filter paths still key off `adapter_name`

3. Interface templates/wrappers
   - runtime template/wrapper code still emits legacy vocabulary and flags

4. Config model
   - serde aliases still permit old key names in active runtime parsing

5. Mother/session persistence
   - schema currently stores `adapter_name` in session rows

## Execution Slices

1. Identifier and module cutover
   - rename internal identifiers to `interface*`
   - cut `src/commands/adapter.rs` out of active routing
   - establish `src/commands/interface.rs` as canonical command path

2. CLI/help and command grammar hard cut
   - remove legacy command and flag aliases from active clap surfaces
   - ensure help output is interface-only

3. Payload and API contract migration
   - rename public JSON/result keys and internal fields to interface naming
   - migrate all direct callsites in `ai`, `session`, `launch`, `interface`

4. Config migration
   - implement one-time migration from legacy keys to canonical keys
   - conflict rule: if both legacy and new keys exist with different values,
     migration fails with an actionable error (no silent override); matching
     values are silently deduplicated to the new key
   - remove runtime serde aliases after migration tests are green

5. Storage/schema migration
   - migrate persisted columns/fields from `adapter_name` to `interface_name`
   - provide backfill/compat read path only during migration window
   - bridge removal trigger: bridge code is removed in the same PR/commit
     sequence that checks off ivh7 and ivh9 — not deferred to a later lane

6. Template and generated resource cutover
   - update wrapper scripts/prompts/templates to interface-only terms
   - ensure generated files from setup/init/session paths contain no legacy term

7. Proof and scan gate
   - run compile/tests
   - run zero-match scan against `src/`, `mother/`, `resources/`, `AGENTS.md`
   - excluded historical trees: `layer/sessions/`, `layer/surface/build/`,
     `layer/core/`
   - any match in scan targets is a blocker; matches in excluded trees are
     acceptable as historical artifacts

## Contract and Compatibility Policy

- No permanent dual naming (`adapter` + `interface`) in active runtime paths.
- Any temporary bridge must be explicitly scoped to migration and removed before
  checking off exit criteria.
- The end state must not break users. Intermediate commits may be broken during
  the refactor, but the completed lane must leave all user-facing flows working:
  old configs migrated automatically, old schema backfilled, CLI commands
  functional under the new names. If something breaks during transition that is
  acceptable, but nothing stays broken at lane close.

## Risk Controls

1. CLI break risk
   - Mitigation: stage command cutover with explicit migration messaging in
     release notes and tests for new command paths.

2. Config ingestion break risk
   - Mitigation: one-time migration utility + fixture tests with legacy config
     samples.

3. Schema migration risk
   - Mitigation: transactional migration with backup + startup sanity checks +
     idempotent migration guard.

4. Hidden vocabulary drift risk
   - Mitigation: final `rg` gate across active source/resource trees.

## Verification

Canonical proof commands are defined in SPEC.md § Proof Commands. This section
defers to that list to avoid divergence.

## Release Boundary

Single-release hard cut. Migration and new vocabulary ship together. There is
no transitional release with dual command support. Migration runs on first
startup after upgrade; from that point forward, only interface vocabulary is
active.

## Done Criteria (Design-Level)

- all spec criteria `ivh1`..`ivh9` are checkable with direct evidence
- migration story is complete (CLI, config, schema)
- no ambiguous compatibility language remains in active docs/code
