---
type: feat
id: child-registry-control-plane
status: active
created: 2026-04-24
updated: 2026-04-24
related:
- mother/src/registry.rs
- mother/src/runtime.rs
- mother/src/state/mod.rs
- mother/src/state/children_registry.rs
- src/commands/mother/mod.rs
- src/commands/mother/daemon.rs
- src/paths.rs
- children/slate-manager/
- crates/patina-protocol/src/lib.rs
beliefs:
- '[[spec-driven-design]]'
- '[[safety-boundaries]]'
- '[[dependable-rust]]'
- '[[adapter-pattern]]'
references:
- layer/core/values/spec-driven-design.md
- layer/core/values/safety-boundaries.md
- layer/core/values/dependable-rust.md
- layer/core/values/adapter-pattern.md
exit_criteria:
- id: crc1-registry-model
  text: Mother has a first-class child registry model (index, source, version, artifact, trust, compatibility, approval state) persisted in state DB and queryable via CLI/API.
  checked: true
- id: crc2-github-first-source
  text: Registry can ingest child entries from GitHub repositories/releases by default (owner/repo, tag/version, asset URL, checksums, optional signature metadata).
  checked: true
- id: crc3-provider-abstraction
  text: Registry source adapter contract supports GitHub now and allows Gitea/other git forge providers without changing core registry semantics.
  checked: true
- id: crc4-approval-lockdown
  text: Mother enforces explicit approval states (untrusted -> candidate -> approved -> blocked/deprecated) and denies install/assignment for non-approved entries unless force policy explicitly allows it.
  checked: false
- id: crc5-pin-and-verify
  text: Install and assignment flows are pin-first (name@version or digest), verify artifact hash/signature metadata, and fail closed on mismatch.
  checked: false
- id: crc6-project-assignment
  text: Project-level child assignment exists as Mother authority data (which projects may use which approved child versions) with clear audit events for grant/deny changes.
  checked: false
- id: crc7-operator-surface
  text: Operator surface exists under `patina mother children ...` for search/list/show/approve/block/pull/install/assign/sync with dry-run where side effects occur.
  checked: false
- id: crc8-external-child-ready
  text: Out-of-repo child workflow is validated by onboarding at least one external child repository (Slate), proving build->publish->registry sync->install->project assignment end-to-end.
  checked: false
- id: crc9-backward-compat
  text: Existing local child loading from ~/.patina/children remains operational; registry-backed install is additive and can coexist during migration.
  checked: true
- id: crc10-state-seam-modularity
  text: 'Mother state implementation follows dependable-rust/unix boundaries for this feature: child-registry state logic is isolated behind a dedicated store seam/module (`ChildRegistryStore`) rather than growing monolithic `state.rs`.'
  checked: true
validated_against_commit: 35db4b562f2052acf610e9fc7ad8f5f5b7d8431e
last_freshness_check: 2026-04-29T02:04:11Z
freshness_scope:
- mother/src/state/mod.rs
- mother/src/state/children_registry.rs
- mother/src/child_registry/mod.rs
- mother/src/child_registry/sync.rs
- mother/src/child_registry/github.rs
- src/commands/mother/mod.rs
- src/commands/mother/children.rs
---
# feat: Mother child registry control plane (GitHub-first, provider-pluggable)

> Build a real child registry so children can live outside Patina core, with Mother-managed trust, approval, pinning, and per-project assignment.

## Problem

Children are becoming the primary feature delivery unit (e.g. Slate), but distribution and trust are still ad-hoc:

- no canonical registry index model in Mother,
- no approval lifecycle for child artifacts,
- no pin/verify workflow as first-class policy,
- no provider abstraction for GitHub now + Gitea/others later.

This blocks the desired architecture: Patina core + Mother orchestration + external child ecosystem.

## Goal

Make Mother the authority for child registry and assignment:

1. Discover child artifacts from GitHub repos/releases by default.
2. Maintain an approval and trust lifecycle before install/use.
3. Install pinned, verified child artifacts into local runtime paths.
4. Assign approved child versions to projects under Mother control.
5. Keep provider adapters swappable (GitHub, Gitea, others).

## Non-Goals

- Implementing full OCI registry compatibility in this spec.
- Replacing all existing local child loading in one step.
- Solving marketplace UX and billing concerns.
- Defining cross-org federation policy beyond local Mother authority.

## Normative architecture

### 1) Registry domain model (Mother state)

Registry record fields (minimum):

- `child_name` (canonical)
- `version` (semver/string)
- `source_provider` (github|gitea|custom)
- `source_ref` (repo/tag/release id)
- `artifact_url`
- `artifact_sha256`
- `manifest_sha256` (if separate)
- `signature_ref` (optional)
- `patina_min` / compatibility metadata
- `declared_operations` and toy requirements (indexed from manifest)
- `approval_state` (`untrusted|candidate|approved|blocked|deprecated`)
- timestamps + operator audit metadata

### 2) Source provider adapters

Define provider adapter interface (core semantics stable, source-specific fetch logic isolated):

- `list_releases(source)`
- `resolve_artifact(version|tag)`
- `fetch_metadata(manifest/checksum/signature)`
- `normalize_to_registry_entry(...)`

GitHub adapter is first implementation. Gitea adapter follows same contract.

### 3) Trust and approval workflow

Approval lifecycle:

- newly synced entries default to `candidate` or `untrusted`
- operator (or policy rule) transitions to `approved`
- `blocked` hard-denies install/assignment
- `deprecated` warns but can remain installed

Mother enforces fail-closed policy:

- no assignment for non-approved versions
- no install if checksum/signature verification fails
- explicit override path must be auditable

### 4) Install and assignment lifecycle

Install flow:

1. resolve pinned child entry (`name@version` or digest)
2. download artifacts
3. verify hashes/signature metadata
4. stage atomically into `~/.patina/children/`
5. refresh/warmup child runtime

Assignment flow:

- bind approved child version(s) to project identity
- track assignment provenance and policy reason
- emit Mother audit events for grant/deny/revoke

### 5) Compatibility/migration policy

- existing local child loading remains supported
- registry-backed installations are additive first
- rollout supports mixed mode until children are externalized

## Operator UX (proposed)

Under `patina mother children`:

- `sources add github <owner>/<repo>`
- `sources add gitea <base-url> <owner>/<repo>`
- `sources list`
- `sync [--source ...]`
- `search <name>`
- `show <name> [--version ...]`
- `approve <name>@<version>`
- `block <name>@<version> [--reason ...]`
- `install <name>@<version> [--dry-run]`
- `assign <project> <name>@<version>`
- `unassign <project> <name>`
- `status`

## Security and policy constraints

- Hash verification is required before install.
- Signature metadata support is pluggable; absence can be policy-denied by strict mode.
- Approval state transitions are audited.
- Project assignment is explicit authority data, not inferred from local files.

## Externalization proof target

Use Slate as first external child:

1. Build/publish Slate artifact from its own repository.
2. Sync into Mother registry from GitHub source.
3. Approve pinned version.
4. Install to local children runtime path.
5. Assign to project(s).
6. Verify routed `patina spec`/Slate workflows execute through assigned child.

## Modularity requirement (core values lock)

Before implementing provider/CLI slices, child-registry state logic must be isolated behind a dedicated seam/module to avoid monolithic state growth.

- Boundary target: `ChildRegistryStore` (or equivalent) with a small, honest API.
- Existing `MotherRuntimeStore` may delegate to this seam.
- This refactor is in-scope and required by `crc10-state-seam-modularity`.

## Status

- State seam refactor completed (`ChildRegistryStore` extracted from monolithic state file).
- Slice A schema + state APIs completed.
- Slice B provider abstraction + GitHub sync ingestion completed.
- Minimal operator surface landed for sources/sync/add/enable/disable with JSON output.
- Remaining slices (approval/install/assignment-runtime/external proof) are still open.

## Solution

Mother owns child-registry authority state while provider adapters only supply discoverable release metadata.
The implemented path is now:

1. source records in Mother state DB,
2. provider-selected sync (`github` implemented),
3. normalized release ingestion into registry entries,
4. operator control-plane commands under `patina mother children`.

This preserves fail-closed behavior for unknown providers and non-approved assignment guardrails.

## Implementation Order

1. ✅ Slice A0 — state seam modularity (`ChildRegistryStore`).
2. ✅ Slice A — registry schema and state CRUD/guardrails.
3. ✅ Slice B — provider trait + GitHub release ingestion.
4. 🔲 Slice C — approval + install pin/verify workflow.
5. 🔲 Slice D — assignment runtime enforcement + audit flow.
6. 🔲 Slice E — full operator surface parity (`search/show/approve/block/install/assign/...`).
7. 🔲 Slice F — external Slate proof end-to-end.

## Resolved Decisions

- Child-registry persistence stays behind a dedicated store seam (dependable-rust boundary).
- GitHub is the first provider; provider trait is stable seam for Gitea/custom follow-on.
- Unknown/unimplemented providers fail closed (no implicit fallback).
- Registry-backed install remains additive to existing local child loading.

## Build Readiness

- `cargo fmt --all` passes.
- `cargo check -q` passes.
- Mother state + child-registry tests pass.
- CLI command tests for `mother children` paths pass.

## Verification

```bash
patina spec check child-registry-control-plane --json
cargo check -q --workspace
```

Scenario checks:

1. GitHub source sync discovers new child versions.
2. Non-approved version assignment is denied.
3. Approved pinned version installs and loads.
4. Assignment drives runtime routing for selected projects.
5. Adapter swap to Gitea works without changing domain semantics.

## Exit Criteria

Frontmatter `crc1..crc9` are source of truth.
