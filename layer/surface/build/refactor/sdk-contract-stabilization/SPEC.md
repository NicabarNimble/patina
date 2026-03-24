---
type: refactor
id: sdk-contract-stabilization
status: draft
created: 2026-03-24
sessions:
  origin: 20260323-092255-893791000
related:
  - layer/surface/build/refactor/greenfield-mother-patina-rebuild/SPEC.md
  - layer/surface/build/refactor/greenfield-mother-patina-rebuild/DESIGN.md
  - sdk/patina-sdk/src/lib.rs
  - sdk/patina-sdk/README.md
  - sdk/patina-sdk/Cargo.toml
  - sdk/patina-sdk-core/src/lib.rs
  - sdk/patina-sdk-data/src/lib.rs
  - sdk/patina-sdk-agent/src/lib.rs
beliefs:
  - children-have-agency-toys-are-capabilities
  - core-verbs-standalone-mother-additive
  - agents-are-guests-mother-is-infrastructure
exit_criteria:
  - id: SDK1
    text: "SDK surfaces are classified as stable/experimental/internal with explicit rationale and docs alignment"
    checked: false
  - id: SDK2
    text: "Legacy compatibility worlds are marked as migration shims with explicit removal gates"
    checked: false
  - id: SDK3
    text: "Manifest vocabulary in SDK docs/examples uses child-native contract ([needs].toys + optional [needs.scopes])"
    checked: false
  - id: SDK4
    text: "Child-first API vocabulary is canonical while legacy aliases remain compatibility-only"
    checked: false
  - id: SDK5
    text: "Compatibility matrix for all supported world/feature lanes compiles and remains evidence-backed"
    checked: false
  - id: SDK6
    text: "Protocol/core extraction readiness for third-party builders is documented with rollback-safe migration slices"
    checked: false
---
# refactor: refactor: SDK Contract Stabilization

> Extract SDK stabilization/removal work from greenfield architecture lane into dedicated spec.

## Problem

SDK stabilization details were embedded inside the broader greenfield Mother/Patina architecture lane, causing two issues:

- architectural boundary work and SDK contract work were coupled in one oversized spec,
- third-party SDK contract evolution lacked a focused execution surface with explicit compatibility gates.

## Goal

Establish a dedicated SDK lane that hardens `patina-sdk` as a stable external contract for child authors while preserving migration safety.

This lane defines and verifies:

- stable vs experimental vs internal world surfaces,
- child-native vocabulary and manifest schema alignment,
- compatibility guarantees for first-party and third-party child builders,
- rollback-safe shim-removal sequencing.

## Status

- Split from `greenfield-mother-patina-rebuild` M5 scope.
- Initial groundwork already landed in code and docs; this spec now becomes the canonical continuation lane.

## Non-Goals

- No forced removal of legacy worlds without parity evidence.
- No redesign of Mother/Patina runtime ownership boundaries (handled in greenfield M6 lane).
- No SDK feature expansion unrelated to contract stabilization.

## Current State

- `patina-sdk` is child-first in public naming, with legacy plugin aliases preserved for compatibility.
- World lanes exist with mixed maturity:
  - canonical: `knowledge-child`
  - experimental: `pipeline`
  - migration shims: `task`, `command`, `mother-child`
- Tier crates (`patina-sdk-core`, `patina-sdk-data`, `patina-sdk-agent`) are present and used as support layers.
- Compatibility compile matrix has been run for all world lanes.

## Target State

- One clear external SDK contract surface (`patina-sdk`) with explicit stability classes.
- Legacy compatibility lanes are documented as shims with criteria-driven removal gates.
- SDK docs/examples consistently express child-native capability contract language.
- Third-party child builders can rely on stable lanes without depending on internals.
- M6 crate extraction (`patina-core`/`patina-protocol`) has a clean SDK-ready handoff contract.

## Solution

1. Normalize SDK contract language and stability tiers in code/docs.
2. Keep compatibility aliases, but mark them as transitional.
3. Lock compatibility evidence through compile/test matrix.
4. Define removal gates for shim worlds.
5. Prepare typed protocol/core migration touchpoints so SDK can consume future contracts without breaking consumers.

## Implementation Order

1. SDK-A: Inventory + classify all exposed worlds/features and mark stability tiers.
2. SDK-B: Mark shim worlds and migration policy in docs/manifests.
3. SDK-C: Validate schema vocabulary and examples against `[needs].toys` + `[needs.scopes]`.
4. SDK-D: Child-first API vocabulary pass with compatibility aliases retained.
5. SDK-E: Run and record compatibility compile matrix.
6. SDK-F: Define shim-removal playbook + rollback triggers.
7. SDK-G: Align with M6 protocol/core extraction requirements.

## Resolved Decisions

- Keep `patina-sdk` as the single public SDK brand.
- Keep legacy aliases/macros for compatibility until explicit removal gates are satisfied.
- Keep doctor as host-native runtime capability (not a forced WASM-child target).
- Split SDK lane from greenfield architecture lane to reduce spec bloat and preserve ownership clarity.

## Verification

- `cargo check -q`
- `cargo check -q -p patina-sdk --features knowledge-child,toy-log,toy-state,toy-session,toy-lake,toy-checkpoint,toy-query,toy-emit,toy-measure,toy-github,toy-connector,toy-peer,toy-events,toy-belief,toy-task`
- `cargo check -q -p patina-sdk --features pipeline`
- `cargo check -q -p patina-sdk --features task`
- `cargo check -q -p patina-sdk --features command`
- `cargo check -q -p patina-sdk --features mother-child`
- `cargo test -q -p patina-ai scaffold::tests::test_scaffold`

Spec status verification:

- `patina spec check sdk-contract-stabilization --json`

## Exit Criteria

See `exit_criteria` frontmatter (SDK1-SDK6). No promotion until each criterion has evidence anchors in DESIGN.

## Build Readiness

Ready for active execution once promoted and linked to M6 dependency notes in greenfield lane.
