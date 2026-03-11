# Design: Init and AI Interface Projection Separation

## Purpose

This refactor makes the ownership boundary operational instead of
implicit:

- `patina init .` should scaffold Patina core
- `patina ai` should own native interface projection lifecycle

The immediate driver is stale OpenCode/Gemini projection drift, but the
deeper goal is to stop treating `init` as the place where interface
integration hacks accumulate.

## Design Position

Patina has two different responsibilities that should not be fused:

1. **Core scaffold**
   Project identity, `.patina/`, `layer/`, recipe/config, environment
   capture, navigation.
2. **Interface projection**
   Adapter-specific command files, bootstrap content, generated context,
   launch preparation.

`init` should own (1).
`patina ai` should own (2) for native interfaces.

Claude compatibility is the deliberate exception for now because the old
trusted path still matters.

## Approach

- Shrink `init` scope in code and in user-facing help text.
- Add an explicit `patina ai setup <adapter>` or equivalent refresh path.
- Make launch call the same projection function instead of a special
  bootstrap-only branch.
- Separate user-owned files from Patina-managed generated files so
  refresh is safe.
- Keep OpenCode and Gemini on the new path; leave Claude compatibility
  stable on the old path.

## File Ownership Model

### Core-owned

- `.patina/config.toml`
- `.patina/uid`
- `.patina/oxidize.yaml`
- `.patina/versions.json`
- `layer/`
- `ENVIRONMENT.toml`

### AI-owned (native path)

- `.opencode/commands/*`
- `.gemini/commands/*`
- generated Patina context fragments
- Patina-managed sections of bootstrap files

### User-owned

- top-level user context shells
- local settings/custom files
- any non-Patina-managed sections of bootstrap files

## Projection Policy

The system needs one shared projection function with modes, not several
commands doing nearly the same thing.

Suggested modes:

- `CreateMissing`
- `RefreshManaged`
- `RebuildManaged`

Callers:

- `patina ai setup <adapter>` → `RefreshManaged`
- `patina ai <adapter>` → ensure projection via the same path, then
  launch
- adapter refresh commands can delegate or become compatibility wrappers

## Why Not Put This In `init`

Because `init` is run for project bootstrap and re-bootstrap, not for
frontend lifecycle.

If `init` owns interface projection:

- re-init gains surprising side effects on user-facing files
- adapter drift becomes harder to reason about
- native interface lifecycle remains coupled to project skeleton

That violates the architecture already established in
`patina-ai-interface-layer`.

## Commits

1. `refactor(init): make init explicitly core-only`
   Remove OpenCode/Gemini ownership expectations from init/rebuild.
2. `feat(ai): add native setup or refresh command`
   Give `patina ai` a first-class projection lifecycle entrypoint.
3. `refactor(interface): centralize setup and launch projection`
   One shared projection function for setup + launch.
4. `refactor(opencode-gemini): separate generated and user-owned files`
   Make refresh safe and predictable.
5. `test(ai): verify ownership boundaries`
   Lock the boundary in with focused tests.

## Key Files

- `src/commands/init/internal/mod.rs` — current init ownership boundary
- `src/commands/rebuild/mod.rs` — confirms rebuild is data-only today
- `src/commands/ai/mod.rs` — natural home for explicit setup command
- `src/commands/ai/internal.rs` — current launch path that should reuse shared setup
- `src/interface/internal/bootstrap.rs` — current native setup seam
- `src/adapters/opencode/internal/mod.rs` — OpenCode generated context behavior
- `src/adapters/gemini/internal/mod.rs` — Gemini generated context behavior
- `src/adapters/launch.rs` — marker-based bootstrap generation

## Resolved In This Slice

- Claude remains on the compatibility path; no forced migration into
  native `patina ai` ownership happened here.
- Legacy `adapter add/refresh` now delegates OpenCode/Gemini setup to
  the same native projection seam instead of owning separate refresh
  logic.
- The generated/user-owned split is:
  - generated `PATINA.md` fragments under `.opencode/` and `.gemini/`
  - marker-scoped refresh inside `AGENTS.md` / `GEMINI.md` when Patina
    already owns a section
  - full preservation of user shells without Patina markers
