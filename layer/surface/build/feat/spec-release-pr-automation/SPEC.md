---
type: feat
id: spec-release-pr-automation
status: draft
created: 2026-04-21
related:
  - src/commands/spec/internal/mutations.rs
  - src/release/internal.rs
  - .github/workflows/test.yml
  - .github/workflows/release.yml
  - install.sh
  - packaging/homebrew/Formula/patina.rb
references:
  - layer/core/values/spec-driven-design.md
  - layer/core/values/session-capture.md
  - layer/core/values/dependable-rust.md
  - layer/core/values/safety-boundaries.md
beliefs:
  - '[[spec-driven-design]]'
  - '[[session-capture]]'
  - '[[dependable-rust]]'
exit_criteria:
  - id: srpa1-single-lane-policy
    text: "Release policy is explicit and enforced as single-lane stable (no alpha/beta/nightly orchestration in this spec)."
    checked: false
  - id: srpa2-version-bump-to-pr
    text: "When a spec completion causes a version bump, Patina can generate and push a release branch and open a PR to main automatically."
    checked: false
  - id: srpa3-structured-pr-body
    text: "PR body is generated from `patina spec packet <id> --json` + deterministic verification metadata (same operator style as session updates)."
    checked: false
  - id: srpa4-pr-ci-unchanged
    text: "Release PR automation triggers the existing PR CI lane without changing current PR workflow behavior in this spec."
    checked: false
  - id: srpa5-main-ship-one-workflow
    text: "Main merge triggers one stable ship workflow that publishes GitHub release assets and updates Homebrew formula checksums/version in the tap."
    checked: false
  - id: srpa6-channel-alignment
    text: "GitHub release tag/assets, curl installer stable resolution, and Homebrew stable formula resolve to the same released version."
    checked: false
  - id: srpa7-phone-notify-ready
    text: "Workflow status surfaces are clear and actionable for GitHub mobile notifications (PR checks pass/fail, ship workflow pass/fail)."
    checked: false
  - id: srpa8-rollback-safe
    text: "Release automation fails closed with clear recovery steps; no partial publish leaves mismatched channel state without explicit operator instruction."
    checked: false
---
# feat: Spec-driven release PR automation (single stable lane)

> Keep shipping fast: one stable lane, one merge path, one ship workflow. Version-bump specs open release PRs automatically with AI-authored structured PR packets.

## Problem

Current flow has friction in two places:

1. PR pipeline carries some release-like work that slows merge feedback.
2. Shipping can require manual choreography across GitHub release, curl installer expectations, and Homebrew formula updates.

User intent is explicit: move fast with **one stable lane only** and keep release mechanics nearly push-button.

## Goal

Implement a minimal, fast release operating model:

1. Spec completion/version bump can auto-create a release PR.
2. PR description is machine-generated in the same operational style as session updates.
3. Release PR automation starts the existing PR CI lane unchanged.
4. Main merge runs a single stable ship workflow that aligns GitHub + curl + Homebrew.

## Non-Goals

- Multi-channel release orchestration (alpha/beta/nightly/canary).
- Multi-stage promotion models.
- PR CI lane redesign/tuning in this spec (follow-on if needed).
- Replacing the entire spec release/archive architecture.

## Value Anchors (layer/core)

- **Spec-Driven Design**: releases are driven by spec completion and explicit gates.
- **Session Capture**: PR packets should be deterministic, concise, and operationally legible like session updates.
- **Dependable Rust**: keep automation boundaries small and composable (branch/PR generation separate from ship workflow).
- **Safety Boundaries**: fail closed on partial release states; always provide rollback/remediation commands.

## Target Flow

1. Operator completes a release-bearing spec.
2. Automation opens `release/vX.Y.Z` PR to `main` with generated PR body.
3. PR checks run and operator receives status notifications.
4. Merge to `main` triggers single stable ship workflow.
5. Ship workflow:
   - builds/publishes GitHub release assets,
   - updates Homebrew formula in tap (version + sha),
   - leaves stable channels aligned.

## Solution Outline

### A) Release-PR generation command/script

Add a stable automation entrypoint (script or command) that:
- resolves version bump result,
- creates/switches release branch,
- commits release metadata changes,
- pushes branch,
- opens PR via `gh`.

### B) PR packet generator for release PR body

Use `patina spec packet <id> --json` as source and render a deterministic PR template:
- Why this release
- What changed
- Verification commands/results
- Safety/rollback notes
- Spec reference

### C) Main stable ship workflow

Single workflow on main merge:
- publish GitHub release assets,
- update Homebrew formula in tap,
- verify version coherence signals.

## Direct Code Targets

- `src/commands/spec/internal/mutations.rs` (version bump + completion seam integration)
- `src/release/internal.rs` (release behavior seam safety)
- `.github/workflows/release.yml` (stable publish path)
- `.github/workflows/` (new workflow(s) for release PR creation / ship orchestration)
- `install.sh` (stable resolution assumptions validation)
- `packaging/homebrew/Formula/patina.rb` (+ tap update automation path)

## Verification

```bash
patina spec check spec-release-pr-automation --json
cargo check -q --workspace
```

Scenario verification:
- complete one patch release spec -> release PR auto-opened,
- PR packet generated with deterministic sections,
- PR checks run and report clearly,
- merge triggers ship workflow,
- `patina --version`, GitHub release tag, installer stable, and Homebrew formula version match.

## Exit Criteria

Frontmatter `srpa1..srpa8` are source of truth.

## Build Readiness

High: release/build primitives already exist; this is primarily orchestration and CI-lane clarity.
