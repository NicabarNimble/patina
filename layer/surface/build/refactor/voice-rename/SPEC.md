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
  - feat/voice-lake-mvp1
exit_criteria:
  - id: vr1
    text: "Mother protocol type renamed: VoiceUid (was PersonaUid)"
    checked: false
  - id: vr2
    text: "ConnectPayload.voice field (was .persona)"
    checked: false
  - id: vr3
    text: "Session binding uses voice_uid, requested_voice, voice_matches() throughout checkin and artifact code"
    checked: false
  - id: vr4
    text: "Mother-side paths use voice: ~/.patina/mother/voice/{uid}/ (was mother/persona/)"
    checked: false
  - id: vr5
    text: "Project binding file is .patina/voice (was .patina/persona)"
    checked: false
  - id: vr6
    text: "Mother state.db column renamed voice_uid (was persona_uid)"
    checked: false
  - id: vr7
    text: "Three persona beliefs updated to voice terminology in belief content"
    checked: false
  - id: vr8
    text: "cargo check passes, all tests pass"
    checked: false
---
# refactor: voice-rename

## Problem

"Persona" accumulated three incompatible meanings across Patina's evolution:

- **Era 1 (Oct 2025):** Persona as interpretive lens — the knowledge oracle. This is what the current `patina persona` CLI implements (JSONL notes, E5 embeddings, vector search). This stays as-is.
- **Era 2 (Mar 2, 2026):** Persona as sovereign Patina instance. Captured as belief, then scoped down.
- **Era 3 (Mar 20-21, 2026):** Persona as cryptographic namespace within Mother — a keypair-scoped identity that owns beliefs, signs them, and spans machines. This is the current architecture, and "persona" is a misname for it.

Era 3's identity namespace concept was called "persona" because it evolved from the persona oracle. But they're different things. The oracle is a knowledge notebook. The identity namespace is the WHO — who owns beliefs, who signs them, who they federate for. Building voice-lake-mvp1 and multiproject-belief-share on top of a misnamed concept creates confusion.

## Goal

Rename "persona" to "voice" **only where it represents the Era 3 identity namespace concept**. The Era 1 persona oracle (`patina persona` CLI, oracle module, JSONL storage) stays untouched — it will be retired later when voice subsumes its functionality.

Voice is the WHO in Patina's vocabulary: Mother=WHERE, Project=WHAT, Child=HOW, Toy=CAN, Pando=PRODUCT, Voice=WHO.

## Status

Draft. Ready to build — scope is small and mechanical.

## Non-Goals

- Renaming Era 1 persona oracle commands or module (stays as `patina persona`)
- Renaming `PersonaEvent`, `PersonaResult`, `PersonaStatus` (Era 1 oracle types)
- Moving `~/.patina/personas/` or `~/.patina/cache/personas/` (Era 1 oracle data)
- Changing `include_persona` scry flag or `[PERSONA]` routing label (queries the Era 1 oracle)
- Changing `collect_persona_values()` / `parse_persona_value()` in Mother graph (reads Era 1 data)
- Migrating the 280 `persona: architect` fields in belief YAML
- Implementing crypto keypair generation, voice belief schema, or P2P federation
- Building any new voice functionality — rename only

## What Changes (Era 3 identity namespace)

| Location | Current | Target |
|---|---|---|
| `mother/src/protocol.rs` | `PersonaUid` | `VoiceUid` |
| `mother/src/protocol.rs` | `ConnectPayload.persona` | `ConnectPayload.voice` |
| `mother/src/state.rs` | `PersonaUid`, `persona_uid` column | `VoiceUid`, `voice_uid` column |
| `src/interface/internal/checkin.rs` | `requested_persona`, `persona_uid`, `persona_matches()` | `requested_voice`, `voice_uid`, `voice_matches()` |
| `src/session/internal/live.rs` | persona metadata fields | voice metadata fields |
| `src/session/internal/artifact.rs` | persona context | voice context |
| `mother/src/services/sessions.rs` | `persona_uid` parameter | `voice_uid` parameter |
| `src/paths.rs:370-411` | `mother::persona` module | `mother::voice` module |
| `src/paths.rs:525-527` | `persona_path()` → `.patina/persona` | `voice_path()` → `.patina/voice` |
| `src/workspace/internal.rs` | creates `mother/persona/` dir | creates `mother/voice/` dir |
| Three belief files | persona terminology | voice terminology |

## What Stays (Era 1 oracle)

| Location | Why it stays |
|---|---|
| `src/commands/persona/mod.rs` | Era 1 oracle module — different concept |
| `patina persona` CLI subcommand | Era 1 oracle commands |
| `PersonaEvent`, `PersonaResult`, `PersonaStatus` | Era 1 oracle types |
| `~/.patina/personas/`, `~/.patina/cache/personas/` | Era 1 oracle data |
| `include_persona` scry flag, `[PERSONA]` label | Controls Era 1 oracle queries |
| `collect_persona_values()` in graph | Reads Era 1 oracle data |
| `.gemini/commands/epistemic-beliefs.toml` | References Era 1 oracle |

## Solution

Three commits: protocol/session types → paths/migration → beliefs. Each compiles.

## Resolved Decisions

- **Scope boundary**: Era 3 (identity) renames. Era 1 (oracle) untouched.
- **Name**: "voice" — short, epistemic, self-documenting, fits vocabulary system
- **Database migration**: ALTER TABLE rename `persona_uid` → `voice_uid` in state.db
- **Backward compat**: No shim — pre-v1 clean break
- **Belief YAML `persona:` field**: Deferred — batch later, not blocking

## Verification

```bash
cargo check
cargo nextest run
# Verify Era 1 oracle still works:
patina persona status
# Verify Era 3 identity paths exist:
ls ~/.patina/mother/voice/default/
```

## Exit Criteria

See frontmatter (vr1-vr8).

## Build Readiness

- All code targets identified
- No external dependencies
- No blockers — this is a leaf refactor
- Small scope: ~10 files, ~80 lines changed
