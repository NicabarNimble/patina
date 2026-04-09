# Design: voice-rename

## Why This Design

Mechanical rename with no behavioral changes. Pre-v1 so no backward compatibility needed. Clean break avoids dual-naming confusion when building voice-lake-mvp1 and multiproject-belief-share.

## Build Target

All six commits land on `patina` branch. Each commit compiles independently.

## Resolved Decisions

- No serde aliases for old field names — clean break
- Database column rename via ALTER TABLE (SQLite supports this since 3.25)
- Filesystem migration: move directories, not copy (atomic on same filesystem)
- Belief YAML `persona:` field deferred to separate batch commit

## Commits

1. `refactor(voice): rename types and module persona → voice` — Move `src/commands/persona/` to `src/commands/voice/`, rename PersonaUid → VoiceUid, PersonaEvent → VoiceEvent, PersonaResult → VoiceResult, PersonaStatus → VoiceStatus in all files

2. `refactor(voice): rename CLI subcommand persona → voice` — Update main.rs enum (Persona → Voice, PersonaCommands → VoiceCommands), help text, --no-persona → --no-voice flag

3. `refactor(voice): rename protocol, session, and routing fields` — ConnectPayload.persona → .voice, ScryPayload.include_persona → .include_voice, checkin requested_persona → requested_voice, persona_uid → voice_uid, persona_matches → voice_matches, [PERSONA] → [VOICE] label

4. `refactor(voice): rename filesystem paths and add migration` — Path module persona → voice, directory paths personas/ → voices/, mother/persona/ → mother/voice/, .patina/persona → .patina/voice. Add startup migration that moves old paths to new if they exist.

5. `refactor(voice): update three persona beliefs to voice terminology` — Rename belief files, update content to use "voice" where it means the Era 3 concept. Keep provenance accurate (note rename in revision log).

6. `refactor(voice): rename graph labels and config templates` — collect_persona_values → collect_voice_values, parse_persona_value → parse_voice_value, gemini config persona references

## Direct Code Targets

### Commit 1: Types + Module
- `src/commands/persona/mod.rs` → `src/commands/voice/mod.rs` (entire module rename)
- `mother/src/protocol.rs:16-24` — PersonaUid → VoiceUid
- `mother/src/state.rs:118-145` — PersonaUid → VoiceUid + validation
- `src/commands/voice/mod.rs:29` — PersonaEvent → VoiceEvent
- `src/commands/voice/mod.rs:45` — PersonaResult → VoiceResult
- `src/commands/voice/mod.rs:354` — PersonaStatus → VoiceStatus

### Commit 2: CLI Surface
- `src/main.rs:284` — Persona { command } → Voice { command }
- `src/main.rs:819-870` — PersonaCommands → VoiceCommands, all variant help text
- `src/main.rs:1347-1370` — match arm Persona → Voice
- `src/main.rs:216-218` — --no-persona → --no-voice in scry

### Commit 3: Protocol + Session + Routing
- `mother/src/protocol.rs:76` — ConnectPayload.persona → .voice
- `mother/src/protocol.rs:120` — ScryPayload.include_persona → .include_voice
- `src/interface/internal/checkin.rs:~40 lines` — requested_persona, persona_uid, persona_matches
- `src/session/internal/live.rs:~9 lines` — persona metadata fields
- `src/session/internal/artifact.rs:~4 lines` — persona context
- `src/commands/scry/internal/routing.rs:224-233` — include_persona, [PERSONA] label
- `src/commands/scry/mod.rs:~3 lines` — flag name
- `mother/src/state.rs:344` — DB schema persona_uid column
- `mother/src/services/sessions.rs:~3 lines` — query parameter

### Commit 4: Filesystem Paths
- `src/paths.rs:83-95` — pub mod persona → pub mod voice, events_dir, cache_dir
- `src/paths.rs:371-413` — mother::persona → mother::voice, persona_dir, identity_age, beliefs_db
- `src/paths.rs:525-528` — persona_path → voice_path
- `src/workspace/internal.rs` — init creates voice dirs, migration moves old persona dirs

### Commit 5: Beliefs
- `layer/surface/epistemic/beliefs/persona-is-a-patina-instance.md` → rename/update to voice terminology
- `layer/surface/epistemic/beliefs/persona-keypair-is-node-identity.md` → voice-keypair-is-node-identity.md
- `layer/surface/epistemic/beliefs/beliefs-live-at-two-levels.md` → update persona references to voice

### Commit 6: Graph + Config
- `src/commands/mother/graph.rs:150-161` — collect_persona_values → collect_voice_values
- `src/commands/mother/graph.rs:523-571` — parse_persona_value → parse_voice_value
- `src/commands/mother/graph.rs:848-900` — display "persona" → "voice"
- `.gemini/commands/epistemic-beliefs.toml` — persona references
- `resources/gemini/epistemic-beliefs.toml` — persona references

## Verification Plan

After each commit:
```bash
cargo check
```

After all commits:
```bash
cargo nextest run
patina voice status
patina voice list
grep -r "persona" src/ mother/src/ --include="*.rs" | grep -v "// persona" | grep -v test
```

The final grep confirms no stray persona references remain (except in comments noting the rename and in test names which can be cleaned up).

## Build Readiness

- Audit complete: 364 occurrences in .rs, 4 in .toml, scope fully mapped
- No external dependencies
- No blockers
- Pre-v1: no backward compatibility needed

## Open Questions

None. Scope is locked.
