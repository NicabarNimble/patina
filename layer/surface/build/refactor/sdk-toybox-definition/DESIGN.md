# Design: Define the toybox framework and enumerate Mother's canonical toy surface

## Why This Design

This lane tightens the SDK platform contract after `sdk-contract-stabilization` closure. The previous spec locked lane stability and compatibility policy; this spec now defines the canonical toybox itself so future toy additions/merges follow a principled, bounded process instead of organic growth.

## Build Target

- A canonical toybox definition with explicit litmus test and anti-goals.
- A deterministic inventory that maps every toy WIT interface to host boundary/resource ownership and SDK tier exposure.
- Explicit overlap-cluster decisions (`keep-separate`, `merge`, `defer`) with rationale.
- Rollback-safe migration gates for any future toy merge/removal sequence.

Execution policy:

1. Contract lock first (definition + canonical toybox + cluster decisions).
2. Cleanup second (toy merge/removal only after contract lock and gate verification).

## Deterministic Inventory (Current)

Inventory sources used in this pass:

- `wit/toys/*.wit` (canonical toy interface list)
- `src/child/toy_host/*.rs` (direct child-host adapter modules)
- `sdk/patina-sdk/src/toys.rs` + `sdk/patina-sdk/src/lib.rs` (SDK toy abstractions and lane/tier exposure)

Current counts:

- WIT toy interfaces: 22
- Direct child-host adapter modules: 8 (`connector`, `events`, `github`, `http`, `ingress`, `lake`, `query`, `session`)

### Toy Mapping Snapshot (A1 Inventory)

| Toy Interface | Host Resource Boundary | Direct Host Adapter Module | SDK Exposure / Tier | Status | Why Child Cannot Do This Itself |
| --- | --- | --- | --- | --- | --- |
| `belief` | epistemic query/mutate boundary | _none observed_ | `BeliefToy` (patina-sdk local) | scaffold/indirect | belief graph state and policy live in host-controlled layer stores |
| `checkpoint` | durable offset/checkpoint persistence | _none observed_ | `CheckpointToy` (data tier) | scaffold/indirect | durable checkpoint writes require host storage authority |
| `connector` | external connector binding/sync authority | `src/child/toy_host/connector.rs` | `ConnectorCatalog`/`ConnectorBackend` (data tier) | implemented | binding metadata and sync authority are host-owned with policy checks |
| `emit` | fact emission into eventlog | _none observed_ | `EmitToy` (agent tier) | scaffold/indirect | eventlog insert path and schema guardrails are host-side authority |
| `events` | ordered stream pull/ack | `src/child/toy_host/events.rs` | `EventToy` (patina-sdk local) | implemented | stream offsets/acks must be mediated to preserve ordering/durability |
| `git` | git operation mediation | _none observed_ | `GitToy` (core tier, feature-gated) | scaffold/indirect | repo mutation and tag operations require host binary/fs access |
| `github` | provider-specific API mediation | `src/child/toy_host/github.rs` | `GithubToy` (data tier) | implemented | credential injection + domain/policy mediation are host responsibilities |
| `graph` | graph query/mutate boundary | _none observed_ | `GraphToy` (patina-sdk local) | scaffold/indirect | graph indexes and mutation integrity are host-owned resources |
| `http` | constrained network egress | `src/child/toy_host/http.rs` | `FetchToy` (patina-sdk local) | implemented | WASM guest has no ambient host network + credential/domain policy |
| `ingress` | source endpoint fetch mediation | `src/child/toy_host/ingress.rs` | `IngressCatalog`/`IngressToy` (patina-sdk local) | implemented | source catalogs and source-level grants are host policy data |
| `lake` | data warehouse + cursor boundary | `src/child/toy_host/lake.rs` | `LakeCatalog`/`LakeToy` (data tier) | implemented | DuckDB/lake access and cursor persistence are host-owned storage paths |
| `layer` | project layer read boundary | _none observed_ | world module helpers in world-specific SDK modules | scaffold | project files/config/environment are host filesystem state |
| `layer-fs` | scoped layer filesystem access | _none observed_ | `LayerFsToy` (core tier, feature-gated) | scaffold/indirect | path normalization and escape prevention require host FS mediation |
| `log` | structured log sink | _none observed_ | `LogToy` (core tier) | scaffold/indirect | host controls sink format/provenance/timestamps |
| `measure` | metric/event measurement path | _none observed_ | `MeasureToy` (data tier) | scaffold/indirect | metric ingestion and event attribution are host-side concerns |
| `peer` | peer event/messaging boundary | _none observed_ | `PeerToy` (core tier, feature-gated) | scaffold/indirect | peer transport/session ownership lives on host network runtime |
| `query` | indexed knowledge query engines | `src/child/toy_host/query.rs` | `QueryToy` (agent tier) | implemented | scry/context/assay indexes and ACL policy are host-managed |
| `schema` | schema contract/registry boundary | _none observed_ | no direct toy abstraction in `sdk/patina-sdk/src/toys.rs` | scaffold | schema files and projection metadata are host-managed contract artifacts |
| `session` | session artifact/tag lifecycle | `src/child/toy_host/session.rs` | `SessionToy` (agent tier) | implemented | session artifacts and git tagging are host-owned state transitions |
| `state` | child-local persistent key/value state | _none observed_ | `StateToy` (core tier) | scaffold/indirect | persistent state durability and namespacing are host storage concerns |
| `task` | task queue enqueue boundary | _none observed_ | `TaskToy` (patina-sdk local) | scaffold/indirect | task scheduling and leasing are host runtime responsibilities |
| `types` | shared cross-toy type definitions | n/a (support interface) | support types only | support | type declarations are compile-time contracts, not direct host capability |

A1 outcome:

- Inventory parity satisfied for this pass: all 22 `wit/toys/*.wit` interfaces are mapped with boundary/resource ownership, SDK exposure, implementation status, and rationale.

## A2 Definition Lock (Completed)

Toy definition locked into SDK-facing docs:

- `sdk/patina-sdk/README.md` now defines toys as Mother-owned sandbox boundary openings.
- `sdk/patina-sdk/src/lib.rs` now carries the toy contract policy in crate docs.

Locked properties:

1. Toys are Mother-defined and granted through manifest declarations.
2. Scopes shape authority and do not create new toy kinds.
3. Domain/provider logic belongs in children, not toys.
4. Litmus test for toy inclusion: "Why can't the child do this itself from pure compute?"
5. Anti-goals documented: no convenience-wrapper sprawl, no plugin-defined toy kinds, no bypass of scoped host policy.

## Overlap Clusters and Decision Template

Clusters to resolve in this spec:

1. Network: `http`, `github`, `connector`, `ingress`
2. Event emission/streaming: `events`, `emit`, `measure`
3. Layer/filesystem: `layer`, `layer-fs`
4. Knowledge structures: `query`, `belief`, `graph`
5. Persistence family: `lake`, `state`, `checkpoint`, `schema`

For each cluster, this spec must record one outcome and why:

- `keep-separate`: distinct boundary/resource ownership proven
- `merge`: shared boundary with migration sequence and rollback
- `defer`: unresolved dependency with explicit revisit trigger

## A3 Cluster Decisions (Completed)

| Cluster | Decision | Why |
| --- | --- | --- |
| Network (`http`, `github`, `connector`, `ingress`) | keep-separate | Distinct host authorities: generic egress policy (`http`), provider semantics + credential mediation (`github`), connector binding/sync authority (`connector`), source catalog mediation (`ingress`). Merging now would blur trust boundaries. |
| Event emission/streaming (`events`, `emit`, `measure`) | keep-separate | `events` is consume/ack stream control; `emit` is fact write path; `measure` is constrained metric reporting semantics. Same transport family, different authority shapes and validation obligations. |
| Layer/filesystem (`layer`, `layer-fs`) | defer | Current implementation split is uneven (`layer-fs` in SDK core abstractions, `layer` mostly world-bound helpers). Need A4 canonical table and adapter evidence before deciding merge vs keep. Revisit trigger: completion of canonical toybox table with per-toy direction/scope fields. |
| Knowledge structures (`query`, `belief`, `graph`) | keep-separate | `query` is read/query-engine boundary; `belief` and `graph` include mutation semantics and different invariants. Keeping separate preserves intent-specific policy and auditability. |
| Persistence family (`lake`, `state`, `checkpoint`, `schema`) | defer | Overlap exists but boundaries differ (warehouse access, local child state, stream checkpoint durability, schema contract management). Revisit trigger: migration sketch proving no loss of isolation or rollback safety. |

A3 outcome:

- Every required overlap cluster now has an explicit `keep-separate` or `defer` decision with rationale.

## Resolved Decisions

1. Toys are platform-defined boundary openings owned by Mother; children do not define new toys.
2. Toy addition/merge/removal requires syscall-level rigor and explicit parity/rollback evidence.
3. No toy merge/removal code execution starts until this spec records deterministic inventory and cluster decisions.
4. This lane extends (does not replace) `sdk-contract-stabilization`; stability policy remains authoritative there.

## Commits

1. _Pending execution commits — will be populated as inventory/consolidation artifacts land._

## Direct Code Targets

- `layer/surface/build/refactor/sdk-toybox-definition/SPEC.md`
- `layer/surface/build/refactor/sdk-toybox-definition/DESIGN.md`
- `sdk/patina-sdk/src/lib.rs`
- `sdk/patina-sdk/src/toys.rs`
- `sdk/patina-sdk/README.md`
- `wit/toys/*.wit`
- `src/child/toy_host/*.rs`

## Verification Plan

1. Spec criteria status:
   - `patina spec check sdk-toybox-definition --json`
2. Inventory parity:
   - every `wit/toys/*.wit` appears once in canonical toybox table
   - each canonical row has boundary/resource/justification/tier/status fields
3. Safety gates before any merge/removal:
   - `cargo check -q -p patina-sdk --features knowledge-child,toy-log,toy-state,toy-session,toy-lake,toy-checkpoint,toy-query,toy-emit,toy-measure,toy-github,toy-connector,toy-peer,toy-events,toy-belief,toy-task`
   - `cargo check -q -p patina-sdk --features pipeline`
   - `cargo check -q -p patina-sdk --features task`
   - `cargo check -q -p patina-sdk --features command`
   - `cargo check -q -p patina-sdk --features mother-child`
   - `cargo test -q -p patina-ai scaffold::tests::test_scaffold`

## Build Readiness

Ready for promotion once cluster decisions and canonical toybox table are filled with explicit `keep-separate`/`merge`/`defer` outcomes.

## Open Questions

1. Which network cluster members are true boundary openings vs domain adapters over `http`?
2. Whether `emit` is a distinct opening from `events` or a constrained shape of one bus boundary.
3. Whether `schema` should remain standalone or fold under a storage/lake boundary contract.
