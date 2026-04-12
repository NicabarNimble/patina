---
type: feat
id: spec-atlas-mct-visibility
status: draft
created: 2026-04-11
related:
- layer/surface/build/feat/sdk-vision-lock/SPEC.md
- layer/surface/build/feat/sdk-developer-platform/SPEC.md
- layer/surface/build/feat/child-construction-canon/SPEC.md
- src/commands
- layer/surface/build/
beliefs:
- '[[sdk-is-mct-entry-point]]'
- '[[children-have-agency-toys-are-capabilities]]'
- '[[wasi-is-foundation-not-option]]'
exit_criteria:
- id: samv1-atlas-command
  text: "`patina atlas` command exists and runs without Mother dependency, producing deterministic project visibility output from local repo truth."
  checked: false
- id: samv2-spec-graph
  text: "Atlas output includes spec inventory with status, criteria progress, and dependency edges (`blocked_by` + resolvable `related`)."
  checked: false
- id: samv3-mct-inventory
  text: "Atlas output includes child manifest/toy visibility (`[needs].toys`, kind/role lane hints) and toy registry visibility from `wit/toys/deps/toys-registry.toml`."
  checked: false
- id: samv4-html-demo-surface
  text: "Atlas can render a standalone HTML dashboard from the same snapshot data so HITL can review spec sprawl and MCT shape visually."
  checked: false
- id: samv5-fail-closed
  text: "Atlas fails closed on malformed SPEC frontmatter with deterministic error path tests."
  checked: false
- id: samv6-proof-commands
  text: "Verification commands are documented and produce a demo artifact that can be walked through end-to-end."
  checked: false
---
# feat: Spec Atlas + MCT Visibility

> Build a local-first visibility surface for spec sprawl and MCT shape using Patina-native truth (spec files, child manifests, toy registry), with both JSON and HTML outputs.

## Problem

Spec and architecture truth is present but distributed across many files and command outputs.
Reviewing state requires too much manual cross-referencing, which increases drift risk.

## Goal

Provide a deterministic visibility layer that answers:

1. What specs exist, in what status, and with what criteria progress?
2. How specs depend on each other (`blocked_by`, `related`)?
3. What does the current MCT surface look like (children + toys)?
4. Can this be reviewed quickly in a visual artifact (HTML) without requiring Mother?

## Non-Goals

- Replacing `patina spec` lifecycle commands.
- Running a network service by default.
- Changing capability policy semantics.

## Solution Shape

- Add `patina atlas` command in Rust.
- Source data from:
  - `layer/surface/build/**/SPEC.md`
  - `children/*/child.toml`
  - `wit/toys/deps/toys-registry.toml`
- Build one normalized snapshot model.
- Emit as:
  - JSON (stdout or file)
  - standalone HTML dashboard (single file)

## Verification

```bash
cargo test -q atlas
patina atlas --json
patina atlas --html --output .tmp/atlas/spec-atlas.html
```

## Build Readiness

High. This is read-only aggregation over existing project artifacts and aligns with current MCT + SDK governance direction.
