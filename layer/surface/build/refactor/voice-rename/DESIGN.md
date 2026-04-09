# Design: voice-rename

## Why This Design

Era 3 used "persona" to mean the crypto-namespace identity concept. That was a
misname — "persona" already means the Era 1 knowledge oracle. We fix the misname
by renaming only the Era 3 identity plumbing to "voice." The Era 1 oracle stays
untouched under `patina persona` until it's naturally retired later.

## Build Target

Two commits on `patina` branch. Each compiles independently.

## Resolved Decisions

- No serde aliases for old field names — clean break, pre-v1
- Database migration is idempotent: check column exists before ALTER TABLE, fresh installs use `voice_uid` directly, retry-safe on failure
- Filesystem migration: if `mother/persona/{uid}/` exists and `mother/voice/{uid}/` does not, rename. If both exist, log warning, prefer `mother/voice/`. Project binding: if `.patina/persona` exists and `.patina/voice` does not, rename file.
- Era 1 oracle paths (`~/.patina/personas/`) are NOT touched
- Belief YAML `persona:` metadata field deferred — batch later
- Belief backlinks: update `[[persona-keypair-is-node-identity]]` → `[[voice-keypair-is-node-identity]]` in `host-proxied-io-is-the-security-model.md`. Session archives are historical, not updated.
- Env var: `PATINA_PERSONA_UID` → `PATINA_VOICE_UID`

## Commits

1. `refactor(voice): rename Era 3 identity types, paths, protocol, session, and AI launch` — ALL code changes in one commit. PersonaUid → VoiceUid. ConnectPayload.persona → .voice. paths.rs: mother::persona → mother::voice, persona_path → voice_path. Session binding. AI launch surface: resolve fn, env var, event key, --persona → --voice flag. Project module. Re-exports. DB migration. Workspace init + registration create voice dirs. Filesystem migration for existing dirs/files. One commit because types, paths, callers, and re-exports are coupled — splitting them creates intermediate states that won't compile.

2. `refactor(voice): update beliefs and backlinks to voice terminology` — Rename persona-keypair-is-node-identity.md → voice-keypair-is-node-identity.md. Update content in persona-is-a-patina-instance.md and beliefs-live-at-two-levels.md where "persona" means Era 3 identity. Update backlink in host-proxied-io-is-the-security-model.md. Add revision log entries.

## Direct Code Targets

### Commit 1: All code changes (types + paths + protocol + session + AI launch + migration)

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
- `src/commands/ai/surface.rs:306-315` — `resolve_persona_uid()` fn + migration trigger (see below)
- `src/commands/ai/surface.rs:413-429` — tests for resolve fn
- `src/commands/ai/mod.rs:64-65` — `--persona` CLI flag → `--voice` (removed immediately, no alias)
- `src/commands/ai/mod.rs:178,188,198` — `persona` field on launch request → `voice`

**Other call sites (struct literals that reference renamed fields):**
- `src/commands/launch/internal.rs:88` — `AiLaunchRequest { persona: None }` → `{ voice: None }`
- `src/commands/ai/internal.rs:378` — `LiveSessionHandle { persona_uid: None }` → `{ voice_uid: None }`

**Project module:**
- `src/project/mod.rs:127-134` — `persona_path()` → `voice_path()`, `get_persona()` → `get_voice()`
- `src/project/internal.rs:345-352` — `persona_path()` → `voice_path()`, `get_persona()` → `get_voice()`

**Path functions (must be in same commit — callers depend on them):**
- `src/paths.rs:370-411` — `mother::persona` mod → `mother::voice`: `validate_persona_uid` → `validate_voice_uid`, `persona_dir()` → `voice_dir()`, `ensure_persona_dir()` → `ensure_voice_dir()`
- `src/paths.rs:525-527` — `persona_path()` → `voice_path()`
- Tests: `test_mother_persona_paths` → `test_mother_voice_paths`

**Workspace init:**
- `src/workspace/internal.rs:131-144` — creates `mother/voice/default/` (was `mother/persona/default/`)

**Project registration:**
- `src/project/internal.rs:408-414` — creates `mother/voice/default/` (was `mother/persona/default/`)
- `src/project/internal.rs:816-838` — test: `test_register_with_mother_creates_default_voice_store`

**Filesystem migration (guaranteed to run on every AI launch):**
Migration lives in `resolve_voice_uid()` (formerly `resolve_persona_uid()`) in `ai/surface.rs`. This function runs on every `patina ai` launch, so already-registered projects get migrated on next use — not only during registration. Logic:
- If `.patina/persona` exists and `.patina/voice` does not → rename file
- If `~/.patina/mother/persona/{uid}/` exists and `~/.patina/mother/voice/{uid}/` does not → rename dir
- If both old and new exist → log warning, prefer new
- Fresh install → no migration needed

**CLI compatibility:**
- `--persona` flag on `patina ai {claude,opencode,gemini}` is **removed immediately**, not aliased. Pre-v1, no external consumers. The flag becomes `--voice`.

### Commit 2: Beliefs + Backlinks

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
