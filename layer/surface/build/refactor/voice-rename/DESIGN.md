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
- Database column rename via ALTER TABLE (SQLite 3.25+)
- Filesystem: rename `~/.patina/mother/persona/` → `~/.patina/mother/voice/` at startup
- Era 1 oracle paths (`~/.patina/personas/`) are NOT touched
- Belief YAML `persona:` metadata field deferred — batch later

## Commits

1. `refactor(voice): rename Era 3 identity types and protocol fields` — PersonaUid → VoiceUid in protocol.rs and state.rs. ConnectPayload.persona → .voice. Session binding: requested_persona → requested_voice, persona_uid → voice_uid, persona_matches → voice_matches in checkin, live, artifact, and sessions service. ALTER TABLE for state.db column.

2. `refactor(voice): rename Mother-side voice paths and project binding` — paths.rs: mother::persona → mother::voice module (persona_dir → voice_dir, ensure_persona_dir → ensure_voice_dir, identity_age, beliefs_db). persona_path() → voice_path() for project binding (.patina/persona → .patina/voice). workspace init creates mother/voice/ dir. Startup migration moves old mother/persona/ to mother/voice/ if it exists.

3. `refactor(voice): update three beliefs to voice terminology` — Rename belief files and update content where "persona" means the Era 3 identity concept. Add revision log entries noting the rename. Keep provenance and evidence references accurate.

## Direct Code Targets

### Commit 1: Protocol + Session (Era 3 identity plumbing)
- `mother/src/protocol.rs:16-24` — `PersonaUid` → `VoiceUid`
- `mother/src/protocol.rs:72-76` — `ConnectPayload.persona` → `.voice`, comment update
- `mother/src/state.rs` — `PersonaUid` → `VoiceUid`, validation fn, `persona_uid` → `voice_uid` column
- `mother/src/services/sessions.rs` — `persona_uid` parameter → `voice_uid`
- `src/interface/internal/checkin.rs` — `InterfaceCheckIn.requested_persona` → `.requested_voice`, `CheckInResult.persona_uid` → `.voice_uid`, `persona_matches()` → `voice_matches()`
- `src/session/internal/live.rs` — persona metadata fields → voice
- `src/session/internal/artifact.rs` — persona context → voice

### Commit 2: Mother-side paths + project binding
- `src/paths.rs:370-411` — `mother::persona` mod → `mother::voice`, `validate_persona_uid` → `validate_voice_uid`, `persona_dir()` → `voice_dir()`, `ensure_persona_dir()` → `ensure_voice_dir()`
- `src/paths.rs:525-527` — `persona_path()` → `voice_path()`
- `src/workspace/internal.rs` — init creates `mother/voice/` dir, migration for old `mother/persona/`
- Tests in paths.rs — update `test_mother_persona_paths` → `test_mother_voice_paths`

### Commit 3: Beliefs
- `layer/surface/epistemic/beliefs/persona-is-a-patina-instance.md` — update Era 3 references to voice (sovereignty = voice namespace, not "persona namespace")
- `layer/surface/epistemic/beliefs/persona-keypair-is-node-identity.md` → `voice-keypair-is-node-identity.md` (rename file, update content)
- `layer/surface/epistemic/beliefs/beliefs-live-at-two-levels.md` — update "persona-level beliefs" → "voice-level beliefs" in content

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
# Verify Era 3 paths renamed:
ls ~/.patina/mother/voice/default/
# Verify no stray Era 3 "persona" in identity code:
grep -n "persona" mother/src/protocol.rs src/interface/internal/checkin.rs
# ^ should return zero matches
```

## Build Readiness

- Scope is small: ~10 files, ~80 lines changed
- No external dependencies
- No blockers
- Pre-v1: no backward compatibility needed

## Open Questions

None. Scope is locked.
