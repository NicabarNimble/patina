---
type: feat
id: crate-naming-policy-and-ci
status: blocked
created: 2026-03-13
sessions:
  origin: 20260313-061738
blocked_by:
  - mother-child-toy-beliefs-layout
exit_criteria:
  - id: naming-matrix-defined
    text: Spec contains an explicit current->target naming matrix for all workspace crates
    checked: true
  - id: full-conversion-implemented
    text: All workspace crates are renamed to target convention (except locked crates.io names patina-ai and patina-sdk)
    checked: false
  - id: docs-match-final-policy
    text: CONTRIBUTING.md and sdk/patina-sdk/README.md document final policy with no grandfather fallback
    checked: false
  - id: ci-strict-enforcement
    text: CI crate naming check enforces final convention with no legacy allowlist
    checked: false
  - id: sdk-policy
    text: sdk/patina-sdk/README.md explains patina-sdk family naming and relationship to patina-ai-* crates
    checked: false
---
# feat: crate naming policy and CI enforcement

> Define crate naming conventions and enforce them in CI with documented contributor guidance

## Problem

Crate naming has grown organically across root app crates, plugin crates,
native children, and SDK/protocol crates. We now have a clear direction:

- keep published crates stable (`patina-ai`, `patina-sdk`)
- adopt `patina-ai-*` as the default prefix for new app/runtime family crates
- keep SDK evolution discoverable under `patina-sdk*`

Without an explicit policy, migration plan, and strict CI enforcement, naming
drift will continue and crate extraction will create avoidable confusion.

## Goal

Define a canonical naming convention in contributor docs and SDK docs, migrate
all workspace crates to that convention, and enforce it in CI with no
grandfather fallback. The only locked crate names are published crates.io names
`patina-ai` and `patina-sdk`.

## Status

Blocked.

Rationale: this spec is critical, but final crate naming must run after full
folder/module boundary restructuring so the naming matrix targets the final
architecture and avoids churn from intermediate paths.

## Non-Goals

- Renaming published crates.io packages `patina-ai` and `patina-sdk`.
- Performing broad crate extraction/refactors in this spec.
- Enforcing crate naming for third-party dependencies from crates.io.

## Target Shape

- `CONTRIBUTING.md` includes a clear crate naming policy and migration stance.
- `sdk/patina-sdk/README.md` documents how SDK naming relates to app/runtime naming.
- CI runs a deterministic workspace crate-name validator.
- No legacy naming allowlist remains in policy enforcement.

## Workspace Naming Matrix

Locked published names:

- `patina-ai` (root app)
- `patina-sdk` (public SDK umbrella)

Current -> target crate names:

- `patina-ai` -> `patina-ai` (locked)
- `patina-sdk` -> `patina-sdk` (locked)
- `patina-pipe` -> `patina-ai-pipe`
- `patina-pipe-types` -> `patina-ai-pipe-types`
- `patina-child-sdk` -> `patina-ai-child-sdk`
- `patina-toy-sdk` -> `patina-ai-toy-sdk`
- `patina-plugin-models` -> `patina-ai-extension-models`
- `patina-plugin-repos` -> `patina-ai-extension-repos`
- `patina-doctor` -> `patina-ai-extension-doctor`
- `patina-plugin-ducklake` -> `patina-ai-child-ducklake`
- `patina-plugin-belief-verifier` -> `patina-ai-child-belief-verifier`
- `github-connector` -> `patina-ai-child-github-connector`
- `ducklake` -> `patina-ai-child-ducklake-native`

Notes:

- This spec migrates crate package names only. Directory layout moves are separate.
- `ducklake` native child remains transitional and should be removed by cutover specs.
- Naming matrix is provisional until boundary reorg is complete and crate
  ownership surfaces are finalized.

## Solution

1. Document policy in `CONTRIBUTING.md`:
   - `patina-ai` remains the top-level app crate.
   - Workspace crates use `patina-ai-*` by default.
   - `patina-sdk` remains one public SDK package for now.
   - No grandfather fallback language.

2. Document policy in `sdk/patina-sdk/README.md`:
   - clarify SDK family naming and relationship to app/runtime crates.

3. Add a CI-enforced checker script:
   - parse workspace packages from `cargo metadata`.
   - validate exact target naming matrix for current workspace crates.
   - fail fast with actionable remediation output.

4. Wire checker into test pipeline before formatting/clippy/tests.

## Implementation Order

1. Lock naming matrix in this spec.
2. Complete full folder/module boundary restructuring.
3. Refresh naming matrix against final crate boundaries.
4. Rename all workspace crates to target names and update dependency references.
5. Update policy text in contributor docs and SDK README to final (no fallback).
6. Update `resources/scripts/check-crate-names.sh` to strict matrix validation.
7. Run local validation and ensure CI check is green.

## Resolved Decisions

- Keep published crate names unchanged (`patina-ai`, `patina-sdk`).
- Use `patina-ai-*` for workspace crates (except locked `patina-sdk`).
- Remove grandfather allowlist behavior from CI once conversion lands.
- Keep a single public SDK package (`patina-sdk`) for now.

## Verification

- `bash resources/scripts/check-crate-names.sh`
- CI step added in `.github/workflows/test.yml` as `Check crate naming policy`

## Exit Criteria

1. Spec defines the current->target naming matrix for all workspace crates.
2. Workspace crates are migrated to target naming convention (except locked
   `patina-ai` and `patina-sdk`).
3. `CONTRIBUTING.md` and `sdk/patina-sdk/README.md` document final policy with no
   grandfather fallback.
4. CI crate naming check enforces strict convention with no legacy allowlist.
5. Local crate naming check passes after migration.

## Build Readiness

Ready; no schema/migration blockers.
