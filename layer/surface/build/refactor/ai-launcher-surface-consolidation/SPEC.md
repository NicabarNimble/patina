---
type: refactor
id: ai-launcher-surface-consolidation
status: complete
created: 2026-03-11
sessions:
  origin: 20260311-135625-KH7V
related:
  - /Users/nicabar/Projects/Sandbox/AI/RUST/patina/layer/surface/build/feat/patina-ai-interface-layer/SPEC.md
  - /Users/nicabar/Projects/Sandbox/AI/RUST/patina/layer/surface/build/refactor/canonical-agents-surface/SPEC.md
  - /Users/nicabar/Projects/Sandbox/AI/RUST/patina/layer/surface/build/refactor/interface-setup-generalization/SPEC.md
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
  - id: single-ai-setup-command
    text: '`patina ai setup` becomes the one project-local AI interface setup command; it prepares the full Patina AI surface instead of requiring per-interface setup commands'
    checked: true
  - id: patina-ai-launches-three-peers
    text: '`patina ai claude`, `patina ai opencode`, and `patina ai gemini` exist as peer launch commands on one interface layer'
    checked: true
  - id: claude-joins-interface-layer
    text: 'Claude Code is brought onto the Patina AI interface layer instead of living behind a separate compatibility-only launcher/setup model'
    checked: true
  - id: default-selection-on-launch
    text: '`patina ai <interface> --default` sets the project default interface, and first successful setup/selection establishes a default automatically'
    checked: true
  - id: patina-remains-friendly-entry
    text: 'Running `patina` in a new or empty directory still provides the friendly guided entry into the system, but now routes into the consolidated Patina AI model truthfully'
    checked: true
  - id: old-command-drift-removed
    text: 'The temporary split between `patina interface setup`, `patina ai setup`, `adapter add`, and native-vs-compatibility launcher semantics is removed or reduced to narrow compatibility shims'
    checked: true
  - id: setup-backup-truth-preserved
    text: 'Backup snapshots, managed markers, truthful MCP/native fallback teaching, and project-scoped safety boundaries remain intact after the launcher/setup cleanup'
    checked: true
  - id: tests-cover-consolidated-model
    text: 'Tests cover the consolidated setup flow, Claude/OpenCode/Gemini launch parity, default selection, and friendly no-project entry behavior'
    checked: true
---
# refactor: AI Launcher Surface Consolidation

> Collapse the current transitional AI interface surfaces into one clean
> product model: `patina ai` is the real launcher, `patina ai setup` is
> the one setup command, Claude/OpenCode/Gemini are peers, and `patina`
> remains the friendly front door into that model.

## Current State

Patina's recent interface work proved the new direction, but it also
left multiple overlapping user models alive at once.

Today, the user can encounter all of these:

- `patina` as the historical launcher/front door
- `patina ai` as the newer native launcher path
- `patina interface setup` as the newer generic setup command
- `patina ai setup` as a compatibility alias
- `patina adapter add` as a separate allowlist/configuration path
- Claude Code living on a materially different path from OpenCode and
  Gemini

The code reflects this split:

- `src/commands/ai/mod.rs` and `src/commands/ai/internal.rs` own native
  launch/session behavior
- `src/commands/interface/mod.rs` and `src/commands/interface/internal.rs`
  now own setup
- `src/interface/mod.rs` only exposes OpenCode and Gemini as native
  interfaces
- `src/commands/adapter.rs` still carries overlapping responsibilities
  for project allowlisting and projection setup
- `src/commands/init/internal/mod.rs` still teaches adapter-era language
  and a transitional setup story

This is safe, but it is no longer clean.

At the product level, the problem is simple:

- too many names for the same lifecycle
- too many temporary seams exposed to the user
- Claude not yet treated as a first-class peer
- a helpful first-run experience that still speaks in older adapter
  terms

This violates the spirit of dependable-rust and unix-philosophy at the
user surface: too many commands are sharing one job.

## Target State

Patina should converge on one clean AI interface model:

- `patina ai` is the real launcher surface
- `patina ai setup` is the one setup command for the Patina AI surface
- supported interfaces are exactly:
  - `claude`
  - `opencode`
  - `gemini`
- those three interfaces are peers on one interface layer
- `patina` remains the friendly front door and launcher convenience
  path, not a competing second setup model

### User-facing command model

Setup:

- `patina ai setup`
- `patina ai setup --force`

Launch:

- `patina ai claude`
- `patina ai opencode`
- `patina ai gemini`

Default selection:

- `patina ai claude --default`
- `patina ai opencode --default`
- `patina ai gemini --default`

Entry behavior:

- running `patina` in a new or empty directory still gives the guided
  bootstrap experience
- that guided flow performs the new truth:
  - initialize Patina core
  - set up the Patina AI surface once
  - choose the first/default interface
  - launch it if appropriate

### Product rules

- there is no per-interface setup command
- there is no long-term `patina interface setup` user-facing surface
- there is no need for users to think in terms of "native vs
  compatibility" for Claude/OpenCode/Gemini
- all three interfaces use one setup story and one launch story
- setup still preserves backups, markers, and truthful capability
  teaching

## Non-Goals

This refactor does NOT:

- collapse interactive interfaces into Patina core
- remove the project-local backup/marker safety model
- remove truthful MCP/native fallback teaching
- require headless/non-terminal interfaces to be solved now
- solve every adapter global config problem in home directories

## Design Rules

- `patina ai` is the product surface for interactive code interfaces
- `patina` remains a convenience/front-door surface, not a second
  competing lifecycle
- one setup command, one launch family, three peers
- Claude must join the same interface model instead of remaining a
  special historical island
- user-facing command count should go down, not up
- setup remains project-scoped and explicit about side effects
- compatibility shims may exist internally during migration, but they
  must stop being the documented product model

## Solution

### 1. Collapse setup to `patina ai setup`

Make `patina ai setup` the only documented setup command for the AI
surface.

It should:

- prepare the Patina AI surface for the current project
- reconcile managed files and snapshots
- write/update canonical root instruction assets
- project any required per-interface command assets
- prepare the supported interface set without requiring a per-interface
  setup invocation

`--force` should remain the hard-reset path:

- create a fresh backup snapshot
- rewrite managed files from current templates

### 2. Make Claude a peer in the Patina AI interface layer

Claude should stop living behind a materially different launcher/setup
story.

This does not mean pretending Claude is identical internally. It means
the user-facing model converges:

- `patina ai claude`
- `patina ai opencode`
- `patina ai gemini`

All three should share:

- one launcher family
- one default-selection model
- one setup lifecycle
- one concept of interface projection ownership

### 3. Move default selection to launch intent

Project default should be set from the launch surface:

- `patina ai <interface> --default`

Rules:

- first successful setup or first guided selection establishes a default
  automatically
- later `--default` explicitly changes it
- setup itself should not need per-interface arguments just to establish
  default state

### 4. Keep `patina` as the friendly entry

The existing no-project / empty-directory experience is valuable and
should be preserved.

But it should route into the cleaned-up model:

- detect that the user is not yet in a Patina project
- offer guided bootstrap
- initialize the Patina core project skeleton
- run the one AI setup path
- let the user choose `claude`, `opencode`, or `gemini`
- mark that choice as default
- optionally launch it immediately

This keeps the welcoming entry behavior without keeping the current
surface sprawl.

### 5. Reduce transitional command drift

This cleanup must explicitly retire or narrow the transitional seams
that are now cluttering the model:

- `patina interface setup`
- setup guidance that still talks in old adapter/native split language
- separate mental models for Claude versus OpenCode/Gemini
- older `adapter add` responsibilities that overlap with AI setup

Compatibility may remain briefly in code, but the documented and taught
surface should become singular.

### 6. Preserve truth and safety

This cleanup is only acceptable if it preserves the guarantees won in
the recent slices:

- truthful MCP/native fallback guidance
- canonical `AGENTS.md`-style teaching where appropriate
- explicit project-local backup snapshots
- managed marker boundaries
- no silent downgrade to legacy session semantics
- project-scoped safety boundaries and user trust

## Implementation Sequence

### Commit 1: `refactor(ai): collapse setup to one surface`

Rework setup so `patina ai setup` is the single product setup path and
retire `patina interface setup` from the taught surface.

### Commit 2: `refactor(ai): unify claude opencode gemini launch model`

Bring Claude into the same Patina AI interface family as OpenCode and
Gemini for launcher/default-selection behavior.

### Commit 3: `refactor(ai): restore friendly patina entry`

Update the no-project / empty-directory entry flow so `patina` provides
the guided bootstrap into the new consolidated AI model.

### Commit 4: `refactor(ai): remove stale command drift`

Clean up outdated setup/help/guidance strings and shrink transitional
compatibility seams.

### Commit 5: `test(ai): verify consolidated launcher model`

Add focused tests for:

- single setup flow
- three-interface launch parity
- default selection
- friendly first-run entry
- preserved safety/truth guarantees

## Exit Criteria

1. `patina ai setup` becomes the one project-local AI interface setup
   command; it prepares the full Patina AI surface instead of requiring
   per-interface setup commands.
2. `patina ai claude`, `patina ai opencode`, and `patina ai gemini`
   exist as peer launch commands on one interface layer.
3. Claude Code is brought onto the Patina AI interface layer instead of
   living behind a separate compatibility-only launcher/setup model.
4. `patina ai <interface> --default` sets the project default
   interface, and first successful setup/selection establishes a default
   automatically.
5. Running `patina` in a new or empty directory still provides the
   friendly guided entry into the system, but now routes into the
   consolidated Patina AI model truthfully.
6. The temporary split between `patina interface setup`,
   `patina ai setup`, `adapter add`, and native-vs-compatibility
   launcher semantics is removed or reduced to narrow compatibility
   shims.
7. Backup snapshots, managed markers, truthful MCP/native fallback
   teaching, and project-scoped safety boundaries remain intact after
   the launcher/setup cleanup.
8. Tests cover the consolidated setup flow, Claude/OpenCode/Gemini
   launch parity, default selection, and friendly no-project entry
   behavior.
