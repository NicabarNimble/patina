---
type: refactor
id: scrape-strategy-seam-exploration
status: draft
created: 2026-03-25
related:
  - layer/surface/build/refactor/patina-code-to-vision/SPEC.md
exit_criteria:
  - id: SSE1
    text: "Current scrape lanes are mapped with file:line evidence, explicitly separating core lanes (layer, beliefs) from non-core lanes (code, git, grammar/pipeline-backed)"
    checked: false
  - id: SSE2
    text: "A stable scrape strategy boundary contract is documented (inputs, outputs, error behavior, and capability expectations) that keeps `patina scrape` as core orchestrator"
    checked: false
  - id: SSE3
    text: "Parity proof plan is executable and deterministic for extraction candidates (code/git/grammar-backed lanes), including command-level acceptance criteria and rollback trigger"
    checked: false
  - id: SSE4
    text: "A decision packet recommends one of: keep in core, extraction-ready seam only, or childization pilot; recommendation includes risk matrix and non-goals"
    checked: false
---
# refactor: Explore scrape strategy seam extraction (CV11)

> Keep `scrape` core while making non-core scrape lanes extraction-safe via explicit seam contracts and parity gates.

## Problem

CV11 remains unresolved from `patina-code-to-vision`: scrape is structured but extraction boundaries are not yet locked with deterministic parity proof. Without a clear seam contract, any future move of code/git/grammar-backed lanes risks behavior drift.

## Goal

Explore and define the scrape seam so `patina scrape` remains core orchestrator while non-core lanes can be independently extracted or childized only with parity proof.

## Non-Goals

- No immediate extraction or childization in this spec.
- No behavior changes to current scrape outputs.
- No rewrite of layer/belief scrape lanes (these stay core).

## Scope

- Scrape lane inventory and seam boundaries.
- Contract definition for strategy lanes (request/response/error model).
- Parity harness plan for candidate lane extraction.
- Decision packet for follow-on implementation spec.

## Verification

- `cargo check -q`
- `cargo test -q`
- command-level parity plan proofs documented for candidate lanes
- explicit map from CV11 claim to concrete code anchors and proof commands

## Build Readiness

Ready to activate. This spec is the active seam-proof lane for CV11.
