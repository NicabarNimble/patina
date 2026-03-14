---
type: refactor
id: boundary-shim-removal-and-proof
status: complete
created: 2026-03-14
sessions:
  origin: 20260313-155708
related:
  - layer/surface/build/refactor/mother-child-toy-beliefs-layout/SPEC.md
exit_criteria:
  - id: mother-graph-host-shim-removed
    text: "Temporary compatibility shim `src/mother/graph_host.rs` is removed and no active callsites depend on `mother::graph_host`"
    checked: true
  - id: remaining-mother-shims-removed
    text: "Remaining runtime compatibility shim `src/mother/child.rs` is removed after callsites migrate"
    checked: true
  - id: boundary-guards-remain-green
    text: "Runtime boundary and layer contract guard scripts remain green after shim removals"
    checked: true
  - id: workspace-verification-remains-green
    text: "`cargo check --workspace` and targeted runtime/plugin tests stay green after each slice"
    checked: true
---
# refactor: boundary shim removal and proof

> Remove temporary compatibility shims introduced during boundary migration,
> proving the new `beliefs`/`child`/`toys` ownership surfaces stand on their own.

## Problem

Boundary migration is functionally complete, but temporary shims still exist in
`src/mother/*`. Keeping them forever weakens ownership clarity and allows drift
back to legacy import surfaces.

## Goal

Delete compatibility shims slice-by-slice with explicit proof (search + compile
+ focused tests) so reorg completion is durable and reversible per commit.

## Status

Complete.

Slice 1 complete: removed `src/mother/graph_host.rs` shim and `mod graph_host`
declaration after confirming no callsites depend on it.

Slice 2 complete: removed `src/mother/belief_host.rs` shim and `mod belief_host`
declaration after confirming no callsites depend on it.

Slice 3 complete: removed `src/mother/graph.rs` shim and `mod graph`
declaration, and switched `mother` graph exports to re-export from
`crate::beliefs::*` directly.

Slice 4 complete: removed `src/mother/child.rs` shim and `mod child`
declaration, and switched `mother` child trait exports to re-export directly
from `crate::child::runtime::*`.

## Non-Goals

- No broad runtime rewrites.
- No behavior changes to graph/belief/child semantics.
- No crate/package rename work.

## Slice Method

For each shim:

1. Verify no active callsites use the shim path.
2. Remove shim and module declaration.
3. Run verification set:
   - `cargo check --workspace`
   - `bash resources/scripts/check-runtime-boundaries.sh`
   - `bash resources/scripts/check-layer-output-contract.sh`
   - one focused plugin/mother test.
4. Commit with explicit rollback note (if break appears, reintroduce only that
   shim in a small revert/fix commit).

## Verification

- `rg "mother::(graph_host|graph|belief_host|child)::" src`
- `cargo check --workspace`
- `bash resources/scripts/check-runtime-boundaries.sh`
- `bash resources/scripts/check-layer-output-contract.sh`
- targeted tests per removed shim slice

## Build Readiness

Ready.
