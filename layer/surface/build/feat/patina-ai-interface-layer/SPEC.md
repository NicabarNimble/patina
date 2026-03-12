---
type: feat
id: patina-ai-interface-layer
status: complete
created: 2026-03-11
related:
- agentic-surface-architecture
- knowledge-child-platform
- session-narrative-system
- persona-federation
- mother-maturation
- core-extraction
- spec-subsystem-plugin
beliefs:
- patina-is-domain-agnostic-knowledge-system
- patina-is-beliefs-plus-action
- mother-is-connection-and-continuity
- session-capture
- interfaces-are-not-core
- session-git-trail-is-sacred
- compatibility-paths-buy-trust
sessions:
  origin: 20260310-230611
exit_criteria:
- id: native-ai-command
  text: 'A new native `patina ai` command exists as a parallel front door and does not change existing `patina` launcher behavior'
  checked: true
- id: native-adapter-contract
  text: '`patina ai` defines a clear native adapter contract for interactive interfaces, separate from Mother children and toy SDKs'
  checked: true
- id: opencode-e2e
  text: 'OpenCode works end to end through `patina ai`, including bootstrap, tmux launch, context injection, and session lifecycle hooks'
  checked: true
- id: gemini-same-contract
  text: 'Gemini works through the same `patina ai` adapter contract with minimal adapter-specific logic'
  checked: true
- id: adapters-outside-core
  text: 'Adapter integrations remain outside Patina core in architecture; Patina core and Mother can function without Claude Code, OpenCode, Gemini CLI, or tmux'
  checked: true
- id: mother-checkin-session-attach
  text: '`patina ai` can check in with Mother, attach to the session-narrative-system live session model, and operate without replacing the existing git-backed session audit trail'
  checked: true
- id: compatibility-path-preserved
  text: 'Existing session/git trail behavior remains available and untouched on the compatibility path; the new path projects into compatible review artifacts in layer/sessions or clearly isolated parallel artifacts during migration'
  checked: true
- id: launcher-friendly
  text: 'The launcher UX remains first-run friendly: detect/bootstrap/select interface without exposing plugin/runtime complexity to the user'
  checked: true
- id: headless-room
  text: 'The design explicitly leaves room for future headless agents to use the same Mother/core services without going through a terminal adapter'
  checked: true
---
# feat: Patina AI Interface Layer

> Build a parallel `patina ai` front door for OpenCode first and Gemini
> second, preserving the existing `patina` launcher and git/session trail
> while introducing a cleaner native interface adapter contract over
> Mother-backed live runtime services.

## Problem

Patina now has a stronger internal architecture than its interface
layer:

- the knowledge system is becoming the real core
- Mother / Child / Toy now provides governed execution
- children can be real WASM apps with typed toys

But the interactive front door is still shaped by older assumptions:

- launcher, adapter bootstrap, sessions, tmux, and git capture are
  tightly coupled
- Claude is materially the most complete interface path
- Gemini and OpenCode are thinner and lack a clean build contract
- the current session system is single-active-session and local-first
- adapter logic has no equivalent to the new child SDK model

The risk is twofold:

1. If the interface layer is changed in place, Patina may lose the clean
   `patina` entry experience and the deep git/session trail that users
   rely on.
2. If the interface layer is not rebuilt at all, Patina will have a
   modern runtime underneath an increasingly ad hoc operator surface.

## Goal

Ship a parallel interface path that preserves trust in the current
system while creating a real architecture for interactive AI interfaces.

Target shape:

- `patina` remains the compatibility launcher path
- `patina ai` becomes the experimental and then primary AI front door
- `patina ai` is native, not a plugin
- OpenCode and Gemini integrate through a shared adapter contract
- Mother handles interface check-in, persona attachment, and live
  session attachment for that path
- the existing git-backed session trail remains intact until any new
  projection path proves itself
- Patina core remains usable without any interactive terminal interface

## Non-Goals

This spec does NOT:

- turn OpenCode, Gemini, or Claude Code into Mother children
- replace the existing `patina` launcher/session flow in place
- remove tmux from the user experience
- define the session model itself; that belongs to
  [[session-narrative-system]]
- require multiplayer sessions to be solved completely in v1
- make interactive interfaces part of Patina core

## Solution

### 1. Define the boundary clearly

Treat the system as three layers:

- **Patina core**: beliefs, eventlog, graph, provenance, Mother runtime,
  toys, children
- **interface layer**: launcher UX, tmux, context/bootstrap, adapter
  lifecycle, session hooks
- **audit projection**: git tags, archived session markdown, spec files,
  and related review artifacts

Interactive adapters are not core. They are native integrations over
core.

### 2. Add `patina ai` as a parallel front door

Create a new native command path, likely `patina ai`, that mirrors the
clean user experience of the current launcher while staying isolated
from it:

- detect/bootstrap workspace
- discover available interfaces
- prompt or select an interface
- launch tmux in a consistent way
- attach shared Patina context/session behavior
- check in with Mother for persona/session/runtime attachment

The existing `patina` no-subcommand path remains unchanged except for
shared bug fixes that are clearly safe.

**Explicit implementation target:**

- add a new `Ai` subcommand in `src/main.rs`
- do not repurpose the existing launcher `None => launch` path while the
  new build is proving itself

### 3. Introduce a native adapter contract

Children and adapters are different roles and need different contracts.

The new adapter contract should cover:

- CLI detection
- bootstrap/context generation
- tmux launch behavior
- session lifecycle hook points
- Mother check-in and runtime attachment
- projection to session/spec/git artifacts

This contract should be consumed by `patina ai`. It is a native
abstraction, not a WASM plugin interface.

**Design decision:** do not keep bolting new behavior onto the existing
`LLMAdapter` trait in `src/adapters/mod.rs`. Introduce a separate native
interface contract for the new path so compatibility and experimental
behavior are not forced through one trait.

### 4. Build OpenCode first

OpenCode should be the first full `patina ai` interface because it is
the cleanest proving ground for the new adapter contract.

OpenCode v1 should include:

- interface detection
- project bootstrap
- context injection
- tmux launch
- session start/update/end hook integration
- attachment to the live session model defined by
  [[session-narrative-system]]
- explicit separation between live runtime state and git-backed review
  artifacts

**File targets:**

- `src/main.rs` — new `Ai` subcommand
- new `src/commands/ai/mod.rs`
- new `src/commands/ai/internal.rs`
- new `src/interface/mod.rs` — narrow `AiAdapter` surface
- new `src/interface/internal/`
- `src/adapters/opencode/mod.rs` and/or a new OpenCode bridge module
  that consumes the new interface contract

### 5. Add Gemini on the same contract

Once OpenCode works, Gemini should be added with the same launcher and
adapter lifecycle surfaces. The implementation should prove that the
contract is real rather than OpenCode-shaped.

### 6. Keep session audit trail as a protected compatibility surface

The current git/session trail is too valuable to use as the experimental
surface.

Therefore:

- the existing session flow remains the baseline truth path
- `patina ai` uses the new live session system rather than the old
  singleton file as its primary coordination model
- any new review projection must either:
  - emit compatible archived session artifacts and git markers, or
  - write explicitly separate parallel artifacts until proven

No fake backfilled history is allowed. New capture must always be
truthful.

**Artifact rule:** the durable session output for `patina ai` must land
in `layer/sessions/` once the session model is stable enough. Temporary
parallel artifact locations are only acceptable as migration scaffolding
and must be called out explicitly.

### 7. Preserve a path to headless agents

The architecture should make it possible for a future headless agent to
use Mother/core services without tmux or a CLI adapter. `patina ai`
should therefore be treated as one interface layer, not the definition
of the runtime itself.

## Implementation Sequence

### Commit 1: `feat(ai): add native patina ai entrypoint`

Add a new parallel command tree and routing path without changing the
current launcher behavior.

**File targets:**

- `src/main.rs`
- new `src/commands/ai/mod.rs`
- new `src/commands/ai/internal.rs`

### Commit 2: `feat(ai): define native interface adapter contract`

Extract the shared interface responsibilities into a clear native
contract used by `patina ai`.

**File targets:**

- new `src/interface/mod.rs`
- new `src/interface/internal/checkin.rs`
- new `src/interface/internal/tmux.rs`
- new `src/interface/internal/bootstrap.rs`

### Commit 3: `feat(ai-opencode): implement opencode on patina ai`

Make OpenCode the first fully supported interface on the new path.

**File targets:**

- bridge from `src/adapters/opencode/` into the new interface contract
- preserve existing OpenCode context/bootstrap assets where useful

### Commit 4: `feat(ai-session): attach Mother-backed live runtime context`

Add the minimal Mother integration needed for live AI session/runtime
state while preserving the existing git-backed audit trail.

**File targets:**

- `src/commands/mother/mod.rs`
- `src/commands/mother/daemon.rs`
- `src/mother/mod.rs`
- `src/session/` from [[session-narrative-system]]

### Commit 5: `feat(ai-gemini): implement gemini on shared adapter contract`

Add Gemini using the same contract with minimal bespoke behavior.

**File targets:**

- bridge from `src/adapters/gemini/` into the new interface contract

### Commit 6: `test(ai): verify compatibility and non-interference`

Prove that `patina ai` works and that the legacy `patina` path remains
unchanged.

## Tmux Compatibility Contract

The new path must preserve the current attach semantics:

- reuse the deterministic per-project session naming logic from
  `src/commands/launch/mod.rs::derive_session_name`
- preserve the current `resolve_tmux_decision` rules unless a spec says
  otherwise
- preserve "reattach to the same project tmux session" behavior as a
  product invariant
- a second `patina ai` launch in the same project should attach to the
  existing tmux session rather than creating a parallel one by default

## Verification

- command-level verification that `patina` with no subcommand still
  behaves as before
- command-level verification that `patina ai opencode` launches and
  reattaches correctly
- tests for new interface check-in path
- tests that the new path does not require `.patina/local/active-session.md`
  as the sole live session primitive

## Exit Criteria

1. A new native `patina ai` command exists as a parallel front door and
   does not change existing `patina` launcher behavior
2. `patina ai` defines a clear native adapter contract for interactive
   interfaces, separate from Mother children and toy SDKs
3. OpenCode works end to end through `patina ai`, including bootstrap,
   tmux launch, context injection, and session lifecycle hooks
4. Gemini works through the same `patina ai` adapter contract with
   minimal adapter-specific logic
5. Adapter integrations remain outside Patina core in architecture:
   Patina core and Mother can function without Claude Code, OpenCode,
   Gemini CLI, or tmux
6. `patina ai` can create or attach a Mother-backed live AI
   session/runtime context without replacing the existing git-backed
   session audit trail
7. Existing session/git trail behavior remains available and untouched
   on the compatibility path; the new path projects into compatible
   review artifacts or clearly isolated parallel artifacts
8. The launcher UX remains first-run friendly: detect/bootstrap/select
   interface without exposing plugin/runtime complexity to the user
9. The design explicitly leaves room for future headless agents to use
   the same Mother/core services without going through a terminal
   adapter
