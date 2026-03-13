---
type: refactor
id: mother-child-toy-beliefs-layout
status: active
created: 2026-03-13
sessions:
  origin: 20260313-061738
related:
- layer/surface/build/refactor/mother-doctrine-cleanup/DESIGN.md
- layer/surface/build/refactor/single-patina-sdk-consolidation/SPEC.md
- layer/surface/build/refactor/child-surface-path-realignment/SPEC.md
- layer/surface/build/refactor/ducklake-knowledge-child-cutover/SPEC.md
exit_criteria:
- id: beliefs-core-surface-is-explicit
  text: Core beliefs engine code is grouped under explicit beliefs/core-belief modules and documented as system center
  checked: false
- id: mother-child-toys-module-boundaries-implemented
  text: Runtime code is reorganized into clear mother, child, and toys module surfaces with canonical ownership
  checked: false
- id: toy-catalog-and-discoverability-exist
  text: Toys have one canonical catalog/registry location so capabilities are discoverable without code hunting
  checked: false
- id: ducklake-legacy-native-path-removed
  text: Legacy native ducklake path is deleted and no legacy folder is introduced for ducklake fallback
  checked: false
- id: core-tools-surface-extracted
  text: Core tools (including spec and scrape-code surfaces) are moved under explicit core-tools ownership boundary
  checked: false
- id: layer-output-contract-preserved
  text: layer/core, layer/surface, and layer/dust output contract remains intact
  checked: false
- id: ci-drift-guards-enforced
  text: CI checks block regressions against mother/child/toys/beliefs boundaries and deleted legacy ducklake path
  checked: false
---
# refactor: refactor: mother-child-toy-beliefs layout and debt cleanup

> Restructure Patina around beliefs core plus mother/child/toys boundaries, remove legacy ducklake debt, and extract core tools like spec/scrape-code into explicit core-tools surface

## Problem

Patina has converged conceptually on beliefs-first + Mother/Child/Toy runtime,
but code and path layout still carries historical layering and terminology debt.
That debt slows shipping and makes architecture harder to read.

Specific pain points:

- canonical responsibilities are not physically obvious in tree layout
- toy implementation/discovery is distributed and requires code hunting
- legacy DuckLake native path still exists and keeps dual-path complexity alive
- core tools (e.g. spec workflow and scrape-code) are mixed into broad command
  surfaces instead of a clear core-tools boundary
- architecture docs and actual paths drift under pressure

## Goal

Establish a durable repository and module layout where:

- beliefs are explicit top system core
- runtime is explicitly partitioned into mother, child, and toys
- core tools are explicit and separate from runtime domains
- legacy DuckLake native debt is removed (no legacy ducklake folder retained)
- layer output contract (`layer/core`, `layer/surface`, `layer/dust`) is preserved

The result should reduce ambiguity and increase shipping velocity.

## Status

Active.

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
- `src/core_tools/**` contains core tools such as spec and scrape-code flows.

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
- Spec workflow + scrape code command internals -> `src/core_tools/**`
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

6. **Core tools extraction**
   - group spec/scrape-code internals under `core_tools` ownership boundary.

7. **Guardrails**
   - add CI checks to prevent regression to mixed/legacy placement.

## Implementation Order

1. Write DESIGN.md with explicit `from -> to` mapping and rationale.
2. Establish canonical roots (`src/beliefs`, `src/child`, `src/toys`,
   `src/core_tools`) with module plumbing.
3. Move belief + toy + child + core-tool internals in small slices.
4. Remove legacy native ducklake path and adjust runtime/tests/spec references.
5. Add/enable CI drift guards.
6. Run verification suite and close dependent specs.

## Resolved Decisions

- Architecture centers on beliefs + Mother/Child/Toy.
- `layer` output contract remains unchanged.
- Legacy ducklake native fallback is dead tech debt and must be removed.
- Core tools (`spec`, `scrape-code`) deserve explicit ownership surface.
- Structural clarity is a shipping acceleration feature, not cosmetic refactor.

## Verification

- `cargo check --workspace`
- `cargo test -q -p patina-ai -- src/plugin/internal/tests.rs`
- `cargo test -q -p patina-ai -- src/commands/spec/internal/tests.rs`
- `cargo test -q -p patina-ai -- src/commands/scrape/internal/tests.rs`
- `rg "children/ducklake" src children sdk tests Cargo.toml layer/surface/build` returns no legacy-native references except explicitly archived history/docs
- `rg "src/(beliefs|mother|child|toys|core_tools)" layer/surface/build/refactor/mother-child-toy-beliefs-layout/DESIGN.md` confirms documented target roots
- `patina spec check mother-child-toy-beliefs-layout --json`

## Exit Criteria

Use frontmatter exit_criteria as source of truth.

## Build Readiness

- [ ] DESIGN.md includes exact file move map.
- [ ] Canonical module roots exist and compile.
- [ ] DuckLake legacy native path removed.
- [ ] Core tools extraction completed for spec + scrape-code.
- [ ] CI drift guards active.
