# Spec: LLM Frontend Integration

**Status**: Core Complete (MCP configs deferred to separate spec)
**Created**: 2025-12-30
**Updated**: 2025-12-31
**Implemented**: Session 20251231-155307
**Purpose**: Unified frontend experience across Claude Code, OpenCode, and Gemini CLI

---

## Goal: Unified Frontend

All 3 interactive frontends get the **same 5 commands**:

| Command | Purpose |
|---------|---------|
| `/session-start` | Begin session with Git tracking |
| `/session-update` | Capture progress |
| `/session-note` | Add insight |
| `/session-end` | Archive and classify |
| `/patina-review` | "Catch me up" on recent work |

Same behavior, different syntax per frontend:
- Claude/OpenCode: Markdown commands
- Gemini: TOML commands

---

## What Exists (Already Built)

### Current State (as of 2025-12-31)

| Frontend | Adapter | Templates | Commands | Status |
|----------|---------|-----------|----------|--------|
| Claude | ✅ ClaudeAdapter | ✅ 9 files | ✅ 5 commands | **Complete** |
| Gemini | ✅ GeminiAdapter | ✅ 9 files | ✅ 5 commands | **Complete** |
| OpenCode | ✅ OpenCodeAdapter | ✅ 9 files | ✅ 5 commands | **Complete** |

All frontends now provide unified experience with Git-tagged sessions.

---

## Target State

All 3 frontends have identical structure (different syntax):

```
.{frontend}/
├── {CONTEXT}.md              # CLAUDE.md, AGENTS.md, or GEMINI.md
├── bin/
│   ├── session-start.sh
│   ├── session-update.sh
│   ├── session-note.sh
│   └── session-end.sh
└── commands/
    ├── session-start.{ext}   # .md or .toml
    ├── session-update.{ext}
    ├── session-note.{ext}
    ├── session-end.{ext}
    └── patina-review.{ext}
```

---

## Tasks

### Phase 0: Cleanup Claude ✅ Complete

Archive deprecated commands before spreading them to other frontends.

- [x] Git tag `archive/persona-experiment` on current commit
- [x] Remove from `resources/claude/`:
  - `persona-start.sh`
  - `persona-start.md`
  - `launch.sh`
  - `launch.md`
- [x] Update `templates.rs` to not embed removed files
- [x] Update `session_scripts.rs` to remove includes
- [x] Update `mod.rs` get_custom_commands() from 6 to 5

**Note**: adapter-manifest.json kept (needed for version tracking, not over-engineering)

**Commits**: e9b28d7c, 98fe8e8d

### Phase 1: Add OpenCode ✅ Complete

- [x] Create `src/adapters/opencode/mod.rs`
- [x] Create `src/adapters/opencode/internal/mod.rs`
- [x] Add OpenCode to `src/adapters/mod.rs` get_adapter()
- [x] Create `resources/opencode/`:
  - 4 session scripts (changed `.claude` → `.opencode`)
  - 5 commands (session-start/update/note/end + patina-review)
  - Generates `AGENTS.md` context template in init
- [x] Add `opencode_templates` module to `templates.rs`
- [x] Add `install_opencode_templates()` function

**Note**: Frontend enum in launch.rs is separate system (runtime detection of binaries on PATH). Adapters are init-time selection via --llm flag.

**Commit**: 168e7be2

### Phase 2: Align Gemini ✅ Complete

- [x] Add `patina-review.toml` to `resources/gemini/`
- [x] Update `templates.rs` to embed and install it
- [x] Update Gemini adapter `get_custom_commands()` from 0 to 5
- [x] Add tests to verify command registration

**Deferred**: MCP config integration (Phase 3)

**Commit**: 95133fad

### Phase 2b: Git Tags Include Frontend ✅ Complete

Updated all 6 session scripts (3 frontends × 2 scripts) to include frontend identifier:

```bash
# Before
git tag "session-${SESSION_ID}-start"

# After (dynamic extraction from directory path)
FRONTEND=$(basename $(dirname $(dirname "$0")) | sed 's/^\.//')
git tag "session-${SESSION_ID}-${FRONTEND}-start"
```

**Result**: Tags like `session-20251231-155307-claude-start`

**Enables**:
- Filter by frontend: `git tag -l "session-*-gemini-*"`
- Track tool usage in multi-frontend projects
- Distinguish session histories

**Files modified**:
- resources/claude/session-start.sh, session-end.sh
- resources/gemini/session-start.sh, session-end.sh
- resources/opencode/session-start.sh, session-end.sh

**Commit**: 5d449f25

### Phase 3: MCP Configs ⏸ Deferred

**Status**: Deferred to separate spec (not blocking unified 5-command experience)

- [ ] Add Gemini MCP config template
- [ ] Add OpenCode MCP config template
- [ ] Test: `patina adapter mcp gemini`
- [ ] Test: `patina adapter mcp opencode`

**Note**: Claude already has MCP support via `patina adapter mcp claude`. Gemini and OpenCode MCP integration deferred until needed.

---

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│  INTERACTIVE LAYER (unified 5 commands)                     │
│                                                             │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐         │
│  │ Claude Code │  │  OpenCode   │  │ Gemini CLI  │         │
│  │  .md cmds   │  │  .md cmds   │  │ .toml cmds  │         │
│  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘         │
│         └────────────────┼────────────────┘                 │
│                          ↓                                  │
│              Same 4 shell scripts (session-*.sh)            │
│                          ↓                                  │
│                ┌─────────────────┐                          │
│                │   Patina MCP    │                          │
│                │  scry/context/  │                          │
│                │     assay       │                          │
│                └─────────────────┘                          │
├─────────────────────────────────────────────────────────────┤
│  DELEGATE LAYER (no session tracking - by design)          │
│                                                             │
│                ┌─────────────────┐                          │
│                │     Codex       │  ← Future spec           │
│                └─────────────────┘                          │
└─────────────────────────────────────────────────────────────┘
```

---

## Notes

### Command Formats

| Frontend | Command ext | Args syntax | Shell syntax |
|----------|-------------|-------------|--------------|
| Claude Code | `.md` | `$ARGUMENTS` | Via hooks |
| OpenCode | `.md` | `$ARGUMENTS` | `` !`cmd` `` |
| Gemini CLI | `.toml` | `{{args}}` | `!{cmd}` |

### AGENTS.md Collision

Both OpenCode and Codex use `AGENTS.md`. Differentiate by config directory:
- OpenCode: `.opencode/` exists
- Codex: `.codex/` exists

### Archived Commands

| Command | Added | Removed | Reason |
|---------|-------|---------|--------|
| `/persona-start` | Aug 2025 | This spec | Deprecated experiment |
| `/launch` | Aug 2025 | This spec | Built but never used |

Tag: `archive/persona-experiment`

---

## Summary

**Implemented**: 2025-12-31 (Session 20251231-155307)

✅ **Core Goal Achieved**: All 3 frontends now provide unified 5-command experience

**What Was Built**:
- Phase 0: Cleaned Claude (removed 2 deprecated commands)
- Phase 1: Built OpenCode adapter from scratch
- Phase 2: Completed Gemini adapter (added patina-review)
- Phase 2b: Enhanced git tags with frontend identifiers

**Commits**: 5 total (e9b28d7c, 98fe8e8d, 168e7be2, 95133fad, 5d449f25)

**Deferred**: MCP configuration templates (Phase 3) - not blocking core functionality

---

## References

- `src/adapters/` - Adapter implementations
- `resources/` - Template files
- Session 20251230-192904 - Audit that discovered spec needed
- Session 20251231-155307 - Implementation session (this spec)
