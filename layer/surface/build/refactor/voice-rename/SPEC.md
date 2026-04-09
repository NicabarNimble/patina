---
type: refactor
id: voice-rename
status: draft
created: 2026-04-09
sessions:
  origin: 20260409-070410-485377000
references:
  - beliefs-live-at-two-levels
  - persona-keypair-is-node-identity
  - persona-is-a-patina-instance
related:
  - feat/multiproject-belief-share
  - feat/persona-lake-mvp1
exit_criteria:
  - id: vr1
    text: "CLI command is `patina voice` (persona removed from CLI surface)"
    checked: false
  - id: vr2
    text: "All Rust types renamed: VoiceUid, VoiceEvent, VoiceResult, VoiceStatus"
    checked: false
  - id: vr3
    text: "Module path is src/commands/voice/mod.rs"
    checked: false
  - id: vr4
    text: "Filesystem paths use voice: ~/.patina/voices/, ~/.patina/mother/voice/"
    checked: false
  - id: vr5
    text: "Mother protocol fields renamed: ConnectPayload.voice, ScryPayload.include_voice"
    checked: false
  - id: vr6
    text: "Session binding uses voice_uid throughout checkin and artifact code"
    checked: false
  - id: vr7
    text: "Scry routing uses include_voice flag and [VOICE] source label"
    checked: false
  - id: vr8
    text: "Three persona beliefs updated: voice-is-a-patina-namespace, voice-keypair-is-node-identity, beliefs-live-at-two-levels"
    checked: false
  - id: vr9
    text: "cargo check passes, all tests pass"
    checked: false
  - id: vr10
    text: "Existing data migrated: ~/.patina/personas/ content moved to ~/.patina/voices/"
    checked: false
---
# refactor: voice-rename

## Problem

"Persona" accumulated three incompatible meanings across Patina's evolution:

- **Era 1 (Oct 2025):** Persona as interpretive lens — a single belief system with Prolog-style when/unless/weight. This is what the current `patina persona` CLI implements (JSONL oracle with E5 embeddings).
- **Era 2 (Mar 2, 2026):** Persona as sovereign Patina instance — each persona is a full separate system. Captured as belief, then scoped.
- **Era 3 (Mar 20-21, 2026):** Persona as cryptographic namespace within Mother — not a separate instance, but a keypair-scoped identity that owns beliefs, signs them, and spans machines.

Era 3 is the current architecture. But the code still says "persona" everywhere, conflating the old oracle implementation with the new identity concept. The multiproject-belief-share and voice-lake specs need a clean foundation — building on "persona" means building on naming debt.

## Goal

Rename "persona" to "voice" everywhere it represents the Era 3 concept (cryptographic identity namespace). Voice is the WHO in Patina's vocabulary: Mother=WHERE, Project=WHAT, Child=HOW, Toy=CAN, Pando=PRODUCT, Voice=WHO.

## Status

Draft. Ready to build — scope is mechanical rename with clear boundaries.

## Non-Goals

- Implementing crypto keypair generation (that's voice-lake or belief-system-hardening)
- Building the voice belief database schema (that's voice-lake-mvp1)
- P2P federation of voices (that's multiproject-belief-share)
- Changing what the oracle commands do — only renaming them
- Migrating the 280 `persona: architect` fields in belief YAML (deferred, batch later)

## Current State

- CLI: `patina persona {note,query,list,materialize,status}`
- Types: `PersonaUid`, `PersonaEvent`, `PersonaResult`, `PersonaStatus`
- Module: `src/commands/persona/mod.rs`
- Paths: `~/.patina/personas/`, `~/.patina/mother/persona/`, `.patina/persona`
- Protocol: `ConnectPayload.persona`, `ScryPayload.include_persona`
- Session: `requested_persona`, `persona_uid`, `persona_matches()`
- Routing: `include_persona` flag, `[PERSONA]` source label
- Graph: `collect_persona_values()`, `parse_persona_value()`

## Target State

- CLI: `patina voice {note,query,list,materialize,status}`
- Types: `VoiceUid`, `VoiceEvent`, `VoiceResult`, `VoiceStatus`
- Module: `src/commands/voice/mod.rs`
- Paths: `~/.patina/voices/`, `~/.patina/mother/voice/`, `.patina/voice`
- Protocol: `ConnectPayload.voice`, `ScryPayload.include_voice`
- Session: `requested_voice`, `voice_uid`, `voice_matches()`
- Routing: `include_voice` flag, `[VOICE]` source label
- Graph: `collect_voice_values()`, `parse_voice_value()`

## Solution

Mechanical rename in four phases: types/module → CLI surface → paths/migration → beliefs/docs. Each phase is one commit, each commit compiles.

## Implementation Order

1. **Types + Module rename** — Rename structs, move module directory
2. **CLI surface** — Rename subcommand, enum variants, help text, flags
3. **Protocol + Session + Routing** — Mother protocol fields, checkin binding, scry routing
4. **Filesystem paths + migration** — Path functions, add migration helper for existing data
5. **Beliefs** — Update three persona beliefs to use voice terminology
6. **Graph + config** — Mother graph source labels, gemini config templates

## Resolved Decisions

- **Name**: "voice" — short, epistemic, self-documenting for LLMs, fits vocabulary system
- **Scope**: Rename only, no new functionality
- **Belief metadata**: The `persona: architect` field in 280 belief files is deferred — can batch-rename later without blocking this spec
- **Database migration**: Mother state.db `persona_uid` column → `voice_uid` via schema migration
- **Backward compat**: No compatibility shim — clean break, Patina is pre-v1

## Verification

```bash
cargo check
cargo nextest run
patina voice status
patina voice list
ls ~/.patina/voices/
```

## Exit Criteria

See frontmatter (vr1-vr10).

## Build Readiness

- All code targets identified by audit
- No external dependencies
- No blockers — this is a leaf refactor
