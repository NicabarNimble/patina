---
type: refactor
id: interface-setup-generalization
status: complete
created: 2026-03-11
sessions:
  origin: 20260311-135625-KH7V
related:
  - /Users/nicabar/Projects/Sandbox/AI/RUST/patina/layer/surface/build/refactor/canonical-agents-surface/SPEC.md
  - /Users/nicabar/Projects/Sandbox/AI/RUST/patina/layer/surface/build/fix/interface-surface-reconciliation/SPEC.md
  - /Users/nicabar/Projects/Sandbox/AI/RUST/patina/layer/surface/build/fix/session-surface-parity/SPEC.md
beliefs:
  - patina-identity
  - spec-driven-design
  - dependable-rust
  - unix-philosophy
  - safety-boundaries
  - interfaces-are-not-core
exit_criteria:
  - id: interface-command-exists
    text: '`patina interface setup <name>` exists as the general project-local interface projection command, while `patina ai setup <name>` remains a compatibility alias for current AI/code interfaces'
    checked: true
  - id: file-backed-code-interface-assets
    text: 'Canonical `AGENTS.md` and vendor shim templates move out of Rust string builders into `resources/interfaces/code/` assets with runtime loading and embedded fallback'
    checked: true
  - id: setup-stays-typed
    text: 'The implementation keeps one typed setup seam for current code interfaces instead of reintroducing stringly hidden behavior in prompts or shell scripts'
    checked: true
  - id: truthful-runtime-rendering
    text: 'Rendered interface files still inject truthful MCP/native fallback guidance and `layer`/`patina --help` entry points at setup time'
    checked: true
  - id: init-remains-core-only
    text: '`patina init` help and guidance stay core-only, and setup guidance now points at `patina interface setup` with `patina ai setup` documented as compatibility'
    checked: true
  - id: tests-cover-command-and-assets
    text: 'Tests cover the new command path, file-backed asset rendering, and compatibility alias behavior'
    checked: true
---
# refactor: interface-setup-generalization

## Current State

Patina has started to separate core setup from interface projection, but
the command and asset boundaries are still mid-transition.

Today:

- `patina init` is core-only in spirit, but project guidance still points
  users toward `patina ai setup`
- `patina ai setup` is the only command entrypoint for native interface
  projection
- the actual root-interface payload for canonical `AGENTS.md` and vendor
  shims is still assembled inside Rust string builders
- the resource tree still reflects adapter-era organization
  (`resources/opencode`, `resources/gemini`, `resources/claude`)
  instead of a clearer interface-assets model

That leaves Patina with a muddled story:

- interface projection is still named as if it were only an AI concern
- changing root instruction text requires editing Rust logic
- there is no clean file-backed source of truth for code-interface root
  assets
- future interface families would have to copy the `ai setup` pattern
  instead of fitting into a clearer `interface setup` command model

## Target State

Patina should present a clearer split:

- `patina init` handles core project setup only
- `patina interface setup <name>` handles project-local interface
  projection
- `patina ai setup <name>` remains as a compatibility alias for the
  currently supported AI/code interfaces

For current code interfaces:

- root instruction assets live under `resources/interfaces/code/`
- setup loads those files at runtime when available
- setup still has an embedded fallback for installed binaries that do
  not have the source tree present
- setup injects truthful runtime capability sections at render time

This keeps the interface surface editable as real files while preserving
typed Rust ownership of capability truth and managed-path behavior.

This refactor is now implemented for the current native code interfaces:

- `patina interface setup <name>` is the general project-local setup path
- `patina ai setup <name>` remains a compatibility alias
- canonical `AGENTS.md` and vendor shims are loaded from
  `resources/interfaces/code/` with embedded fallback
- current setup guidance in `init` now points at `patina interface setup`

## Design Rules

- core setup and interface setup are separate responsibilities
- `interface setup` is the generic command shape; `ai setup` is a
  compatibility alias, not the long-term abstraction
- interface assets are files in `resources/`, not large Rust string
  builders
- capability truth stays in typed Rust and is rendered into those assets
- project-local reconciliation, backup snapshots, and `--force`
  semantics from the prior slice remain intact

## Steps

### Commit 1: `refactor(interface): add general setup command`

Add `patina interface setup <name>` and route it through the same typed
projection path used by current native code interfaces.

### Commit 2: `refactor(interface): externalize code interface assets`

Move canonical `AGENTS.md` and vendor shim templates into
`resources/interfaces/code/` and render them through setup with runtime
loading plus embedded fallback.

### Commit 3: `refactor(interface): align setup guidance`

Update `init` and related guidance so Patina consistently describes
`init` as core-only and `interface setup` as the project-local interface
projection command, while preserving `ai setup` compatibility.

### Commit 4: `test(interface): verify setup generalization`

Add or refresh focused tests for the new command path, compatibility
alias, and file-backed asset rendering.

## Exit Criteria

1. `patina interface setup <name>` exists as the general project-local
   interface projection command, while `patina ai setup <name>` remains
   a compatibility alias for current AI/code interfaces.
2. Canonical `AGENTS.md` and vendor shim templates move out of Rust
   string builders into `resources/interfaces/code/` assets with runtime
   loading and embedded fallback.
3. The implementation keeps one typed setup seam for current code
   interfaces instead of reintroducing stringly hidden behavior in
   prompts or shell scripts.
4. Rendered interface files still inject truthful MCP/native fallback
   guidance and `layer`/`patina --help` entry points at setup time.
5. `patina init` help and guidance stay core-only, and setup guidance
   now points at `patina interface setup` with `patina ai setup`
   documented as compatibility.
6. Tests cover the new command path, file-backed asset rendering, and
   compatibility alias behavior.
