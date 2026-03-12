---
type: fix
id: interface-surface-reconciliation
status: complete
created: 2026-03-11
sessions:
  origin: 20260311-135625-KH7V
related:
- /Users/nicabar/Projects/Sandbox/AI/RUST/patina/layer/surface/build/refactor/init-interface-projection-separation/SPEC.md
- /Users/nicabar/Projects/Sandbox/AI/RUST/patina/layer/surface/build/feat/patina-ai-interface-layer/SPEC.md
- /Users/nicabar/Projects/Sandbox/AI/RUST/patina/layer/surface/build/refactor/cli-mcp-skill-unification/SPEC.md
- /Users/nicabar/Projects/Sandbox/AI/RUST/patina/layer/surface/build/fix/session-surface-parity/SPEC.md
beliefs:
  - patina-identity
  - spec-driven-design
  - dependable-rust
  - unix-philosophy
  - safety-boundaries
  - interfaces-are-not-core
  - compatibility-paths-buy-trust
exit_criteria:
  - id: managed-surface-explicit
    text: '`patina ai setup` manages only a small explicit interface path set and does not recursively sweep arbitrary nested AGENTS.md, GEMINI.md, or CLAUDE.md files'
    checked: true
  - id: snapshot-before-takeover
    text: 'When setup first encounters unmanaged files at Patina-managed interface paths, it snapshots them under `.patina/local/backups/` using the session-style timestamp/uid convention before takeover'
    checked: true
  - id: managed-markers-enable-reruns
    text: 'Patina-generated interface files contain clear managed markers or metadata so reruns can distinguish Patina-owned surfaces from foreign/user-owned files without guessing'
    checked: true
  - id: reruns-are-safe
    text: 'Normal setup reruns refresh Patina-managed content without creating repeated backups or clobbering user-editable content outside managed sections'
    checked: true
  - id: force-resets-safely
    text: '`patina ai setup --force` creates a fresh backup snapshot and rewrites the managed interface surface from current templates'
    checked: true
  - id: tests-cover-lifecycle
    text: 'Targeted tests cover first takeover, rerun refresh, force rewrite, backup placement, and protection against nested-package false positives'
    checked: true
---
# fix: Setup Interface Surface Reconciliation — Backup, Managed Takeover, and Safe Rewrites

> Make patina ai setup snapshot conflicting interface files, replace managed paths with clear Patina-owned surfaces, and refresh them safely on reruns without trampling user intent.

## Problem

Patina's interface setup story is still too easy to distrust.

Today, `patina ai setup` and launch-time projection can create or
refresh interface files, but the user experience around takeover is
still muddy:

- a repo may already contain root `AGENTS.md`, `CLAUDE.md`, or
  `GEMINI.md`
- older Patina-generated interface files can remain after the system
  changes direction
- the operator cannot easily tell which file is canonical, which file
  was injected by Patina, and which file is legacy residue
- a rerun of setup does not yet provide a clear lifecycle contract for
  backup, refresh, or hard reset

That creates two trust failures:

- **surface confusion** — stale or foreign interface files can keep
  steering a tool after Patina setup
- **prompt contamination** — Patina can inherit user/vendor text at a
  managed path without a clear quarantine or ownership boundary

The result is a setup flow that feels invasive when it overwrites and
haunted when it does not.

## Root Cause

The current interface projection model has an ownership boundary, but
not a full lifecycle policy.

Recent specs correctly established that:

- `layer/...` is the real Patina project artifact surface
- interface files are adapter projections, not source-of-truth product
  artifacts
- `patina ai` owns native interface setup/refresh for OpenCode and
  Gemini

But setup still lacks a strong answer to five practical questions:

1. Which exact paths does Patina own?
2. What happens when those paths already contain foreign content?
3. Where are backups stored, and are they committed?
4. How does Patina know a rerun is refreshing its own surface rather
   than trampling user content?
5. How does the operator ask for a hard reset?

Without explicit answers, setup behavior is partly conventional and
partly historical, which violates [[spec-driven-design]] and weakens
the safety boundary around project-local prompt surfaces.

## Fix

Define a narrow, explicit interface-surface reconciliation policy for
`patina ai setup`.

### 1. Manage a small explicit surface, not filename-shaped chaos

Setup should inventory only a known path set per adapter/runtime
surface. Examples:

- root `AGENTS.md`
- root `CLAUDE.md`
- root `GEMINI.md`
- Patina-managed adapter directories such as `.opencode/...`,
  `.gemini/...`, and `.claude/...`

It should **not** recursively search the entire repository for every
matching filename. That would collide with legitimate package-level
agent files in monorepos and violate the narrow-boundary principle.

### 2. Snapshot unmanaged managed-path conflicts before takeover

If setup encounters an unmanaged or foreign file at a Patina-managed
path, it should archive the conflicting surface before writing its own
version.

Backups belong under:

- `.patina/local/backups/interface/`

using the same timestamp/uid style Patina already uses for session
identity, for example:

- `patina-setup-backup-20260311-201500-ABCD/`

This keeps backups:

- project-local
- clearly non-canonical
- automatically gitignored under `.patina/local/`

### 3. Replace active managed paths with clearly Patina-owned surfaces

After snapshotting, setup should write fresh active interface files
using current templates.

Those files must contain clear Patina ownership markers so future runs
can recognize them deterministically. The exact marker syntax can vary
by file shape, but the semantics must be:

- this file or section is Patina-managed
- setup may refresh it on rerun
- user text outside the managed boundary is user-owned

### 4. Make reruns refresh, not re-takeover

Normal `patina ai setup` reruns should:

- detect Patina-managed files via markers/metadata
- refresh only managed content
- preserve user-editable content outside managed blocks
- avoid creating redundant backup archives on every run

This is the stable maintenance path.

### 5. Add an explicit hard-reset mode

`patina ai setup --force` should mean:

- create a fresh backup snapshot from the current managed path set
- rewrite the managed interface surface from current templates
- treat the operation as a deliberate re-takeover/hard reset

This is the operator's escape hatch when the interface surface has
become suspect.

### 6. Keep cleanup truthful and bounded

This spec is not permission to delete arbitrary nested interface files.

Allowed cleanup scope:

- root files Patina explicitly owns
- Patina-owned adapter directories
- clearly stale Patina-generated files within those managed paths

Disallowed cleanup scope:

- package-level upstream `AGENTS.md` / `GEMINI.md` files not owned by
  Patina
- arbitrary vendor docs discovered by filename only

### 7. Test the lifecycle directly

The implementation should be verified with targeted tests that prove:

- first-time takeover archives foreign content before rewrite
- reruns skip new backup creation for already-managed surfaces
- user text outside managed sections survives normal reruns
- `--force` creates a new archive and rewrites the surface
- nested package-level agent files are ignored unless explicitly in the
  managed path set

## Command Shape

The likely user-facing shape is:

- `patina ai setup <adapter>`
- `patina ai setup <adapter> --force`

The adapter-specific path set can vary, but the lifecycle policy should
be shared through one typed reconciliation seam rather than separate
prompt/template hacks per interface.

## Exit Criteria

1. `patina ai setup` manages only a small explicit interface path set
   and does not recursively sweep arbitrary nested `AGENTS.md`,
   `GEMINI.md`, or `CLAUDE.md` files.
2. When setup first encounters unmanaged files at Patina-managed
   interface paths, it snapshots them under `.patina/local/backups/`
   using the session-style timestamp/uid convention before takeover.
3. Patina-generated interface files contain clear managed markers or
   metadata so reruns can distinguish Patina-owned surfaces from
   foreign/user-owned files without guessing.
4. Normal setup reruns refresh Patina-managed content without creating
   repeated backups or clobbering user-editable content outside managed
   sections.
5. `patina ai setup --force` creates a fresh backup snapshot and rewrites
   the managed interface surface from current templates.
6. Targeted tests cover first takeover, rerun refresh, force rewrite,
   backup placement, and protection against nested-package false
   positives.
