---
type: feat
id: child-command-surface
status: draft
created: 2026-03-26
sessions:
  origin: 20260326-063618-515984000
beliefs:
  - "[[children-have-agency-toys-are-capabilities]]"
  - "[[four-roles-no-overlap]]"
  - "[[children-are-wasm]]"
blocked_by:
  - scaffold-world-retirement
  - note: "scaffold-world-retirement deletes the legacy `command` child kind and `wit/command/` directory. This spec's command-handler WIT contract lives in `wit/command-handler/` — a new package with fresh semantics, not the retired one-shot plugin world."
related:
  - src/main.rs
  - src/commands/spec/mod.rs
  - src/commands/lake.rs
  - src/commands/doctor.rs
  - src/commands/mother/daemon.rs
  - crates/patina-protocol/src/lib.rs
  - wit/command-handler/command-handler.wit
  - children/spec-manager/
  - children/lake-manager/
  - children/doctor/
  - sdk/patina-sdk/
exit_criteria:
  - id: ccs0-boundary-lock
    text: "Command ownership is explicit: CLI remains routing UX, Mother remains capability broker, child remains business-logic owner. No domain lifecycle logic remains in CLI or Mother for migrated surfaces."
    checked: false
  - id: ccs1-command-manifest-schema
    text: "child manifest schema supports command registration metadata for SDK children (command name, verbs/subcommands, arg schema reference, help metadata)."
    checked: false
  - id: ccs2-protocol-generalized
    text: "Patina protocol supports generic typed child command dispatch (not hardcoded per builtin command family) with stable request/response envelopes and tests."
    checked: false
  - id: ccs2b-command-wit-contract
    text: "A command-surface WIT contract exists in `wit/command-handler/` and is used as compile-time contract for command-capable children (not manifest-only registration)."
    checked: false
  - id: ccs3-cli-router-generalized
    text: "CLI can resolve and route child-provided command surfaces from registry/manifest metadata, including help rendering and JSON mode pass-through."
    checked: false
  - id: ccs4-spec-fully-child-owned
    text: "Spec lifecycle logic executes in WASM child path (spec-manager) with no core fallback business logic path for normal operation."
    checked: false
  - id: ccs5-third-party-proof
    text: "A third-party example child using only patina-sdk can register and serve a CLI command namespace end-to-end."
    checked: false
  - id: ccs6-policy-guardrails
    text: "Command namespace policy is enforced (reserved core names, collision handling, deterministic precedence, explicit alias behavior)."
    checked: false
  - id: ccs8-no-privilege-by-name
    text: "Command names alone never grant capability. Authorization remains toy/scope/connection grant-based; command routing cannot elevate privilege."
    checked: false
  - id: ccs9-no-shadow-fallbacks
    text: "Migrated command families have no silent core fallback logic path. If child routing is unavailable, CLI fails clearly instead of executing hidden core domain logic."
    checked: false
  - id: ccs10-manifest-needs-coherence
    text: "Command registration and capability grants are coherence-checked: command-declared required toys/scopes/connections must be present in `[needs]`, with failing lint/check on mismatch."
    checked: false
  - id: ccs11-atomic-refresh
    text: "Command surface updates are atomic on child update/reload: Mother swaps command registry snapshots in one step, avoiding mixed old/new route states."
    checked: false
  - id: ccs7-gates-green
    text: "Workspace checks and tests pass, including protocol/router/command-discovery tests and migrated command integration tests."
    checked: false
---
# feat: sdk-defined child CLI command surfaces

## Problem

Patina currently presents several command domains as child-routed (`spec`, `lake`, `doctor`) but the implementation is a hybrid: the CLI forwards to Mother, and Mother still calls core Rust lifecycle logic for some surfaces.

This weakens the child boundary in three ways:

1. Child ownership is not complete; core remains business-logic owner.
2. Third-party children cannot define first-class CLI command namespaces through SDK alone.
3. Protocol and router layers are specialized around hardcoded builtin command families and lack a WIT-backed command contract.

## Goal

Establish a fully child-owned command model where SDK children can define CLI command surfaces declaratively, and the CLI routes those commands generically through Mother with strict policy and capability mediation.

## Status

Draft. Problem and target model are clear. Implementation has not started.

## Non-Goals

- Replacing core bootstrap/admin commands (`init`, `mother`, runtime bootstrap).
- Removing explicit safety confirmations for destructive lifecycle actions.
- Shipping dynamic untrusted remote command packs in this phase.
- Solving every command-domain migration at once; this work proves one canonical migration path first.

## Target Shape

1. Child manifests declare command surfaces.
2. SDK offers typed helpers/macros for command request/response handling.
3. `wit/command-handler/` defines the compile-time command contract that command-capable children implement.
4. CLI resolves command ownership from registry/manifest metadata.
5. Mother routes generic command dispatch envelopes.
6. Mother enforces command metadata validation/coherence and atomically refreshes command routes on child update.
7. Child executes domain logic with granted toys/scopes.
8. CLI prints child text/json/help consistently.

The `spec` command family is the canonical migration target for this spec.

## Solution

### Phase A - Boundary lock and schema

- Freeze ownership contract (CLI UX vs Mother mediation vs child logic).
- Extend manifest schema for `[provides.commands]` registration.
- Define command metadata fields (name, verbs, argument schema path, help text, aliases).
- Define command metadata `requires` fields used for coherence checks against `[needs]`.
- Freeze validation ownership: Mother validates routing/coherence, CLI renders diagnostics, child validates domain payload semantics.

### Phase A1 - WIT contract lock

- Define/refresh command WIT contract in `wit/command-handler/` for command-capable children.
- Ensure contract follows current per-interface import conventions (no legacy `patina:host/*` dependency lane).
- Wire generated bindings for SDK command helpers.

### Phase B - Protocol and router generalization

- Replace hardcoded command-family dispatch with generic child-command envelope in protocol.
- Update Mother daemon dispatch and CLI router to use generic command targets.
- Keep compatibility shims for existing builtin routes during migration.
- Add explicit startup/check failure for command metadata coherence mismatches.

### Phase C - SDK command API

- Add SDK command handler surface (typed request/response, JSON/text mode, optional confirmation hint).
- Add registration macro support for command-capable children.

### Phase D - Spec migration proof

- Move spec lifecycle execution ownership into `children/spec-manager`.
- Keep CLI command syntax (`patina spec ...`) stable while changing backend ownership.

### Phase E - Third-party proof + policy

- Add one example third-party style child command namespace via SDK-only path.
- Enforce reserved command names and collision resolution policy.
- Prove atomic command-surface refresh behavior on child update/reload.

### Guardrails (hard invariants)

1. No privilege-by-command-name
   - Routing only selects handler; it never authorizes behavior.
   - Child authorization is validated against granted toys/scopes/connections.
   - Tests include a denied-path command where route succeeds but capability check rejects.

2. Reserved namespace and collision policy
   - Core namespaces are reserved (`init`, `mother`, and other bootstrap/admin surfaces).
   - Non-reserved collisions resolve deterministically with explicit aliasing.
   - Ambiguous ownership without explicit policy fails at startup/check time.

3. No silent core fallback
   - For migrated command families, remove hidden core business-logic fallback execution.
   - On child unavailability, return explicit routing error with remediation text.
   - Regression tests assert failure behavior when Mother/child is unavailable.

4. Manifest/needs coherence
   - Command registration includes explicit required capabilities.
   - Mother check fails if `[provides.commands].requires` exceeds `[needs]` grants.
   - Regression tests cover mismatch failures and valid-pass cases.

5. Atomic refresh semantics
   - On child update/reload, command registry updates as a single snapshot swap.
   - No interleaved mixed route state between versions is allowed.
   - Regression tests cover command rename/add/remove across update.

### Phase F - cleanup

- Remove now-dead hardcoded core dispatch paths for migrated command families.

## Implementation Order

1. Boundary lock + validation ownership decisions.
2. WIT command contract lock in `wit/command-handler/`.
3. Manifest command-schema + parser + needs-coherence checks.
4. Protocol generic command envelope + transport tests.
5. CLI command resolver/router path.
6. SDK command handler API and macro ergonomics.
7. Spec-manager migration to full child ownership.
8. Third-party example command namespace proof + atomic refresh proof.
9. Remove dead legacy hardcoded dispatch and compatibility branches.

## Resolved Decisions

- Keep `patina <namespace> ...` as the user-facing command syntax.
- Command ownership is capability-based and child-declared, not core-hardcoded.
- Core commands remain reserved and non-overridable.
- Use deterministic collision resolution with explicit aliases instead of implicit overrides.
- Mother is the authoritative validator for command registry/coherence; CLI does not duplicate validation logic.
- Command registry refresh on child update/reload is atomic snapshot-swap behavior.

## Verification

```bash
cargo check --workspace -q
cargo test -q --workspace
patina spec check child-command-surface --json
```

Command-surface verification (when implementation lands):

```bash
patina <child-command> --help
patina <child-command> <verb> --json
```

## Exit Criteria

See frontmatter `exit_criteria` (`ccs0`-`ccs11`).

## Build Readiness

Ready for implementation planning and phased execution.
