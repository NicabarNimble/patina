# Design: refactor: SDK Contract Stabilization

## Why This Design

SDK stabilization is now isolated from broad Mother/Patina boundary work so we can:

- keep the architecture lane focused on runtime/core ownership,
- maintain a dedicated contract lane for third-party child builders,
- enforce compatibility gates without bloating greenfield docs.

## Build Target

- Public contract: `patina-sdk` remains the single external SDK brand.
- Stability policy:
  - `stable`: knowledge-child + tier support crates
  - `experimental`: pipeline
  - `internal/shim`: task, command, mother-child
- Vocabulary policy: child-first names are canonical; plugin names/macros are compatibility aliases.
- Manifest policy: SDK docs/examples align with `[needs].toys` + optional `[needs.scopes]`.

## Resolved Decisions

1. Keep legacy world lanes until parity-backed removal criteria pass.
2. Keep compatibility aliases to avoid breaking existing children during stabilization.
3. Track shim removal separately from vocabulary cleanup.
4. Keep SDK lane separate from greenfield architecture lane.

## Stability Classification (current)

| Surface | Classification | Evidence anchor |
| --- | --- | --- |
| `patina-sdk` umbrella | stable | `sdk/patina-sdk/src/lib.rs`, `sdk/patina-sdk/README.md` |
| `knowledge-child` world | stable | `sdk/patina-sdk/src/knowledge_child.rs`, first-party children |
| tier crates (`core/data/agent`) | stable-support | `sdk/patina-sdk-*/src/lib.rs` |
| `pipeline` world | experimental | `sdk/patina-sdk/src/pipeline.rs` |
| `task`, `command`, `mother-child` | internal shim | `sdk/patina-sdk/Cargo.toml`, world docs |

## Commits

1. `a867be73` — start SDK inventory/classification scope and tracking.
2. `d41eb4ff` — mark world stability classes and manifest contract wording.
3. `c78f4097` — child-first SDK vocabulary with compatibility aliases.
4. `09f4adc6` — record vocabulary migration evidence.
5. `f9c570ea` — record M5 evidence/gates and rollback boundaries.
6. `274671d9` — split/lock M6 architecture plan in greenfield lane (dependency context).

## Direct Code Targets

- `sdk/patina-sdk/src/lib.rs` — umbrella world/export policy and child-first docs.
- `sdk/patina-sdk/Cargo.toml` — feature classification comments and metadata wording.
- `sdk/patina-sdk/README.md` — world stability labels and migration framing.
- `sdk/patina-sdk/src/task.rs` — manifest vocabulary + child-first naming.
- `sdk/patina-sdk/src/command.rs` — manifest vocabulary + child-first naming.
- `sdk/patina-sdk/src/mother_child.rs` — shim lane wording + child-first naming.
- `sdk/patina-sdk/src/pipeline.rs` — experimental lane wording + child-first naming.
- `sdk/patina-sdk/src/knowledge_child.rs` — canonical trait naming + alias compatibility.
- `sdk/patina-sdk-core/src/lib.rs` — knowledge child alias compatibility.
- `resources/templates/plugin/*/lib.rs.tmpl` — generated child-first macros/traits.
- `src/child/scaffold.rs` — scaffold assertions and generated naming expectations.

## Verification Plan

1. Baseline compile:
   - `cargo check -q`
2. SDK world matrix compile:
   - `cargo check -q -p patina-sdk --features knowledge-child,toy-log,toy-state,toy-session,toy-lake,toy-checkpoint,toy-query,toy-emit,toy-measure,toy-github,toy-connector,toy-peer,toy-events,toy-belief,toy-task`
   - `cargo check -q -p patina-sdk --features pipeline`
   - `cargo check -q -p patina-sdk --features task`
   - `cargo check -q -p patina-sdk --features command`
   - `cargo check -q -p patina-sdk --features mother-child`
3. Scaffolding parity:
   - `cargo test -q -p patina-ai scaffold::tests::test_scaffold`
4. Spec criteria check:
   - `patina spec check sdk-contract-stabilization --json`

## Build Readiness

- Ready to promote once SPEC frontmatter criteria are linked to evidence bullets in this document.
- Shim-removal work remains gated behind first-party + compatibility matrix parity.

## Open Questions

1. Removal order for shim worlds (`task`, `command`, `mother-child`).
2. Timeline coupling with M6 `patina-protocol` extraction (when to move typed world contracts).
3. External builder compatibility window policy (how long aliases remain supported).
