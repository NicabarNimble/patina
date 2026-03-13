---
type: refactor
id: agentic-surface-architecture
status: draft
created: 2026-03-11
related:
- mother-maturation
- core-extraction
- patina-ai-interface-layer
- session-narrative-system
- persona-federation
- continuous-operation
- spec-subsystem-plugin
beliefs:
- patina-identity
- patina-is-beliefs-plus-action
- mother-is-connection-and-continuity
- session-capture
- spec-driven-design
- safety-boundaries
- dependable-rust
- unix-philosophy
- interfaces-are-not-core
sessions:
  origin: 20260310-230611
exit_criteria:
- id: children-complete
  text: Child specs patina-ai-interface-layer, session-narrative-system, persona-federation, continuous-operation, and spec-subsystem-plugin are complete and aligned
  checked: false
- id: boundary-explicit
  text: The operator surface has an explicit boundary of interface actor -> Mother -> session/child -> toy across specs and implementation
  checked: true
- id: compatibility-preserved
  text: The current patina launcher and git-backed session trail remain trusted compatibility surfaces until the new path proves itself
  checked: true
---
# refactor: Agentic Surface Architecture

> Unify Patina interfaces, multi-session narrative capture, persona identity, and Mother check-in into one operator-surface architecture while preserving the current launcher and git-backed session trail as trusted compatibility surfaces.

## Current State

Patina's core/runtime direction and its operator surface have started to
diverge.

What is now becoming clear from both the code and the spec tree:

- Patina core is the belief system and protocol substrate
- Mother is the authority/runtime seam around that substrate
- Children are bounded workers/apps with toys
- interactive surfaces like Claude Code, OpenCode, Gemini CLI, or a web
  app are not core and are not children

But the current operator surface still reflects older assumptions:

- the launcher is excellent product UX but tightly coupled internally
- the session system is single-active-session and local-first
- personas and continuous Mother operation are designed but not yet tied
  into the interface/session story
- spec workflow extraction is being designed independently of interface
  and session evolution

Without a unifying spec, these areas can drift into overlapping partial
systems. This spec now serves better as a roadmap/index than as an
active implementation queue item.

## Target State

The target architecture is an explicit operator surface over Patina
core:

- **Patina core** remains the knowledge protocol and belief runtime
- **Mother** is the authority and connection layer
- **sessions** are the durable narrative and handoff layer
- **interfaces** are thin connection points for humans and external
  agents
- **children** remain bounded workers with toys
- **personas** provide isolation, identity, and federation scope

The core rule is:

- interfaces connect
- sessions narrate
- Mother authorizes
- children work
- toys grant capability

This refactor does not collapse those layers into one abstraction. It
clarifies them and connects them through Mother.

## Child Specs

| Spec | Role in this architecture | Why it belongs here |
|------|---------------------------|---------------------|
| [[patina-ai-interface-layer]] | Native front door and adapter contract | Makes interfaces thin and explicit |
| [[session-narrative-system]] | Multi-session narrative model | Makes sessions deeper, multiplayer, and still git-anchored |
| [[persona-federation]] | Identity and isolation | Gives Mother the who/boundary layer |
| [[continuous-operation]] | Always-on Mother/node runtime | Gives interfaces and streams a living authority host |
| [[spec-subsystem-plugin]] | Governance app under Mother authority | Keeps spec workflow aligned with the same operator model |

## Architectural Decisions

### 1. Interfaces are not core and not children

Interactive surfaces like OpenCode, Gemini CLI, Claude Code, and future
web/headless agents are interface actors. They connect users or agents
to Patina. They are not children and do not own capability bundles of
their own.

### 2. Mother check-in is the first authority boundary

An interface actor should first check in with Mother:

- identify interface type
- identify project/workspace
- identify persona or requested persona context
- attach to or create a session
- request access to relevant children/services

This preserves a single authority seam before any work begins.

### 3. Sessions are controlled narrative, not just local scratch files

The current git/session trail is too valuable to lose. The next system
must preserve and deepen it:

- same or better durable artifacts in `layer/sessions/`
- same or better git tag and commit capture
- richer handoff/provenance semantics
- later semantic extraction into datablocks/beliefs

### 4. Personas isolate worlds; federation connects them

A user working for company ABC, company XYZ, and personal projects does
not share one undifferentiated world. Personas and Mother-federated
links provide the boundary model.

### 5. Headless agents use the same operator surface

`patina ai` and future web interfaces are not special. They are the
first interface actors. A future headless agent should use the same
Mother/session/persona surface rather than a separate bypass path.

## Implementation Prerequisites

- Read the existing launcher/session/Mother code before rewriting any of
  it. The trusted UX and git trail came from real constraints, not
  accidents.
- Preserve the current `patina` no-subcommand launcher path until the
  new path is demonstrably better.
- Use narrow, typed Rust module boundaries per [[dependable-rust]].
- Keep specs authoritative per [[spec-driven-design]]; edge cases are
  spec updates, not ad hoc architectural improvisation.

## Steps

1. Build the new session model before or alongside the new interface
   path so `patina ai` has a real narrative substrate to attach to.
2. Build `patina ai` as a native compatibility-preserving front door.
3. Tie interface check-in to Mother and persona identity.
4. Bring spec workflow under the same Mother authority model rather than
   leaving it as a special in-process subsystem forever.
5. Let continuous Mother operation become the always-on node/runtime
   that makes local-first multiplayer and federation possible.

## Exit Criteria

1. Child specs `patina-ai-interface-layer`,
   `session-narrative-system`, `persona-federation`,
   `continuous-operation`, and `spec-subsystem-plugin` are complete and
   aligned
2. The operator surface has an explicit boundary of interface actor ->
   Mother -> session/child -> toy across specs and implementation
3. The current `patina` launcher and git-backed session trail remain
   trusted compatibility surfaces until the new path proves itself
