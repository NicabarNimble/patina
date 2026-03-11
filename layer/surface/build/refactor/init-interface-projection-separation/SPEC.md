---
type: refactor
id: init-interface-projection-separation
status: complete
created: 2026-03-11
sessions:
  origin: 20260311-125452-QUHF
related:
  - /Users/nicabar/Projects/Sandbox/AI/RUST/patina/layer/surface/build/feat/patina-ai-interface-layer/SPEC.md
  - /Users/nicabar/Projects/Sandbox/AI/RUST/patina/layer/surface/build/refactor/cli-mcp-skill-unification/SPEC.md
  - /Users/nicabar/Projects/Sandbox/AI/RUST/patina/layer/surface/build/feat/opencode-session-spec-capabilities/SPEC.md
  - /Users/nicabar/Projects/Sandbox/AI/RUST/patina/layer/surface/build/refactor/agentic-surface-architecture/SPEC.md
beliefs:
  - patina-identity
  - interfaces-are-not-core
  - dependable-rust
  - unix-philosophy
  - compatibility-paths-buy-trust
exit_criteria:
  - id: init-core-only
    text: '`patina init .` is reduced to core Patina scaffold responsibilities and no longer owns OpenCode or Gemini interface projection'
    checked: true
  - id: ai-setup-command-exists
    text: '`patina ai` has a first-class setup/refresh path for native interface projection without requiring launcher side effects'
    checked: true
  - id: opencode-gemini-owned-by-ai
    text: 'OpenCode and Gemini project files are installed and refreshed from the `patina ai` path rather than from init/rebuild'
    checked: true
  - id: claude-compatibility-preserved
    text: 'The trusted Claude compatibility path remains available and is not broken by the init boundary change'
    checked: true
  - id: user-files-preserved
    text: 'User-owned interface files are preserved while Patina-managed projection files can be safely regenerated'
    checked: true
  - id: projection-lifecycle-centralized
    text: 'Launch/setup/refresh flows use one shared projection function instead of drifting per-command logic'
    checked: true
---
# refactor: Init and AI Interface Projection Separation

> Make `patina init .` core-only scaffolding, move OpenCode/Gemini interface projection ownership under `patina ai`, and preserve Claude compatibility on the trusted path.

## Current State

Patina now has two overlapping setup stories:

- `patina init .` creates the core project scaffold
- adapter commands and `patina ai` also create interface projection

But the ownership boundary is still muddy.

Today:

- `patina init .` is intended to be core-only bootstrap
- `patina rebuild` only rebuilds scrape/oxidize data under `.patina/local`
- OpenCode and Gemini projection files live under adapter/template paths
- `patina ai` launches those interfaces through the new native contract

The problem is that setup and refresh behavior for interface files is
still distributed across older adapter-era mechanisms:

- existing `.opencode/` or `.gemini/` files can remain stale
- `patina ai` launch may not refresh all generated files
- `init` does not own those interfaces but users can still reasonably
  expect it to bring the project into a coherent state
- user-owned and Patina-generated files are not separated clearly enough

This creates drift:

- Patina core may be current
- the AI interface projection may be stale
- the operator cannot tell which command is responsible for fixing it

## Target State

The responsibility boundary becomes explicit:

- `patina init .` owns only core Patina scaffold and project identity
- `patina ai` owns native interface projection lifecycle
- OpenCode and Gemini are fully managed from the native interface layer
- Claude compatibility remains on the trusted old path until
  intentionally migrated

That means:

- `init` creates `.patina/`, `layer/`, recipe/config, environment
  capture, navigation, and related core artifacts
- `init` does not install or refresh `.opencode/` or `.gemini/`
- `rebuild` remains a data/projection rebuild for scrape + oxidize, not
  an interface refresh tool
- `patina ai` gets a first-class setup/refresh command for interface
  projection
- launch uses the same shared projection path as explicit setup

## Design Rules

- Keep `init` boring and dependable
- Keep interfaces outside core ownership
- Separate user-owned files from Patina-generated files
- Preserve the trusted Claude compatibility path during migration
- Prefer one shared projection function over command-specific copies

## Solution

### 1. Freeze `init` to core Patina scaffold

`patina init .` should explicitly own:

- `.patina/config.toml`
- `.patina/uid`
- `.patina/oxidize.yaml`
- `.patina/versions.json`
- `layer/`
- `ENVIRONMENT.toml`
- minimal Mother-aware project registration if needed later

It should explicitly not own:

- `.opencode/`
- `.gemini/`
- OpenCode/Gemini bootstrap files
- OpenCode/Gemini command templates

### 2. Add native interface setup under `patina ai`

`patina ai` should gain a setup/refresh command that can be called
without launching the interface.

Examples:

- `patina ai setup opencode`
- `patina ai setup gemini`

This becomes the authoritative way to:

- install missing native interface projection
- refresh stale Patina-managed interface files
- prepare a project for later `patina ai opencode` /
  `patina ai gemini`

### 3. Make launch call the same projection path

`patina ai opencode` and `patina ai gemini` should still be
launcher-friendly:

- if setup is missing, they repair it
- if setup is stale, they can refresh Patina-managed projection

But that behavior must call the same shared setup/projection function as
the explicit `patina ai setup` path.

### 4. Separate user-owned files from generated files

The current drift problem is worsened by treating mixed files as if
Patina owns the whole thing.

The new model should separate:

- **user-owned files** — never overwritten silently
- **Patina-generated files** — safe to regenerate
- **mixed files with Patina sections** — update only within markers

For OpenCode and Gemini, that likely means:

- generated Patina context file separate from user context shell, or
- marker-scoped regeneration inside bootstrap files

### 5. Preserve Claude compatibility as a deliberate exception

The old `patina` compatibility path still matters and Claude is the most
trusted surface on that path.

So this refactor should not force Claude onto the native `patina ai`
ownership model immediately.

Rule for this slice:

- OpenCode + Gemini move to `patina ai` ownership
- Claude compatibility remains available and working
- any future Claude migration should be explicit and separately specced

## Steps

### Commit 1: `refactor(init): make init explicitly core-only`

Clarify and enforce that `patina init .` does not own OpenCode/Gemini
projection.

Targets:

- `src/commands/init/*`
- help text and user guidance

### Commit 2: `feat(ai): add native setup/refresh command`

Add a first-class `patina ai` command for interface setup/refresh.

Targets:

- `src/commands/ai/*`
- `src/interface/*`

### Commit 3: `refactor(interface): centralize projection lifecycle`

Move launch/setup/refresh onto one shared projection function.

Targets:

- `src/interface/internal/bootstrap.rs`
- `src/adapters/templates.rs`
- relevant adapter bridge files

### Commit 4: `refactor(opencode-gemini): separate generated and user-owned files`

Ensure OpenCode and Gemini can be refreshed safely without breaking user
customization.

Targets:

- `src/adapters/opencode/*`
- `src/adapters/gemini/*`
- `resources/opencode/*`
- `resources/gemini/*`

### Commit 5: `test(ai): verify setup and launch ownership`

Add focused tests and command-level verification around ownership
boundaries.

Targets:

- `init` does not project OpenCode/Gemini
- `patina ai setup <adapter>` creates/refreshes managed files
- `patina ai <adapter>` uses the same projection path
- Claude compatibility path still works

## Exit Criteria

1. `patina init .` is reduced to core Patina scaffold responsibilities
   and no longer owns OpenCode or Gemini interface projection.
2. `patina ai` has a first-class setup/refresh path for native
   interface projection without requiring launcher side effects.
3. OpenCode and Gemini project files are installed and refreshed from
   the `patina ai` path rather than from init/rebuild.
4. The trusted Claude compatibility path remains available and is not
   broken by the init boundary change.
5. User-owned interface files are preserved while Patina-managed
   projection files can be safely regenerated.
6. Launch/setup/refresh flows use one shared projection function instead
   of drifting per-command logic.
