# Design: voice-rename

## Why This Design

Era 3 used "persona" to mean the crypto-namespace identity concept. That was a
misname — "persona" already means the Era 1 knowledge oracle. We fix the misname
by renaming only the Era 3 identity plumbing to "voice." The Era 1 oracle stays
untouched under `patina persona` until it's naturally retired later.

## Build Target

Three commits on `patina` branch. Each compiles independently.

## Resolved Decisions

- No serde aliases for old field names — clean break, pre-v1
- Database migration is idempotent: check column exists before ALTER TABLE, fresh installs use `voice_uid` directly, retry-safe on failure
- Filesystem migration: if `mother/persona/{uid}/` exists and `mother/voice/{uid}/` does not, rename. If both exist, log warning, prefer `mother/voice/`. Project binding: if `.patina/persona` exists and `.patina/voice` does not, rename file.
- Era 1 oracle paths (`~/.patina/personas/`) are NOT touched
- Belief YAML `persona:` metadata field deferred — batch later
- Belief backlinks: update `[[persona-keypair-is-node-identity]]` → `[[voice-keypair-is-node-identity]]` in `host-proxied-io-is-the-security-model.md`. Session archives are historical, not updated.
- Env var: `PATINA_PERSONA_UID` → `PATINA_VOICE_UID`

## Commits

1. `refactor(voice): rename Era 3 identity types, protocol, session, and AI launch` — PersonaUid → VoiceUid in protocol.rs and state.rs. ConnectPayload.persona → .voice. Session binding: requested_persona → requested_voice, persona_uid → voice_uid, persona_matches → voice_matches in checkin, live, artifact, session mod, and sessions service. AI launch surface: resolve_persona_uid → resolve_voice_uid, PATINA_PERSONA_UID → PATINA_VOICE_UID, persona event payload key → voice_uid. AI mod: persona launch field → voice. Re-exports in mother/mod.rs and session/mod.rs. Project module: persona_path → voice_path, get_persona → get_voice. DB schema migration for state.db column.

2. `refactor(voice): rename Mother-side voice paths, project binding, and workspace init` — paths.rs: mother::persona → mother::voice module (validate, dir, ensure_dir, identity_age, beliefs_db). persona_path() → voice_path() for project binding (.patina/persona → .patina/voice). workspace init creates mother/voice/ dir with migration for old mother/persona/. project/internal.rs registration creates mother/voice/default/. Filesystem migration with edge case handling.

3. `refactor(voice): update beliefs and backlinks to voice terminology` — Rename persona-keypair-is-node-identity.md → voice-keypair-is-node-identity.md. Update content in persona-is-a-patina-instance.md and beliefs-live-at-two-levels.md where "persona" means Era 3 identity. Update backlink in host-proxied-io-is-the-security-model.md. Add revision log entries.

## Direct Code Targets

### Commit 1: Types + Protocol + Session + AI Launch

**Mother protocol (type definition):**
- `mother/src/protocol.rs:16-24` — `PersonaUid` → `VoiceUid`
- `mother/src/protocol.rs:72-76` — `ConnectPayload.persona` → `.voice`, comment update

**Mother state (DB schema + type):**
- `mother/src/state.rs` — `PersonaUid` → `VoiceUid`, validation fn, `persona_uid` → `voice_uid` in CREATE TABLE and all queries
- Migration: idempotent ALTER TABLE RENAME COLUMN (check first, no-op if already done)

**Mother services:**
- `mother/src/services/sessions.rs` — `persona_uid` parameter → `voice_uid`

**Re-exports (must move in same commit as types):**
- `src/mother/mod.rs:51` — re-export `PersonaUid` → `VoiceUid`
- `src/session/mod.rs:55` — `persona_uid` field → `voice_uid`

**Session binding:**
- `src/interface/internal/checkin.rs` — `InterfaceCheckIn.requested_persona` → `.requested_voice`, `CheckInResult.persona_uid` → `.voice_uid`, `persona_matches()` → `voice_matches()`, tests
- `src/session/internal/live.rs` — persona metadata fields → voice
- `src/session/internal/artifact.rs` — persona context → voice

**AI launch surface:**
- `src/commands/ai/surface.rs:28` — `persona` field → `voice`
- `src/commands/ai/surface.rs:188` — `resolve_persona_uid()` → `resolve_voice_uid()`
- `src/commands/ai/surface.rs:195` — `requested_persona` → `requested_voice`
- `src/commands/ai/surface.rs:241` — `PATINA_PERSONA_UID` → `PATINA_VOICE_UID`
- `src/commands/ai/surface.rs:290` — `"persona_uid"` event key → `"voice_uid"`
- `src/commands/ai/surface.rs:306-315` — `resolve_persona_uid()` fn, reads `.patina/persona` → `.patina/voice`
- `src/commands/ai/surface.rs:413-429` — tests for resolve fn
- `src/commands/ai/mod.rs:65,178,188,198` — `persona` field on launch request → `voice`

**Project module:**
- `src/project/mod.rs:127-134` — `persona_path()` → `voice_path()`, `get_persona()` → `get_voice()`
- `src/project/internal.rs:345-352` — `persona_path()` → `voice_path()`, `get_persona()` → `get_voice()`

### Commit 2: Paths + Migration + Workspace Init

**Path functions:**
- `src/paths.rs:370-411` — `mother::persona` mod → `mother::voice`: `validate_persona_uid` → `validate_voice_uid`, `persona_dir()` → `voice_dir()`, `ensure_persona_dir()` → `ensure_voice_dir()`
- `src/paths.rs:525-527` — `persona_path()` → `voice_path()`
- Tests: `test_mother_persona_paths` → `test_mother_voice_paths`

**Workspace init:**
- `src/workspace/internal.rs:131-144` — creates `mother/voice/default/` (was `mother/persona/default/`)
- Add migration: if old `mother/persona/` exists and `mother/voice/` does not, rename

**Project registration:**
- `src/project/internal.rs:408-414` — creates `mother/voice/default/` (was `mother/persona/default/`)
- `src/project/internal.rs:816-838` — test: `test_register_with_mother_creates_default_voice_store`
- Add migration: if `.patina/persona` exists and `.patina/voice` does not, rename file

**Edge cases:**
- Both `mother/persona/` and `mother/voice/` exist → log warning, prefer `mother/voice/`
- Partial move (dir renamed but beliefs.db not) → rename at directory level (atomic on same FS)
- Fresh install → create `mother/voice/` directly, no migration needed

### Commit 3: Beliefs + Backlinks

**Belief file renames:**
- `persona-keypair-is-node-identity.md` → `voice-keypair-is-node-identity.md` (rename file + update id in frontmatter)
- `persona-is-a-patina-instance.md` — update content where "persona" means Era 3 (keep filename since belief is about the scoped concept, add revision log)
- `beliefs-live-at-two-levels.md` — update "persona-level beliefs" → "voice-level beliefs"

**Backlink updates:**
- `layer/surface/epistemic/beliefs/host-proxied-io-is-the-security-model.md` — `[[persona-keypair-is-node-identity]]` → `[[voice-keypair-is-node-identity]]`

**NOT updated (historical records):**
- Session archives referencing `[[persona-keypair-is-node-identity]]` (5 session files) — these are historical

### NOT touched (Era 1 oracle — stays as persona)
- `src/commands/persona/mod.rs` — entire oracle module
- `src/main.rs` — `Persona` CLI subcommand, `PersonaCommands` enum
- `src/paths.rs:83-93` — `persona::events_dir()`, `persona::cache_dir()`
- `src/commands/scry/` — `include_persona` flag, `[PERSONA]` label
- `src/commands/mother/graph.rs` — `collect_persona_values()`, `parse_persona_value()`
- `.gemini/commands/epistemic-beliefs.toml` — references Era 1 oracle

## Verification Plan

After each commit:
```bash
cargo check
```

After all commits:
```bash
cargo nextest run
# Verify Era 1 oracle untouched:
patina persona status
# Verify Era 3 identity code fully renamed (should return zero matches):
grep -rn "PersonaUid\|persona_uid\|requested_persona\|PATINA_PERSONA\|get_persona\|persona_path\|persona_dir\|persona_matches" \
  mother/src/protocol.rs mother/src/state.rs mother/src/services/ \
  src/interface/internal/checkin.rs src/commands/ai/surface.rs src/commands/ai/mod.rs \
  src/session/mod.rs src/session/internal/ src/mother/mod.rs \
  src/project/ src/paths.rs src/workspace/
# Verify Era 3 paths renamed:
ls ~/.patina/mother/voice/default/
# Verify env var and event payload:
grep -n "PATINA_VOICE_UID\|voice_uid" src/commands/ai/surface.rs
# Verify belief backlink updated:
grep -r "persona-keypair-is-node-identity" layer/surface/epistemic/beliefs/
# ^ should return zero (only session archives may still reference it)
```

## Build Readiness

- Scope: ~17 files, ~120 lines changed
- No external dependencies
- No blockers
- Pre-v1: no backward compatibility needed
- All edge cases (DB migration, filesystem migration, backlinks) defined

## Open Questions

None. Scope is locked.
