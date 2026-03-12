---
type: refactor
id: mother-owned-interface-bundles
status: complete
created: 2026-03-12
sessions:
  origin: 20260311-190725-YNCW
related:
- /Users/nicabar/Projects/Sandbox/AI/RUST/patina/layer/surface/build/refactor/agentic-surface-architecture/SPEC.md
- /Users/nicabar/Projects/Sandbox/AI/RUST/patina/layer/surface/build/refactor/cli-mcp-skill-unification/SPEC.md
- /Users/nicabar/Projects/Sandbox/AI/RUST/patina/layer/surface/build/refactor/ai-launcher-surface-consolidation/SPEC.md
- /Users/nicabar/Projects/Sandbox/AI/RUST/patina/layer/surface/build/refactor/canonical-agents-surface/SPEC.md
- /Users/nicabar/Projects/Sandbox/AI/RUST/patina/layer/surface/build/refactor/init-interface-projection-separation/SPEC.md
beliefs:
- patina-identity
- mother-is-the-daemon
- interfaces-are-not-core
- mcp-is-shim-cli-is-product
- context-files-are-rules-not-docs
- skills-for-structured-output
- dependable-rust
- unix-philosophy
- safety-boundaries
exit_criteria:
- id: mother-bundle-catalog
  text: Mother owns a thin runtime catalog of Patina interface bundles for Claude, OpenCode, and Gemini and can report which bundles are available for deployment
  checked: true
- id: project-owned-truth
  text: Durable Patina interface truth remains project-owned on the patina branch, with layer and project files as git-tracked truth rather than Mother-only runtime state
  checked: true
- id: deploy-on-demand
  text: "`patina ai claude`, `patina ai opencode`, and `patina ai gemini` deploy or refresh their managed interface bundle on demand before launch when needed"
  checked: true
- id: default-front-door
  text: Running `patina` in an existing Patina project launches the project default interface, and running `patina` in an empty or non-Patina directory routes through a friendly bootstrap into the same bundle model
  checked: true
- id: managed-surface-ownership
  text: Patina-managed interface paths on the patina branch are explicitly reconciled, backed up when conflicting, and then replaced with Mother's managed surface while preserving allowed user customizations
  checked: true
- id: core-skills-packaged
  text: Each interface bundle includes the current Patina-specific core skills and command assets needed for session and belief workflow, with truthful MCP-first or CLI-JSON fallback guidance
  checked: true
- id: session-human-flow-preserved
  text: "`/session-start`, `/session-update`, and `/session-end` remain human-driven interface commands for this phase, with `/session-end` performing update then end, while backend behavior converges on one canonical session capability surface"
  checked: true
- id: update-path-explicit
  text: Patina exposes an explicit bundle refresh path so interface projections can be kept in sync without reintroducing adapter-era drift
  checked: true
---
# refactor: Mother-Owned Interface Bundles

> Package Claude, OpenCode, and Gemini as Mother-managed interface bundles deployed onto the patina branch, with thin runtime handshakes, project-owned truth, and native skills/commands over deterministic Patina capabilities.

## Current State

Patina has already converged much of the old adapter mess into one AI
surface, but the product model is still mid-transition.

What is true now:

- `patina ai setup` prepares Claude, OpenCode, and Gemini together
- `AGENTS.md` is the canonical root instruction surface
- setup reconciliation can back up and replace managed interface paths
- `patina` remains a friendly front door for first-run and project
  launch

What is still structurally unclear:

- interface assets are still largely thought of as copied adapter
  templates, not as Mother-owned bundles
- update and refresh logic still carries adapter-era vocabulary and code
- skills are mostly locked to interface-specific projections rather than
  one deliberate package model
- session commands still carry old single-player assumptions even though
  the desired user flow remains human-driven for now
- Mother is at risk of growing too broad if she owns too much durable
  interface truth instead of coordinating deployment

The repo itself is the island. Patina's real durable value lives in the
project and on the `patina` branch. Mother should coordinate interface
deployment and runtime attachment, not become the only place where
interface truth lives.

## Target State

The target model is:

- the project on the `patina` branch is the Patina island
- Mother owns a runtime catalog of interface bundles
- each bundle is a Patina-managed deployment for one interface
- the bundle is projected into the project and launched there
- durable text remains git-tracked in the repo
- skills provide promptful workflow
- CLI and MCP provide deterministic capability

Each supported interface has a Mother-managed bundle:

- `claude`
- `opencode`
- `gemini`

Each bundle contains:

- canonical Patina instruction truth rooted in `AGENTS.md`
- vendor-required root shim files such as `CLAUDE.md` or `GEMINI.md`
- interface-native command/skill assets
- helper scripts only where needed
- bundle metadata describing managed paths and version

Mother's job stays thin:

- know which interface bundles exist
- know which bundle version is current
- deploy or refresh the managed bundle into the current project
- attach runtime context for launch
- record that the interface checked in

Mother does not become the durable owner of project text. The repo
remains the durable source of truth.

## Design Rules

- project is the island
- `git` is the source of truth for durable project text
- `layer/` is the value Patina adds, even when interface traces also
  exist in the repo
- the `patina` branch is Patina-controlled operational territory
- Mother is thin runtime orchestration, not project document ownership
- skills are promptful wrappers over real capabilities
- CLI is the product surface; MCP mirrors it truthfully
- future universal skill catalogs are out of scope for this slice; this
  refactor packages current Patina-specific core skills first

## User Model

### 1. Empty or non-Patina directory

Running `patina` in an empty or non-Patina directory should preserve the
friendly old single-player feeling, but land in the new truthful model:

1. detect that this is not yet a Patina project
2. offer friendly bootstrap
3. initialize the project island on the `patina` branch
4. resolve or ask for the default interface
5. have Mother deploy the chosen interface bundle
6. launch through the normal wrapper path

### 2. Existing Patina project

Running `patina` in an existing Patina project should mean:

1. detect the Patina project
2. resolve project default interface
3. ensure that interface bundle is present and current
4. launch that interface

### 3. Explicit interface launch

Running `patina ai claude`, `patina ai opencode`, or
`patina ai gemini` should mean:

1. resolve current project
2. detect whether that interface bundle is already deployed and current
3. if not, have Mother deploy or refresh it
4. launch the requested interface directly

## Bundle Model

For this phase, a bundle is a Patina-managed deployment unit, not yet a
general-purpose skill catalog.

Each bundle should define:

- interface id and display name
- managed path set
- root bootstrap files
- packaged skills/command assets
- helper scripts and references
- bundle version or freshness marker
- truthful MCP/CLI fallback teaching requirements

The immediate core bundle contents should include Patina-specific
workflow assets such as:

- session commands/skills
- epistemic belief skill(s)
- Patina review/spec workflow assets already treated as core

This keeps the product useful now without waiting for the later
universal skill registry.

## Session Stance For This Slice

This refactor does not force the session workflow fully agentic yet.

For this phase, preserve the human-driven interface flow:

- `/session-start`
- `/session-update`
- `/session-end`

Rules:

- these remain interface-native command/skill assets
- `/session-end` performs update then end
- they should converge toward one canonical backend capability surface
  under `patina ai session ...` and/or `session.*`
- they should stop depending on old adapter-era behavioral truth living
  in copied interface files

Automatic session start at interface attach is explicitly deferred. The
current trusted human-in-the-loop start flow remains the product choice
for now.

## Steps

### 1. Define Mother-owned interface bundle metadata

Introduce a typed interface bundle model under the interface/Mother seam
that can answer:

- what interfaces Patina supports
- what managed paths belong to each interface
- what packaged assets belong to each interface
- how freshness/version is determined

This should replace the remaining adapter-era mental model without
turning Mother into a document warehouse.

### 2. Make project detection and front-door routing explicit

Clarify what makes a directory a Patina project and route `patina`
through one truthful flow:

- not a project -> bootstrap
- Patina project -> launch default interface bundle

Project detection should reflect the project-island model, not just
historical accidents of the old adapter system.

### 3. Recast setup/refresh as bundle deployment

Replace the remaining adapter-oriented refresh/setup posture with bundle
deployment:

- `patina ai setup` deploys the peer interface bundles into the project
- explicit interface launch deploys or refreshes only what it needs
- refresh remains available as an explicit sync/update path

Preserve:

- backup snapshots
- managed markers
- safe overwrite of Patina-owned paths
- preservation of allowed user custom files

### 4. Package core Patina skills per interface

Bundle the current Patina-specific skills and command assets into each
interface deployment in native format.

Near-term target:

- Claude gets native skills and command assets
- OpenCode gets native command assets with equivalent Patina workflows
- Gemini gets native command assets with equivalent Patina workflows

This refactor should not wait for the future universal skill catalog.
It should package the core Patina workflows that exist today.

### 5. Keep Mother thin at launch and session start

The handshake should be small:

- interface asks Mother to launch or check in
- Mother validates project/interface context
- Mother records the runtime event and returns the launch/session
  envelope
- project-local Patina systems own the durable text and artifact writes

This leaves space for richer future handoff logic without bloating
Mother now.

### 6. Unify session entrypoints behind one backend

Keep the human-facing session commands, but move their behavioral truth
behind one backend capability surface. The interface bundle should carry
the UX assets; the backend should carry the real session behavior.

### 7. Add explicit bundle freshness and update commands

Patina should be able to answer:

- which bundles are deployed
- whether they are current
- how to refresh them

This keeps interface surfaces in sync without reintroducing command
drift and stale copied prompt files.

## Implementation Sequence

### Commit 1: `refactor(interface): define Mother-owned interface bundle model`

Introduce typed bundle metadata and move interface support discovery
toward bundle definitions instead of scattered adapter/template logic.

### Commit 2: `refactor(launch): route front door through bundle deployment`

Make `patina` and explicit `patina ai <interface>` launches resolve and
deploy bundles truthfully.

### Commit 3: `refactor(interface): package core Patina skills and commands`

Bundle the current Patina-specific session and belief workflow assets
per interface in native format.

### Commit 4: `refactor(session): keep human session commands, unify backend`

Preserve `/session-start`, `/session-update`, and `/session-end` as the
user-facing flow while converging them onto one canonical backend
capability path.

### Commit 5: `test(interface): verify bundle deployment and refresh`

Add focused tests for:

- front-door routing
- deploy-on-demand
- default-interface launch
- backup and reconciliation
- session command packaging
- truthful MCP/CLI teaching

## Exit Criteria

1. Mother owns a thin runtime catalog of Patina interface bundles for
   Claude, OpenCode, and Gemini and can report which bundles are
   available for deployment.
2. Durable Patina interface truth remains project-owned on the
   `patina` branch, with `layer/` and project files as git-tracked truth
   rather than Mother-only runtime state.
3. `patina ai claude`, `patina ai opencode`, and `patina ai gemini`
   deploy or refresh their managed interface bundle on demand before
   launch when needed.
4. Running `patina` in an existing Patina project launches the project
   default interface, and running `patina` in an empty or non-Patina
   directory routes through a friendly bootstrap into the same bundle
   model.
5. Patina-managed interface paths on the `patina` branch are explicitly
   reconciled, backed up when conflicting, and then replaced with
   Mother's managed surface while preserving allowed user
   customizations.
6. Each interface bundle includes the current Patina-specific core
   skills and command assets needed for session and belief workflow,
   with truthful MCP-first or CLI-JSON fallback guidance.
7. `/session-start`, `/session-update`, and `/session-end` remain
   human-driven interface commands for this phase, with `/session-end`
   performing update then end, while backend behavior converges on one
   canonical session capability surface.
8. Patina exposes an explicit bundle refresh path so interface
   projections can be kept in sync without reintroducing adapter-era
   drift.
