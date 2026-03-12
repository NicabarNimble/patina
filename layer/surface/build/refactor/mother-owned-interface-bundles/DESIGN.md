# Design: refactor: Mother-Owned Interface Bundles

## Approach

Keep the project as the durable island and Mother as the thin runtime
coordinator.

The implementation should introduce a typed interface bundle seam that
Mother and the interface layer both use:

- bundle definition says what Patina owns for `claude`, `opencode`, and
  `gemini`
- setup and launch deploy bundles into the project
- bundle deployment reuses existing managed-surface reconciliation and
  backup behavior
- session and belief workflow assets remain bundled per interface in
  native format
- the real behavioral truth for sessions moves behind one canonical
  backend capability surface

Do not build the future global skill catalog in this slice. Package the
current core Patina workflows first and leave catalog generalization for
the later skill-capability unification work.

## Commits
1. `refactor(interface): define Mother-owned bundle catalog` — add typed
   bundle metadata and centralize interface support discovery
2. `refactor(launch): deploy bundles on setup and launch` — route
   `patina`, `patina ai setup`, and explicit interface launch through
   bundle deployment
3. `refactor(interface): package session and belief workflows` — move
   core Patina-specific command/skill assets into the bundle model for
   all three interfaces
4. `refactor(session): preserve human commands, thin backend seam` —
   keep `/session-start`, `/session-update`, and `/session-end` while
   converging their backend behavior
5. `test(interface): cover bundle freshness and deployment` — verify
   front door, default launch, refresh, backup, and truthful runtime
   guidance

## Key Files
- [src/interface/internal/surface.rs](/Users/nicabar/Projects/Sandbox/AI/RUST/patina/src/interface/internal/surface.rs) —
  current peer-interface preparation seam; likely home for bundle
  resolution
- [src/interface/internal/bootstrap.rs](/Users/nicabar/Projects/Sandbox/AI/RUST/patina/src/interface/internal/bootstrap.rs) —
  managed path reconciliation, backup, and projection safety
- [src/commands/ai/surface.rs](/Users/nicabar/Projects/Sandbox/AI/RUST/patina/src/commands/ai/surface.rs) —
  explicit interface launch path
- [src/commands/launch/internal.rs](/Users/nicabar/Projects/Sandbox/AI/RUST/patina/src/commands/launch/internal.rs) —
  no-subcommand front door and bootstrap flow
- [src/project/internal.rs](/Users/nicabar/Projects/Sandbox/AI/RUST/patina/src/project/internal.rs) —
  project detection and project-local config/backup seams
- [src/adapters/templates.rs](/Users/nicabar/Projects/Sandbox/AI/RUST/patina/src/adapters/templates.rs) —
  old adapter-era template shipping logic to be mined or narrowed
- [resources/interfaces/code/AGENTS.md](/Users/nicabar/Projects/Sandbox/AI/RUST/patina/resources/interfaces/code/AGENTS.md) —
  shared Patina runtime teaching included in bundles
- [resources/claude/skills/epistemic-beliefs/SKILL.md](/Users/nicabar/Projects/Sandbox/AI/RUST/patina/resources/claude/skills/epistemic-beliefs/SKILL.md) —
  current core skill example that should inform cross-interface workflow
  packaging
- [src/commands/session/internal.rs](/Users/nicabar/Projects/Sandbox/AI/RUST/patina/src/commands/session/internal.rs) —
  compatibility-heavy backend that should be wrapped or narrowed, not
  re-expanded in interface files

## Open Questions
- should Patina project detection stay `.patina/`-based for this slice,
  or should `layer/` and/or a stronger project marker become part of the
  detection contract now?
- should bundle freshness be version-based, hash-based, or marker-based?
- how much of the old `adapter` command should survive as compatibility
  shim versus being moved entirely under `patina ai`?
