---
type: refactor
id: mother-child-toy-beliefs-layout
status: complete
created: 2026-03-13
updated: 2026-03-13
sessions:
  origin: 20260313-061738
blocked_by: []
related:
- layer/surface/build/refactor/ducklake-knowledge-child-cutover/SPEC.md
- layer/surface/build/refactor/ducklake-native-removal-and-verification/SPEC.md
- layer/surface/build/refactor/child-plugin-sdk-alignment/SPEC.md
- layer/surface/build/refactor/doctrine-boundary-reorg-no-core-tools/SPEC.md
- layer/surface/build/feat/spec-shadow-knowledge-child/SPEC.md
exit_criteria:
- id: beliefs-core-surface-is-explicit
  text: Core beliefs engine code is grouped under explicit beliefs/core-belief modules and documented as system center
  checked: true
- id: mother-child-toys-module-boundaries-implemented
  text: Runtime code is reorganized into clear mother, child, and toys module surfaces with canonical ownership
  checked: true
- id: toy-catalog-and-discoverability-exist
  text: Toys have one canonical catalog/registry location so capabilities are discoverable without code hunting
  checked: true
- id: ducklake-legacy-native-path-removed
  text: Legacy native ducklake path is deleted and no legacy folder is introduced for ducklake fallback
  checked: true
- id: core-tools-surface-deferred
  text: Core tools extraction is explicitly deferred for this spec and command surfaces remain stable while runtime boundaries are completed
  checked: true
- id: layer-output-contract-preserved
  text: layer/core, layer/surface, and layer/dust output contract remains intact
  checked: true
- id: ci-drift-guards-enforced
  text: CI checks block regressions against mother/child/toys/beliefs boundaries and deleted legacy ducklake path
  checked: true
---
# refactor: refactor: mother-child-toy-beliefs layout and debt cleanup

> Restructure Patina around beliefs core plus mother/child/toys boundaries and remove legacy ducklake debt; defer core-tools extraction to follow-up specs

## Status Note

This spec runs in small execution slices to reduce blast radius.

Execution lane now is:

1. `ducklake-native-removal-and-verification` (complete)
2. `ducklake-knowledge-child-cutover` (complete)
3. `ducklake-child-path-name` (complete)
4. `mother-child-toy-beliefs-layout` (active, this spec)

The broad architecture intent remains valid, but implementation proceeds through the split specs above.

## Problem

Patina has converged conceptually on beliefs-first + Mother/Child/Toy runtime,
but code and path layout still carries historical layering and terminology debt.
That debt slows shipping and makes architecture harder to read.

Specific pain points:

- canonical responsibilities are not physically obvious in tree layout
- toy implementation/discovery is distributed and requires code hunting
- legacy DuckLake native path still exists and keeps dual-path complexity alive
- core runtime ownership (beliefs/mother/child/toys) still has mixed placement
  that slows implementation and review
- architecture docs and actual paths drift under pressure

## Goal

Establish a durable repository and module layout where:

- beliefs are explicit top system core
- runtime is explicitly partitioned into mother, child, and toys
- core tools extraction is explicitly deferred so runtime boundary completion can
  ship first without command-surface churn
- legacy DuckLake native debt is removed (no legacy ducklake folder retained)
- layer output contract (`layer/core`, `layer/surface`, `layer/dust`) is preserved

The result should reduce ambiguity and increase shipping velocity.

## Status

Active (slice execution).

Slice A (root boundary wiring) is in progress: canonical root facades now exist
for `beliefs`, `child`, `toys`, and `core_tools`, and a canonical toy catalog
surface is available at `src/toys/catalog.rs`.

Slice B started: lake toy host ownership moved from `src/mother/lake_host.rs`
to `src/toys/lake.rs`, and knowledge-child host calls now route through
`crate::toys::lake::*`.

Slice B continued: ingress toy host helpers extracted to `src/toys/ingress.rs`
and knowledge-child ingress host methods now delegate grant/source resolution
through `crate::toys::ingress::*`.

Slice B continued again: connector toy binding lifecycle helpers extracted to
`src/toys/connector.rs` (grant gating, list/upsert/remove/load, endpoint/type
validation), and knowledge-child connector host methods now delegate through
`crate::toys::connector::*`.

Slice B continued again: query and HTTP toy host delegation now route through
`src/toys/query.rs` and `src/toys/http.rs`, with `knowledge_child.rs` retaining
WIT host orchestration and grant wiring while behavior lives in toy modules.

Slice C started: beliefs graph core implementation moved to
`src/beliefs/graph.rs`, with a temporary compatibility shim at
`src/mother/graph.rs` to preserve external APIs during migration.

Slice C continued: graph host query/mutate runtime moved to
`src/beliefs/graph_host.rs`, with `knowledge_child` graph host bindings now
calling `crate::beliefs::graph_host::*` and a temporary compatibility shim at
`src/mother/graph_host.rs`.

Slice C continued: belief host query/mutate runtime moved to
`src/beliefs/belief_host.rs`, with `knowledge_child` belief host bindings now
calling `crate::beliefs::belief_host::*` and a temporary compatibility shim at
`src/mother/belief_host.rs`.

Slice D started: CI drift guards and contract checks added via
`resources/scripts/check-runtime-boundaries.sh` and
`resources/scripts/check-layer-output-contract.sh`, wired into
`.github/workflows/test.yml`.

Slice D continued: child runtime contracts moved from `src/mother/child.rs` to
`src/child/runtime.rs` with temporary compatibility shim, completing explicit
mother/child/toys boundary ownership at canonical module roots.

## Non-Goals

- Renaming published crates.io identities (`patina-ai`, `patina-sdk`).
- Rewriting the belief model itself in this spec.
- Creating a new build system in this spec.
- Moving every historical plugin/tooling crate in one pass.

## Current State

- Significant progress on single SDK and child path doctrine exists.
- Runtime code still has mixed ownership boundaries across modules.
- DuckLake has both new doctrine child and legacy native child footprints.
- Core tooling surfaces (spec/scrape-code) are command-heavy and not isolated as
  a named core-tools subsystem.

## Target State

### Top-Level System Shape

- `src/beliefs/**` (or equivalently named beliefs-core module) is explicit core.
- `src/mother/**` is Mother-only authority/orchestration.
- `src/child/**` is child runtime/registry/contracts.
- `src/toys/**` is toy host implementations + toy registry/catalog.
- `src/core_tools/**` exists as a placeholder boundary facade in this spec;
  internals stay in existing command surfaces for now.

### Project Output Shape (unchanged)

- `layer/core/**`
- `layer/surface/**`
- `layer/dust/**`

### Debt Policy

- Delete legacy native DuckLake path and references.
- Do not retain a `legacy/ducklake*` folder as fallback.
- Any transitional compatibility must be explicit in spec, time-bounded, and not
  encoded as permanent folder structure.

## Move/Ownership Matrix

- Belief engine/query/mutation modules -> `src/beliefs/**`
- Mother runtime authority modules -> `src/mother/**`
- Child execution/registry/lifecycle modules -> `src/child/**`
- Toy host capability implementations -> `src/toys/**`
- Spec workflow + scrape code command internals -> deferred in this spec
- `children/ducklake` native legacy -> deleted

Exact file-level mapping will be captured in DESIGN.md and executed in phases.

## Solution

1. **Lock architecture doctrine in design first**
   - encode boundaries and anti-drift constraints in DESIGN.md.

2. **Create canonical module roots**
   - introduce/normalize `beliefs`, `mother`, `child`, `toys`, `core_tools`.

3. **Move code by ownership, not convenience**
   - relocate modules to canonical roots with minimal behavior changes.

4. **Toy discoverability hardening**
   - add `src/toys/catalog.rs` (or equivalent) as single toy registry source.

5. **Remove dead DuckLake legacy path**
   - delete native legacy ducklake child and remove all references.

6. **Guardrails**
   - add CI checks to prevent regression to mixed/legacy placement.

## Implementation Order

1. Write DESIGN.md with explicit `from -> to` mapping and rationale.
2. Establish canonical roots (`src/beliefs`, `src/child`, `src/toys`,
   `src/core_tools`) with module plumbing.
3. Move belief + toy + child internals in small slices.
4. Remove legacy native ducklake path and adjust runtime/tests/spec references.
5. Add/enable CI drift guards.
6. Run verification suite and close dependent specs.

## Resolved Decisions

- Architecture centers on beliefs + Mother/Child/Toy.
- `layer` output contract remains unchanged.
- Legacy ducklake native fallback is dead tech debt and must be removed.
- Core tools extraction is deferred and handled by follow-up specs.
- Structural clarity is a shipping acceleration feature, not cosmetic refactor.

## Verification

- `cargo check --workspace`
- `cargo test -q -p patina-ai -- src/plugin/internal/tests.rs`
- `rg "children/ducklake" src children sdk tests Cargo.toml layer/surface/build` returns no legacy-native references except explicitly archived history/docs
- `rg "src/(beliefs|mother|child|toys)" layer/surface/build/refactor/mother-child-toy-beliefs-layout/DESIGN.md` confirms documented target roots
- `patina spec check mother-child-toy-beliefs-layout --json`

## Exit Criteria

Use frontmatter exit_criteria as source of truth.

## Build Readiness

- [ ] DESIGN.md includes exact file move map.
- [ ] Canonical module roots exist and compile.
- [ ] DuckLake legacy native path removed.
- [x] Core tools extraction deferred explicitly for this spec.
- [ ] CI drift guards active.
