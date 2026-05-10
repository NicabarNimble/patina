# Design: Mother View Maturation

## Why This Design

[[mother-view-composer]] already has Mother-owned shapes, requests, buffers, revisions, and observability gaps. Maturation should therefore extend the same backend substrate instead of becoming renderer state or agent-only notes.

The Allium target defines `ViewMaturationEvent` and `ObservabilityImprovementArtifact` as explicit entities. This slice turns those entities into durable Mother records and adds minimal derivation/pattern library records so non-shape view artifacts can actually carry maturity.

## Build Target

Implement `mvc7-maturation` from [[mother-view-composer]] and the Allium rules:

- `PromoteMatureViewArtifact`
- `CreateObservabilityImprovementFromMatureDerivation`

## Resolved Decisions

- Maturation is not buffer revision. It changes an artifact maturity value and records an immutable event; it does not replace shapes or buffers.
- Maturation is forward-only and one step at a time: `exploratory -> candidate -> stable -> promoted`.
- Shape maturation only applies to active shapes.
- Derivation/pattern records are persisted separately and linked to existing shapes.
- Observability-improvement artifacts are created only from derivation maturation to `stable` or `promoted`, and `work_item_created` remains false.

## Commits

1. [[commit-516ba249]] `spec: define mother view maturation slice` — describe the Allium-aligned maturation slice and exit criteria.
2. [[commit-5d003448]] `spec: promote mother-view-maturation to ready` — mark the slice ready for implementation.
3. [[commit-ca903bde]] `feat: mature mother view artifacts` — add maturation model, persistence, service, daemon, HTTP APIs, and tests.

## Direct Code Targets

- `mother/src/view_buffer/model.rs` — add `ViewDerivation`, `DisplayPattern`, `ViewMaturationEvent`, `ObservabilityImprovementArtifact`, target/origin/pattern enums, and request/outcome types.
- `mother/src/view_buffer/store.rs` — add schema tables and mapping helpers for derivations, display patterns, maturation events, and observability-improvement artifacts.
- `mother/src/view_buffer/service.rs` — add in-memory artifact maps and `mature_view_artifact` transition logic.
- `mother/src/state/mod.rs` — expose runtime-store persistence wrappers and tests.
- `mother/src/http_api.rs` — extend `ApiRuntime`/`ViewBufferApi` and route construction.
- `mother/src/http_api/view_buffer.rs` — add HTTP handlers for artifact upserts/lists and maturation.
- `mother/src/http_routes.rs` — add route table entries.
- `src/commands/mother/daemon/dispatch.rs` — implement daemon API methods backed by `MotherRuntimeStore`.
- `mother/src/http_api/tests/mod.rs` and `src/commands/mother/daemon/tests/mod.rs` — add API/daemon coverage.

## Verification Plan

```bash
cargo check -q
cargo test -q -p mother view_maturation
cargo test -q -p mother view_buffer
cargo test -q -p mother
patina spec check mother-view-maturation --json
allium check layer/allium/mother/mother-view-composer-target.allium
```

## Build Readiness

Ready to implement after spec/design commit and promotion to ready.

## Open Questions

- Should future matured derivations compile to typed WIT contracts or remain framed JSON until a later renderer/contract slice?
