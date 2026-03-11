---
type: refactor
id: canonical-agents-surface
status: complete
created: 2026-03-11
sessions:
  origin: 20260311-135625-KH7V
related:
- /Users/nicabar/Projects/Sandbox/AI/RUST/patina/layer/surface/build/fix/interface-surface-reconciliation/SPEC.md
- /Users/nicabar/Projects/Sandbox/AI/RUST/patina/layer/surface/build/refactor/init-interface-projection-separation/SPEC.md
- /Users/nicabar/Projects/Sandbox/AI/RUST/patina/layer/surface/build/fix/session-surface-parity/SPEC.md
- /Users/nicabar/Projects/Sandbox/AI/RUST/patina/layer/surface/build/feat/patina-ai-interface-layer/SPEC.md
beliefs:
  - patina-identity
  - spec-driven-design
  - dependable-rust
  - unix-philosophy
  - safety-boundaries
  - interfaces-are-not-core
  - compatibility-paths-buy-trust
exit_criteria:
  - id: agents-root-canonical
    text: 'Patina projects use root `AGENTS.md` as the canonical in-project instruction surface for Patina-managed guidance plus user-editable project notes'
    checked: true
  - id: vendor-files-are-shims
    text: 'Vendor root files such as `CLAUDE.md` and `GEMINI.md` are reduced to thin compatibility shims that point to `AGENTS.md` instead of carrying the primary Patina payload'
    checked: true
  - id: no-patina-sidecars
    text: 'Native interface projection no longer generates `.opencode/PATINA.md`, `.gemini/PATINA.md`, or similar `PATINA.md` instruction sidecars'
    checked: true
  - id: runtime-truth-preserved
    text: 'The canonical `AGENTS.md` content still teaches truthful MCP/native session and spec behavior for each runtime without reintroducing hidden fallback semantics'
    checked: true
  - id: setup-reconciliation-updated
    text: 'The setup reconciliation lifecycle works with canonical `AGENTS.md` ownership and still preserves user text, backup snapshots, and `--force` rewrite behavior'
    checked: true
  - id: tests-cover-new-shape
    text: 'Tests cover canonical AGENTS generation, vendor shim generation, sidecar removal, and truthful runtime guidance after the projection reshaping'
    checked: true
---
# refactor: Canonical AGENTS Surface — Root Truth, Vendor Shims, No PATINA Sidecars

> Make root AGENTS.md the canonical Patina instruction surface, reduce vendor root files to compatibility shims, and remove generated PATINA.md sidecars from native interface projection.

## Current State

Patina's native interface projection is cleaner than it was, but the
instruction surface is still split across too many files.

Before this refactor:

- native setup still treated root vendor files as primary surfaces
- adapter-local `PATINA.md` sidecars under `.opencode/` and `.gemini/`
  carried much of the Patina payload
- there was no single canonical root instruction file
- users had to guess whether the real Patina rules lived in
  `AGENTS.md`, `GEMINI.md`, `CLAUDE.md`, `OPENCODE.md`, or `PATINA.md`

That left Patina with an awkward projection model:

- no single canonical root instruction file
- too much generated text hidden in sidecars
- vendor files still feel more authoritative than they should
- users have to guess whether the real Patina rules live in
  `AGENTS.md`, `GEMINI.md`, `CLAUDE.md`, `OPENCODE.md`, or `PATINA.md`

This refactor closes that split:

- root `AGENTS.md` is now the canonical Patina instruction surface for
  native interface projection
- `GEMINI.md` is now a thin compatibility shim that points back to
  `AGENTS.md`
- OpenCode now projects root `AGENTS.md` directly without `OPENCODE.md`
- `.opencode/PATINA.md` and `.gemini/PATINA.md` are no longer generated
- truthful MCP/native fallback teaching now lives in `AGENTS.md`

## Target State

Patina should have one canonical in-project instruction surface:

- root `AGENTS.md`

That file becomes the primary place for:

- Patina-managed workflow teaching
- truthful capability and fallback guidance
- user-editable project instructions outside Patina-managed sections

Vendor-specific root files become compatibility shims only:

- `CLAUDE.md`
- `GEMINI.md`
- any other vendor-required root file Patina still has to project

Those shims should be small and obvious:

- point the runtime to `AGENTS.md`
- carry only minimal vendor-specific notes when required
- avoid duplicating Patina's full instruction payload

Adapter-local `PATINA.md` sidecars should disappear. If runtime-specific
command assets are still needed, they should live in dedicated adapter
command/config files, not in a second hidden instruction document.

## Design Rules

- `AGENTS.md` is canonical; vendor files are projections
- keep one root truth, not many peers
- keep vendor shims thin and readable
- do not move hidden semantics into markdown sidecars
- preserve truthful MCP/native fallback teaching
- preserve setup backup/rewrite safety from the reconciliation spec

## Solution

### 1. Canonicalize root `AGENTS.md`

`patina ai setup` should create or refresh root `AGENTS.md` as the main
Patina-managed instruction surface.

That file should contain:

- a small user-editable area outside Patina markers
- the Patina-managed workflow block
- truthful capability teaching for discovery/session/spec workflow
- explicit fallback rules when MCP is unavailable

### 2. Turn vendor roots into shims

`CLAUDE.md`, `GEMINI.md`, and any other required vendor root files
should be reduced to thin compatibility shims.

They should:

- tell the runtime to read `AGENTS.md`
- optionally include minimal vendor-specific notes
- avoid carrying the main Patina workflow payload

### 3. Remove `PATINA.md` instruction sidecars

The current `.opencode/PATINA.md` and `.gemini/PATINA.md` files are
projection artifacts, not core value.

This refactor should:

- stop generating those files
- move their useful instructional content into `AGENTS.md`
- keep adapter-local directories only for assets that are genuinely
  adapter-local, such as command files or config stubs

### 4. Keep runtime teaching truthful

Moving content into `AGENTS.md` must not make the guidance vaguer.

The canonical root file still has to teach:

- when MCP tools are available
- when they are not
- the correct native machine-readable fallback path
- how session/spec workflow should work for that runtime

If some runtime-specific wording differs, the shim can add a small note,
but the truth contract should remain centralized.

### 5. Keep setup reconciliation intact

The previous reconciliation slice established:

- narrow managed path sets
- backup snapshots
- managed markers
- safe reruns
- `--force` rewrite

This refactor should reuse that lifecycle rather than inventing a new
setup path. The managed surface changes, but the safety model should
stay the same.

## Steps

### Commit 1: `refactor(interface): make AGENTS root canonical`

Add canonical `AGENTS.md` generation and move the main Patina workflow
payload there.

### Commit 2: `refactor(interface): reduce vendor roots to shims`

Rewrite vendor root files to small compatibility shims that point to
`AGENTS.md`.

### Commit 3: `refactor(interface): remove PATINA sidecars`

Stop generating `.opencode/PATINA.md`, `.gemini/PATINA.md`, and any
equivalent instruction sidecars.

### Commit 4: `test(interface): verify canonical projection`

Add focused tests for canonical root generation, shim behavior, sidecar
removal, truthful guidance, and setup reconciliation compatibility.

## Exit Criteria

1. Patina projects use root `AGENTS.md` as the canonical in-project
   instruction surface for Patina-managed guidance plus user-editable
   project notes.
2. Vendor root files such as `CLAUDE.md` and `GEMINI.md` are reduced to
   thin compatibility shims that point to `AGENTS.md` instead of
   carrying the primary Patina payload.
3. Native interface projection no longer generates
   `.opencode/PATINA.md`, `.gemini/PATINA.md`, or similar `PATINA.md`
   instruction sidecars.
4. The canonical `AGENTS.md` content still teaches truthful MCP/native
   session and spec behavior for each runtime without reintroducing
   hidden fallback semantics.
5. The setup reconciliation lifecycle works with canonical `AGENTS.md`
   ownership and still preserves user text, backup snapshots, and
   `--force` rewrite behavior.
6. Tests cover canonical AGENTS generation, vendor shim generation,
   sidecar removal, and truthful runtime guidance after the projection
   reshaping.
