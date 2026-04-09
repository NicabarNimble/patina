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

Era 3's identity namespace concept was called "persona" because it evolved from the persona oracle. But they are fundamentally different things:

- **Persona (Era 1)** is a knowledge notebook — it captures notes, embeds them, and answers semantic queries. It's a tool for personal knowledge management within a single machine. The `patina persona` CLI, its JSONL storage, and its vector search are a complete, self-contained feature. "Persona" is the correct name for this.
- **Persona (Era 3)** is a cryptographic identity namespace within Mother — it owns beliefs, signs them with a keypair, scopes knowledge across projects, and federates across machines. This is not a notebook. It's the WHO behind the knowledge system. "Persona" is a misname inherited from the oracle's evolution path.

Era 2 (persona as sovereign instance) was an intermediate step that got scoped down into Era 3. It's captured in the `persona-is-a-patina-instance` belief (status: scoped) and doesn't have standalone code — its surviving ideas live in Era 3's design.

This spec corrects the Era 3 misname. Era 1 persona keeps its name and its code. They will coexist until voice subsumes the oracle's functionality in a future spec.

## Goal

Rename "persona" to "voice" **only where it represents the Era 3 identity namespace concept**. The Era 1 persona oracle (`patina persona` CLI, oracle module, JSONL storage) is a different concept and stays untouched.

Voice is the WHO in Patina's vocabulary: Mother=WHERE, Project=WHAT, Child=HOW, Toy=CAN, Pando=PRODUCT, Voice=WHO.

## Status

Draft. Ready to build — scope is small and mechanical.

## Non-Goals

**Era 1 persona oracle is out of scope — it is a different concept, not a rename target:**
- `patina persona` CLI commands and module (`src/commands/persona/`) — this is the knowledge notebook, correctly named
- `PersonaEvent`, `PersonaResult`, `PersonaStatus` types — Era 1 oracle types
- `~/.patina/personas/` and `~/.patina/cache/personas/` — Era 1 oracle data
- `include_persona` scry flag and `[PERSONA]` routing label — controls Era 1 oracle queries
- `collect_persona_values()` / `parse_persona_value()` in Mother graph — reads Era 1 data
- `.gemini/commands/epistemic-beliefs.toml` — references Era 1 oracle

**Other non-goals:**
- Migrating the 280 `persona: architect` fields in belief YAML (deferred, batch later)
- Implementing crypto keypair generation, voice belief schema, or P2P federation
- Building any new voice functionality — this is a misname correction, not new feature work

## What Changes (Era 3 identity namespace)

| Location | Current | Target |
|---|---|---|
| `mother/src/protocol.rs` | `PersonaUid` | `VoiceUid` |
| `mother/src/protocol.rs` | `ConnectPayload.persona` | `ConnectPayload.voice` |
| `mother/src/state.rs` | `PersonaUid`, `persona_uid` column | `VoiceUid`, `voice_uid` column |
| `src/mother/mod.rs:51` | re-exports `PersonaUid` | re-exports `VoiceUid` |
| `src/interface/internal/checkin.rs` | `requested_persona`, `persona_uid`, `persona_matches()` | `requested_voice`, `voice_uid`, `voice_matches()` |
| `src/session/mod.rs:55` | `persona_uid` field | `voice_uid` field |
| `src/session/internal/live.rs` | persona metadata fields | voice metadata fields |
| `src/session/internal/artifact.rs` | persona context | voice context |
| `mother/src/services/sessions.rs` | `persona_uid` parameter | `voice_uid` parameter |
| `src/commands/ai/surface.rs` | `resolve_persona_uid()`, `PATINA_PERSONA_UID` env, `persona_uid` event key | `resolve_voice_uid()`, `PATINA_VOICE_UID`, `voice_uid` |
| `src/commands/ai/mod.rs` | `persona` field on launch request | `voice` field |
| `src/project/internal.rs` | `persona_path()`, `get_persona()`, creates `mother/persona/default/` | `voice_path()`, `get_voice()`, creates `mother/voice/default/` |
| `src/project/mod.rs` | re-exports `persona_path()`, `get_persona()` | re-exports `voice_path()`, `get_voice()` |
| `src/paths.rs:370-411` | `mother::persona` module | `mother::voice` module |
| `src/paths.rs:525-527` | `persona_path()` → `.patina/persona` | `voice_path()` → `.patina/voice` |
| `src/workspace/internal.rs` | creates `mother/persona/` dir | creates `mother/voice/` dir |
| 1 belief backlink file | `[[persona-keypair-is-node-identity]]` | `[[voice-keypair-is-node-identity]]` |
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

Three commits: protocol/session/AI-launch types → paths/migration → beliefs. Each compiles.
Re-exports and downstream consumers must move in the same commit as their type definitions.

## Resolved Decisions

- **Scope boundary**: Era 3 (identity) renames. Era 1 (oracle) untouched.
- **Name**: "voice" — short, epistemic, self-documenting, fits vocabulary system
- **Database migration**: Idempotent — check if column exists before ALTER TABLE. Fresh installs create `voice_uid` directly. Handle: already renamed (no-op), never created (fresh CREATE TABLE), failed mid-run (retry-safe).
- **Filesystem migration**: At startup, if `mother/persona/{uid}/` exists and `mother/voice/{uid}/` does not, rename. If both exist, log warning and prefer `mother/voice/`. Project binding: if `.patina/persona` exists and `.patina/voice` does not, rename file.
- **Backward compat**: No serde aliases — pre-v1 clean break.
- **Belief YAML `persona:` metadata field**: Deferred — batch later, not blocking.
- **Belief backlinks**: `[[persona-keypair-is-node-identity]]` reference in `host-proxied-io-is-the-security-model.md` must be updated when renaming the belief file. Session archives are historical records and not updated.
- **Env var**: `PATINA_PERSONA_UID` → `PATINA_VOICE_UID`.

## Verification

```bash
cargo check
cargo nextest run
# Verify Era 1 oracle untouched:
patina persona status
# Verify Era 3 identity code renamed (should return zero matches):
grep -rn "PersonaUid\|persona_uid\|requested_persona\|PATINA_PERSONA" \
  mother/src/protocol.rs mother/src/state.rs \
  src/interface/internal/checkin.rs src/commands/ai/surface.rs \
  src/session/mod.rs src/mother/mod.rs src/project/
# Verify Era 3 paths renamed:
ls ~/.patina/mother/voice/default/
# Verify event payload and env var use voice:
grep -n "voice_uid\|PATINA_VOICE" src/commands/ai/surface.rs
```

## Exit Criteria

See frontmatter (vr1-vr8).

## Build Readiness

- All code targets identified (audit verified: ~17 files, ~120 lines)
- No external dependencies
- No blockers — this is a leaf refactor
