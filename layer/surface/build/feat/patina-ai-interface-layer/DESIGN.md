# Design: Patina AI Interface Layer

## Why This Exists

Patina’s runtime architecture is moving in the right direction:

- beliefs are the real core
- Mother is the native authority/runtime
- children are governed apps with toys

But the interface story is still older and less explicit. The current
launcher is valuable precisely because it feels simple:

- type `patina`
- bootstrap if needed
- choose an interface
- land in tmux with the right context

This design keeps that product contract while moving the implementation
to a cleaner architecture in parallel.

## Core Decision

`patina ai` is a native interface shell over Patina core.

It is not:

- a child
- a toy
- a plugin
- part of the belief substrate itself

OpenCode, Gemini, and Claude Code are interface adapters. They are ways
humans use Patina, not the thing Patina fundamentally is.

The session model itself is defined in [[session-narrative-system]].
This spec consumes that model from the interface side.

## Layering

### 1. Patina core

Owns:

- beliefs
- eventlog
- graph/provenance
- Mother runtime
- toys
- children

Must function without:

- tmux
- Claude/OpenCode/Gemini
- interactive terminal workflows

### 2. Interface layer

Owns:

- launcher UX
- first-run bootstrap behavior
- interface selection
- tmux/session layout
- context/bootstrap files
- adapter lifecycle hooks

This is where `patina ai` lives.

### 3. Audit projection layer

Owns the durable review trail:

- session markdown
- git tags
- commit linkage
- spec files

This layer remains protected during the migration. New runtime/session
state may exist behind the scenes, but the git-backed review trail
cannot be casually replaced.

## Adapter Contract

The first clean interface contract should be native and narrow.

Suggested responsibilities:

```rust
trait AiAdapter {
    fn name(&self) -> &'static str;
    fn detect(&self) -> AdapterDetection;
    fn bootstrap(&self, project: &Path, env: &Environment) -> Result<()>;
    fn context_file(&self, project: &Path) -> PathBuf;
    fn launch(&self, request: LaunchRequest) -> Result<()>;
    fn check_in(&self, request: InterfaceCheckIn) -> Result<CheckInResult>;
    fn session_hooks(&self) -> SessionHookSupport;
}
```

`patina ai` then owns the orchestration around that contract:

- bootstrap workspace/project if needed
- choose adapter
- check in with Mother
- attach persona and session context
- create or attach live runtime state
- project audit artifacts
- invoke adapter launch

**Exact module decision:** create a new `src/interface/` module for this
contract instead of extending `src/adapters/mod.rs::LLMAdapter` until it
means two incompatible things.

## Why Adapters Are Not Children

Children are Mother-governed apps with toys and agency.

Adapters are operator-facing transport shells.

If an interface like OpenCode were modeled as a child, the architecture
would confuse:

- human interaction transport
- agent application logic
- runtime authority boundaries

So the line should stay:

- adapters are native
- children are Mother-hosted apps behind them when needed

## Session Strategy

The current session system is single-active-session and deeply
git-integrated. It should remain the compatibility baseline.

`patina ai` may add live session/runtime state behind the scenes, but it
must not corrupt the review trail.

For the new path, the live session system should come from
[[session-narrative-system]], not from continuing to treat
`.patina/local/active-session.md` as the sole source of truth.

That means the interface build depends on the new `src/session/` API,
not direct file reads.

Safe migration rule:

- live AI session state can evolve
- audit artifacts must remain truthful and reviewable
- compatibility mode remains available until the new path is trusted

That means `patina ai` should initially either:

1. project into compatible session artifacts deliberately, or
2. write clearly separate parallel artifacts for comparison

## OpenCode First

OpenCode is the right first target because:

- it is already partially integrated
- it is less historically overloaded than Claude
- it forces the adapter contract to be clean rather than Claude-shaped

Minimum OpenCode milestone:

- `patina ai opencode`
- bootstrap/context generation
- tmux launch
- live runtime/session attachment
- truthful audit projection strategy

OpenCode should prove:

- Mother check-in
- persona/session attachment
- deterministic tmux reattach
- durable session projection

## Gemini Second

Gemini should reuse the same contract with as little special logic as
possible. If Gemini requires architectural exceptions, the contract is
not clean enough yet.

## Smallest Safe Sequence

1. Add `patina ai` command tree with no behavior change to the current
   `patina` launcher path.
2. Introduce a native `AiAdapter` contract and move OpenCode onto it.
3. Attach the new path to Mother check-in and the session model from
   [[session-narrative-system]].
4. Decide and implement the truthful audit projection strategy.
5. Add Gemini on the same contract.
6. Only after that, consider whether Claude should migrate to the new
   path as well.

## Exact File Targets

- `src/main.rs`
  Add `Commands::Ai { ... }`. Do not change launcher-mode fallback yet.
- new `src/commands/ai/mod.rs`
  Thin public CLI routing.
- new `src/commands/ai/internal.rs`
  Orchestration for selection, check-in, and launch.
- new `src/interface/mod.rs`
  Small public interface contract.
- new `src/interface/internal/checkin.rs`
  Mother check-in request/response and translation.
- new `src/interface/internal/tmux.rs`
  Reuse/compose around existing tmux semantics.
- new `src/interface/internal/bootstrap.rs`
  Bootstrap/context file generation rules.
- `src/commands/launch/mod.rs`
  Compatibility reference for `derive_session_name` and
  `resolve_tmux_decision`.
- `src/commands/launch/internal.rs`
  Compatibility reference for the current first-run and "Are you lost?"
  flow.
- `src/commands/mother/mod.rs` and `src/commands/mother/daemon.rs`
  Minimal check-in plumbing.
- `src/adapters/opencode/mod.rs`
  Source of existing OpenCode-specific bootstrap/context behavior.
- `src/adapters/gemini/mod.rs`
  Follow-on implementation target after OpenCode.

## Interface Check-In Contract

Recommended first request shape:

```rust
struct InterfaceCheckIn {
    interface_kind: InterfaceKind,
    project_root: PathBuf,
    project_uid: Option<String>,
    requested_persona: Option<String>,
    requested_session: Option<String>,
    capabilities: InterfaceCapabilities,
}
```

Recommended first response shape:

```rust
struct CheckInResult {
    persona_uid: Option<String>,
    session_runtime_id: String,
    session_file_id: String,
    launch_policy: LaunchPolicy,
}
```

Keep this small. Do not turn check-in into a general RPC escape hatch.

## Tmux Contract

The current attach behavior is a requirement, not an implementation
detail.

Preserve:

- `derive_session_name()` compatibility
- `resolve_tmux_decision()` compatibility
- per-project stable tmux session identity
- attach-or-create semantics

Do not regress to "always spawn a new tmux session" behavior.

## Verification

- tests for `Ai` CLI routing in `src/main.rs`
- unit tests for tmux decision reuse
- integration-ish verification for OpenCode check-in and reattach
- explicit regression check that no-subcommand `patina` path still works
- explicit regression check that Mother need not change for old launch
  path to keep working

## Key Files

- `src/main.rs` — entrypoint and new command routing
- `src/commands/launch/internal.rs` — current compatibility launcher
- `src/adapters/launch.rs` — current adapter discovery/bootstrap logic
- `src/adapters/mod.rs` — current adapter abstraction to compare against
- `src/adapters/opencode/mod.rs` — first new-path adapter target
- `src/adapters/gemini/mod.rs` — second new-path adapter target
- `src/commands/session/` — compatibility audit trail behavior to
  preserve
- `src/commands/mother/daemon.rs` — runtime integration points

## Risks To Watch

- accidental breakage of the beloved `patina` no-subcommand path
- hidden coupling to Claude-specific behavior
- mixing live session state with audit trail semantics too early
- over-generalizing adapters into a plugin system before the contract is
  proven

## Open Questions

- Should `patina ai` start as a top-level subcommand or a small parallel
  binary first?
- Should live runtime/session state be created eagerly on launch or only
  when an adapter asks for it?
- What is the first truthful projection format for `patina ai` sessions:
  compatible session artifacts, or clearly separate parallel artifacts?
- When OpenCode and Gemini are stable, should Claude migrate onto the
  same contract or remain compatibility-only for a while?
