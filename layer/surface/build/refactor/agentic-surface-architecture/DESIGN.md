# Design: Agentic Surface Architecture

## Why This Exists

Patina's runtime model has become clearer than its operator model.

The belief system and Mother/Child/Toy work now define a coherent core.
What remains under-specified is the human/agent-facing surface:

- how interfaces attach
- how sessions persist and hand off meaning
- how personas isolate work
- how Mother brokers all of the above

This design makes the operator surface explicit instead of letting it
remain an accidental mix of launcher scripts, singleton session files,
and interface-specific conventions.

## Layering

### 1. Core

Owns:

- beliefs
- eventlog
- graph/provenance
- children
- toys

### 2. Mother

Owns:

- authority
- identity and isolation attachment
- interface check-in
- child access/routing
- live session coordination
- federation/node continuity

### 3. Session narrative

Owns:

- durable handoff record
- git-linked narrative
- decisions/evidence/handoff semantics
- future semantic extraction into datablocks/beliefs

### 4. Interface actors

Own:

- transport and UX
- conversation/input
- tmux/browser/CLI lifecycles
- presenting context and results

Interfaces do not own truth, capability grants, or specialist worker
logic.

## Interface Check-In Contract

The first operator interaction with Patina should be a Mother check-in.

Conceptually:

```rust
struct InterfaceCheckIn {
    interface_kind: InterfaceKind,
    project_uid: ProjectUid,
    persona: Option<PersonaSelector>,
    requested_session: Option<SessionSelector>,
    capabilities: InterfaceCapabilities,
}
```

Mother responds with the allowed operating context:

- persona attachment
- session attachment or creation
- available children/services
- runtime endpoints
- projection/audit policy

This should be a narrow, typed API. The SDK lesson applies here too:
coarse, explicit bundles beat universal grab-bags.

## Session Contract

The session system should split into:

- **live session state** — Mother-managed, many active sessions,
  participants, leases, interface seats
- **session artifact** — durable markdown + git-linked narrative in
  `layer/sessions/`

The artifact remains human-first and reviewable. The live state exists
to coordinate reality, not replace the artifact.

## Persona Contract

Every operator interaction occurs in a persona context, explicit or
resolved by Mother. Personas isolate worlds and give provenance meaning.

This means:

- interfaces do not pick arbitrary global state silently
- sessions should know which persona they belong to
- children/toys are granted within that scoped world

## Spec Contract

Specs remain governance objects:

- spec = what should happen
- session = what happened and why
- belief = what is now considered true

This is why `spec-subsystem-plugin` belongs under the same container:
the governance path should share Mother authority and not remain a
special side channel.

## Rust Design Rules

- Keep public interfaces small and typed per [[dependable-rust]]
- Prefer composition over giant orchestration modules per
  [[unix-philosophy]]
- Preserve strong safety boundaries at the Mother host layer per
  [[safety-boundaries]]
- Read existing launch/session code before replacement because that code
  captures product requirements, not just legacy accidents

## Smallest Safe Sequence

1. Build the session narrative system so multiplayer/live state has a
   truthful durable projection target.
2. Build `patina ai` on top of that instead of inventing its own
   temporary session semantics.
3. Tie interface attachment to persona-aware Mother check-in.
4. Bring spec workflow under the same Mother authority seam.
5. Use continuous Mother operation to support local-first multiplayer
   and future multi-Mother nodes.

## Key Files

- `src/commands/launch/internal.rs` — current trusted launcher UX
- `src/commands/session/internal.rs` — current singleton session logic
- `src/main.rs` — current command entry structure
- `src/mother/mod.rs` — current Mother authority surface
- `layer/core/values/patina-identity.md` — core identity constraint
- `layer/core/values/session-capture.md` — low-friction capture rule
- `layer/core/values/spec-driven-design.md` — governance chain

## Open Questions

- Should the first Mother check-in API live in the main binary, Mother
  daemon command surface, or a dedicated internal crate?
- What is the first truthful projection format for multi-session live
  state into `layer/sessions/`?
- When multi-Mother/node transport arrives, which parts of interface
  check-in stay local and which become federated?
