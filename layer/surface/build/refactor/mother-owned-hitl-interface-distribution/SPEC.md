---
type: refactor
id: mother-owned-hitl-interface-distribution
status: active
created: 2026-04-10
sessions:
  origin: 20260410-105046-075601000
related:
- src/main.rs
- src/commands/launch/internal.rs
- src/commands/ai/surface.rs
- src/interface/launch.rs
- src/interface/internal/bundle.rs
- src/interface/runtime/templates.rs
- src/session/internal/live.rs
beliefs:
- '[[core-verbs-standalone-mother-additive]]'
- '[[session-as-interface-agnostic-work-record]]'
- '[[stale-context-is-hostile-context]]'
exit_criteria:
- id: mhid1-launcher-ux-stable
  text: '`patina` with no subcommand keeps current launcher UX and behavior: if outside a Patina project, show ''Are you lost?'' flow; on accept, prompt interface selection and launch directly into the selected interface session.'
  checked: true
- id: mhid2-ai-command-stable
  text: '`patina ai <interface>` keeps current command contract: same command shape, no new required flags, existing flags still accepted (`--path`, `--force`, `--tmux`, `--no-tmux`), and launch/session wrapper flow preserved for claude|gemini|opencode|pi.'
  checked: true
- id: mhid3-hitl-taxonomy
  text: Interface selection prompt reads 'Available HITL interfaces', and registry distinguishes HITL interfaces from non-HITL/agent runtime surfaces.
  checked: true
- id: mhid4-pi-interface-added
  text: PI appears as a 4th HITL interface in the same selection and launch pathways as Claude/Gemini/OpenCode, sourced from registry metadata instead of hardcoded CLI lists. PI is the proof artifact that registry-based representation/launch works for a non-legacy interface.
  checked: true
- id: mhid5-mother-registry-authority
  text: Mother becomes authority for interface package ownership/distribution (manifests, templates, skills references, bundle metadata). CLI reads interfaces from Mother-managed registry, not static arrays. Completion requires PI to be registry-defined and launchable without hardcoded interface additions.
  checked: true
- id: mhid6-mother-skills-authority
  text: Mother-owned skills repository is the authoritative source for projected Patina/HITL skills. For built-in HITL interfaces, projected command surfaces include the existing wrapper commands (`/session-start`, `/session-update`, `/session-note`, `/session-end`).
  checked: false
- id: mhid7-mother-session-authority
  text: 'Session lifecycle remains Mother-authoritative: start/update/note/end continue to produce tags, durable artifacts, and active interface pointers through existing wrapper/script flows.'
  checked: false
- id: mhid8-init-existing-flows-preserved
  text: 'Both paths stay intact while ownership shifts: (a) new project bootstrap (`patina` lost-flow + init + launch), (b) existing project flows (`patina`, `patina ai`, `patina ai setup`).'
  checked: true
- id: mhid9-no-age-eviction
  text: 'No time-based eviction in this phase: active HITL sessions are never auto-cleared due to age. Existing explicit lifecycle paths (`session-end` / `patina ai end`) remain the cleanup boundary while ownership moves to Mother.'
  checked: false
- id: mhid10-compile-proof
  text: cargo check --workspace -q passes; launcher smoke checks pass for both `patina` and `patina ai <interface>` entry paths with unchanged UX semantics.
  checked: true
- id: mhid11-managed-path-governance
  text: Managed interface paths are explicit metadata and centralized in runtime path handling (via Patina path utilities). Projection/cleanup operations only touch declared managed paths and write an operation log for deterministic cleanup auditing.
  checked: false
- id: mhid12-skill-registry-model
  text: Skills are modeled as Mother-managed registry packages with interface-specific projection adapters. Registry remains intentionally small and opinionated for Patina client-infrastructure and MCT-adjacent workflows while preserving current wrapper command surfaces.
  checked: false
- id: mhid13-project-detection-unified
  text: 'Project detection uses one canonical predicate across launcher/session flows. Canonical Patina project test in this phase is: `.patina/config.toml` exists AND `layer/` directory exists.'
  checked: true
- id: mhid14-bootstrap-decision-tree
  text: 'Detect/bootstrap behavior is deterministic: (1) picker lists detected HITL interfaces only, (2) explicit `patina ai <interface>` with detect failure hard-fails with install guidance, (3) vendor bootstrap files are projected only after detect succeeds and launch flow proceeds. No detect-fail auto-bootstrap retries in this phase.'
  checked: false
- id: mhid15-operation-log-contract
  text: 'Managed-path operation log contract is explicit and testable: append-only JSONL at `.patina/local/interface-ops.jsonl`, one record per mutation with `{ts, interface, runtime_id, op, path, result}`, idempotent cleanup writes `result=skipped` when path already absent, and concurrent writes are serialized by file lock.'
  checked: true
- id: mhid16-pi-registry-proof-hard
  text: 'PI registry-proof is mechanically enforced: adding PI requires registry/package metadata only and no compile-time interface-name additions in static catalogs/enums. Verification requires an integration test that loads registry fixture data containing PI metadata and proves PI discovery/launch from registry wiring alone.'
  checked: true
validated_against_commit: 22a5b25f0d99
last_freshness_check: 2026-04-10
freshness_scope:
- src/commands/launch/internal.rs
- src/commands/ai/surface.rs
- src/interface/launch.rs
- src/interface/internal/bundle.rs
- src/interface/runtime/templates.rs
- src/session/internal/live.rs
- src/session/internal/projection.rs
- src/paths.rs
---
# refactor: Mother-Owned HITL Interface Distribution

## Problem

Authority for HITL interfaces/skills/sessions is split across hardcoded CLI lists,
embedded templates, and Mother runtime state. This prevents Mother from being the
single operational owner.

## Goal

Make Mother the owner of HITL interface and skill distribution while preserving
user-visible behavior for:

1. `patina` launcher mode
2. `patina ai <interface>` launch mode
3. Session lifecycle wrappers (`session-start/update/note/end`)

This is an ownership/distribution refactor only.

PI definition for this spec: PI is a HITL interface surfaced through the same
Patina launcher and `patina ai <interface>` contracts as other HITL interfaces.

## Core Value Anchors

- **Adapter Pattern:** read existing code paths fully before introducing or
  removing abstractions; no speculative boundaries.
- **Dependable Rust:** keep public behavior stable; constrain changes to minimal
  internal seams that preserve command contracts.
- **Safety Boundaries:** mutate only declared project-scoped managed paths.
- **Execution Discipline:** read code before write/remove code.
- **Git Discipline:** update with scalpel, not shotgun.

## Status

Draft. Ready for implementation planning and phased build execution.

## Non-Goals

- Redesigning launcher UX copy/flow beyond the explicit HITL label change.
- Refactoring command dispatch size (`src/main.rs`) in this phase.
- Reworking session artifact model (summary vs full logs) in this phase.
- Introducing breaking CLI changes, new required flags, or altered command shape.
- Replacing tmux/session orchestration semantics.

## Execution Topology

This is a single umbrella refactor executed in three gated phases:

1. **Phase A — Interface Registry Contract**
   - Mother registry authority for HITL interface discovery/selection/launch.
   - Includes PI as proof artifact for registry generality.
2. **Phase B — Skills Authority Migration**
   - Mother skill registry authority with interface-specific projection adapters.
3. **Phase C — Session/Path Ownership Hardening**
   - Mother lifecycle authority + managed path governance + operation log.

A phase must satisfy its relevant exit criteria before advancing.

## Current State

- **Launcher path:** `patina` no-subcommand routes to launch; outside a project
  it uses the 'Are you lost?' init-and-launch flow.
- **Patina project test is split across code paths:** project checks use
  `.patina/config.toml` in launch path and `.patina/ + layer/` in session tooling.
- **Interface inventory is partly hardcoded:** list/detect/select logic relies on
  static interface lists and enum matching.
- **Skills are distributed via interface template bundles:** not fully sourced from
  a Mother-owned skill repository.
- **Session state:** Mother tracks session records/tags/artifacts; interface
  ownership/distribution is not yet fully Mother-authoritative.

## Target State

- Mother owns HITL interface package registry and distribution metadata.
- CLI launcher reads available HITL interfaces from Mother registry.
- Prompt copy becomes `Available HITL interfaces`.
- PI is added as a first-class HITL interface using registry/package metadata.
- Mother owns skill package source and projection inputs for HITL interfaces.
- `patina` and `patina ai <interface>` behavior is preserved from user perspective.

## Solution

### 1) Define HITL Taxonomy in Interface Metadata

Introduce explicit classification in Mother-managed interface manifests so launch
selection can filter to HITL-only interfaces. This keeps human CLI interfaces
separate from agent/runtime surfaces.

Required metadata fields for this phase:
- `name`
- `display`
- `classification` (`hitl` only)
- `detect_commands` (ordered; first success marks detected)
- `vendor_bootstrap` (optional)
- `managed_paths` (explicit, complete)
- `tmux_policy`
- `skills.include`

Manifest validation behavior in this phase:
- missing required field => interface marked invalid and excluded from picker,
- explicit launch to invalid interface => hard fail with field-level validation error,
- invalid interfaces do not block valid interface discovery.

HITL distribution is Mother client infrastructure (human-facing interface
management), not MCT child/toy composition.

### 2) Mother-Owned Registry as Runtime Source of Truth

Move runtime interface discovery for launcher/AI commands to Mother registry
records (manifests + package state) instead of static arrays in CLI code.

Detection semantics remain aligned with current behavior:
- picker flow shows detected HITL interfaces,
- explicit `patina ai <interface>` fails with install/detect error when not detected.

`detect_commands` execution contract in this phase:
- probe runner executes commands directly (no shell interpolation),
- environment inherits current process `PATH` and environment,
- per-command timeout is 3000ms,
- success = exit code 0,
- failure classes = timeout / non-zero exit / executable not found,
- probe sequence stops on first success.

Bootstrap semantics in this phase:
- detect failure does not trigger auto-bootstrap/retry,
- vendor bootstrap projection only runs after detect succeeds.

PI is the required proof interface for this phase.

### 3) Mother-Owned Skills Distribution

Treat skills as Mother-managed packages and project them into interface surfaces
through existing projection behavior. Change authority/distribution only.

Registry model:
- intentionally small,
- opinionated for Patina client-infrastructure and MCT-adjacent workflows,
- interface-specific projection adapters allowed/required.

No attempt is made in this phase to force identical skill layout across all
HITL interfaces.

### 3a) Managed Paths and Operation Log

Managed paths are first-class runtime data and must be centralized through
Patina path handling. Projection and cleanup operations must:
- only mutate declared managed paths,
- emit an operation log (create/update/delete + path + interface + runtime_id)
  suitable for deterministic cleanup/audit.

Operation log contract in this phase:
- location: `.patina/local/interface-ops.jsonl`,
- record shape: `{ts, interface, runtime_id, op, path, result}`,
- append-only retention,
- idempotent cleanup writes `result="skipped"` when path is already absent,
- write serialization uses an exclusive file lock on the log file,
- lock timeout is 1000ms; on timeout, operation fails closed with actionable error.

### 4) Session Authority Clarification

Keep the current wrapper workflow and UX, but ensure the canonical lifecycle
state remains Mother-owned and explicitly modeled as such in runtime paths.

Stale pointer reconciliation in this phase:
- active-runtime predicate source of truth is Mother `mother_sessions` state:
  `status=active` for the referenced `runtime_id`,
- on launch/start paths, if interface pointer runtime_id does not resolve to
  an active Mother session record for the project+interface, clear the pointer
  before continuing,
- `patina ai end` remains idempotent and clears pointers even if runtime process
  is already gone.

### 5) Abandoned Session Handling (No Auto-Clear by Time)

Do not clear active HITL interface state based on elapsed time. Sessions can
remain open for days.

Phase boundary: this spec does not add a new heartbeat/liveness subsystem.
Cleanup remains explicit via current lifecycle commands.

## Implementation Order

1. **Metadata + taxonomy:** add HITL classification and PI manifest/package.
2. **Read-path cutover:** switch launcher and `patina ai` discovery/listing to
   Mother registry while preserving UX output and command behavior.
3. **Skills authority cutover:** source skill projection inputs from Mother-owned
   package roots with wrapper-compatible projection output.
4. **Path governance cutover:** centralize managed-path handling and operation log
   so projection/cleanup mutate only declared paths.
5. **Vendor/tmux parity hardening:** preserve existing vendor bootstrap and tmux
   behavior by manifest-driven metadata with no UX drift.
6. **Session ownership hardening:** unify project detection predicate and verify
   session wrappers/command paths use Mother lifecycle state consistently.
7. **Session policy hardening:** enforce no time-based eviction, explicit cleanup,
   and stale-pointer reconciliation behavior.
8. **PI hard proof:** verify PI lands via registry wiring without compile-time
   interface catalog additions.
9. **Compatibility verification:** smoke test new-project and existing-project
   flows to prove no user-facing behavior drift.

## Resolved Decisions

- **Behavior lock:** preserve launcher and `patina ai <interface>` behavior.
- **Ownership boundary:** this phase changes ownership/distribution only.
- **HITL label:** launcher prompt is `Available HITL interfaces`.
- **PI:** add PI as a first-class HITL interface.
- **PI proof role:** PI is the explicit proof that registry representation and
  launch are not hardcoded to legacy interfaces.
- **Detection contract:** keep current detect model (ordered command probes,
  picker shows detected, explicit launch errors if not detected).
- **Bootstrap contract:** no detect-fail auto-bootstrap/retry in this phase.
- **Vendor bootstrap boundary:** keep interface-specific vendor bootstrap metadata;
  no forced cross-interface abstraction in this phase.
- **Managed paths:** centralize path handling and require operation logging for
  projection/cleanup mutations.
- **Tmux boundary:** preserve current tmux behavior; make policy manifest-driven
  and documented by code.
- **Skills model:** Mother skill registry is small/opinionated for Patina
  client-infrastructure and MCT-adjacent workflows; projection remains
  interface-specific.
- **Vocabulary stance:** HITL distribution is a Mother client-infrastructure
  concern, distinct from MCT child/toy runtime composition.
- **Mother scope stance:** Big Mother is accepted in this phase. Decomposition
  is tracked as a separate follow-up spec and not a blocker for this migration.
- **No age eviction:** no time-based session cleanup.
- **Cleanup boundary:** cleanup via `session-end` and `patina ai end` only.
- **Stale-pointer handling:** stale interface pointers are reconciled on
  launch/start and cleared by idempotent end paths.
- **Runtime active predicate:** active is defined by Mother session status
  (`mother_sessions.status=active`) for project+interface+runtime_id.
- **Liveness boundary:** no new heartbeat/orphan automation in this phase.
- **Taxonomy boundary:** `hitl` classification only in this phase.
- **SDK taxonomy boundary:** `patina_sdk::InterfaceKind` remains an SDK
  classification type used by session records and is out of scope for this
  spec. `mhid5`/`mhid16` apply only to CLI-side catalogs (`src/interface/launch.rs`,
  `patina ai` dispatch, and interface factory wiring in `src/interface/mod.rs`).
  SDK-level taxonomy decomposition is deferred to a future spec because Mother
  shape is under active review for MCT/wasm direction and changing SDK taxonomy
  now would be wasted motion before that redesign lands.
- **Skills authority vs storage:** Mother is the authoritative owner of skill
  source content and projection selection in this phase. Storage mechanism
  (embedded `include_str!`, filesystem under `~/.patina/skills/`, or wasm
  packages) is implementation detail and intentionally flexible while Mother
  shape is under review for MCT/wasm direction. Long-term vision is a small,
  curated Mother skill registry (`skills.sh`-shaped at smaller scale);
  `mhid6`/`mhid12` close on ownership/selection, not storage layout.

## Verification

```bash
# Spec sanity
patina spec show mother-owned-hitl-interface-distribution

# Compile
cargo check --workspace -q

# Canonical project predicate must be shared across launcher/session
# - `.patina/config.toml` exists
# - `layer/` exists

# Existing project launcher path (must launch default HITL interface as before)
patina

# Existing project direct ai path (same behavior/expectations as before)
patina ai claude
patina ai gemini
patina ai opencode
# plus new HITL interface
patina ai pi

# Explicit lifecycle cleanup remains authoritative
patina ai end

# Managed path governance (manual/automated assertions)
# - projection/cleanup only touches declared managed_paths
# - operation log contains path mutations with interface + runtime_id
# - operation log path: `.patina/local/interface-ops.jsonl`

# PI hard proof
# - required integration test uses registry fixture data to register PI
# - PI appears in discovery/selection and launches without compile-time
#   interface catalog/enum additions

# Outside project path must preserve "Are you lost?" UX
mkdir -p /tmp/patina-hitl-smoke && cd /tmp/patina-hitl-smoke && patina

# Interface selection prompt text must use HITL terminology
# (manual check during launcher prompt)
```

## Exit Criteria

Frontmatter criteria `mhid1..mhid16` are the source of truth.

## Build Readiness

Medium-High. Existing launcher/session architecture supports most target behavior.
Main implementation risk is replacing hardcoded interface authority with
Mother-managed registry/package authority without UX drift.
