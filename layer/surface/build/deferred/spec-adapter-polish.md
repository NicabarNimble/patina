---
id: spec-adapter-polish
status: design
created: 2026-01-13
tags: [spec, adapter, scaffold, context, mcp]
references: [adapter-pattern, dependable-rust, unix-philosophy]
---

# Spec: Adapter Polish

**Problem:** Current adapter scaffolding overwrites context files, drifts from templates, and puts too much content in static files instead of MCP tools.

**Solution:** Minimal injection pattern - scaffold is a pointer, not the content. Push intelligence to MCP.

---

## Core Principle

> Patina's scaffold should be the smallest possible surface that connects the LLM to patina's capabilities.

**Before:** Full context embedded in CLAUDE.md, commands with inline instructions
**After:** One-line pointer to central context, MCP tools deliver intelligence

---

## Design

### Directory Structure

```
project/
├── CLAUDE.md                    # User's file (append pointer OR leave alone)
├── .claude/
│   ├── CLAUDE.md               # Minimal: @include pointer only
│   ├── commands/               # Slash commands (keep minimal)
│   └── skills/                 # Future: migrate session commands here
├── .patina/
│   ├── context/                # NEW: central context for all adapters
│   │   ├── claude.md          # Full Claude context
│   │   ├── gemini.md          # Full Gemini context
│   │   └── opencode.md        # Full OpenCode context
│   └── ...
└── layer/                      # Patterns (can be @included)
```

### The @include Pattern

Claude Code (since 0.2.107) supports file includes:
```markdown
@path/to/file.md
```

**Minimal `.claude/CLAUDE.md`:**
```markdown
@.patina/context/claude.md
```

That's it. One line. All actual content lives in `.patina/context/`.

### Existing CLAUDE.md Handling

| Scenario | Action |
|----------|--------|
| No CLAUDE.md exists | Create minimal with @include |
| CLAUDE.md exists, no patina section | Append `## Patina` + @include |
| CLAUDE.md exists, has patina section | Update @include path only |
| User declines modification | Create .claude/CLAUDE.md only |

### Central Context File

`.patina/context/claude.md` contains what currently lives scattered:

```markdown
# Patina Project Context

## Available Tools (MCP)
- `scry` - Search codebase knowledge
- `context` - Get patterns and conventions
- `assay` - Query codebase structure

## Session Commands
- `/session-start [name]` - Begin session with Git tracking
- `/session-update` - Capture progress
- `/session-note [insight]` - Add insight
- `/session-end` - Archive and distill

## Project Reference
@layer/core/build.md
```

**Key insight:** The MCP tools ARE the intelligence. The context file just tells the LLM they exist.

---

## Version Tracking

### Current State

- Claude: Has `adapter-manifest.json` + VERSION_CHANGES changelog
- Gemini/OpenCode: Stub implementations only

### Target State

All adapters track:
1. **Adapter version** - Our scaffold version (e.g., `0.8.0`)
2. **CLI version** - Detected at launch (e.g., `claude 2.1.3`)
3. **Template checksums** - Detect drift from source

### Manifest Format

```json
{
  "adapter": "claude",
  "version": "0.8.0",
  "cli_version": "2.1.3",
  "installed_at": "2026-01-13T20:00:00Z",
  "files": {
    ".claude/CLAUDE.md": "sha256:abc123",
    ".claude/commands/session-start.md": "sha256:def456"
  }
}
```

---

## Template Drift

### Problem

Old files persist after template updates:
- `launch.md` removed from source but still in `.claude/commands/`
- `persona-start.md` removed but still installed

### Solution

`adapter refresh` performs clean update:

1. Read manifest to get installed files
2. Compare with current source templates
3. **Remove** files no longer in source
4. **Update** files that changed
5. **Add** new files
6. Update manifest with new checksums

### User Customizations

Files with `# USER CUSTOMIZATIONS` section are preserved during refresh:
- Content above marker: replaced with new template
- Content below marker: preserved

---

## Adapter Parity

All three adapters get identical capabilities:

| Feature | Claude | Gemini | OpenCode |
|---------|--------|--------|----------|
| Context file | `.claude/CLAUDE.md` | `GEMINI.md` | `AGENTS.md` |
| @include support | Yes (0.2.107+) | Yes (hierarchical) | TBD |
| Version tracking | Full | Stub → Full | Stub → Full |
| Clean refresh | Yes | Add | Add |
| MCP integration | Yes | Partial | Yes |

---

## Implementation Phases

### Phase 1: Central Context

- [ ] Create `.patina/context/` directory structure
- [ ] Move content from `.claude/CLAUDE.md` → `.patina/context/claude.md`
- [ ] Update `.claude/CLAUDE.md` to single @include line
- [ ] Test Claude Code loads included content

### Phase 2: Smart Init

- [ ] `adapter add` detects existing context files
- [ ] Prompt user for handling strategy (append/skip/replace)
- [ ] Create central context in `.patina/context/`
- [ ] Update templates to use @include pattern

### Phase 3: Version Parity

- [ ] Implement full version tracking for Gemini adapter
- [ ] Implement full version tracking for OpenCode adapter
- [ ] Add CLI version detection to manifest
- [ ] Add file checksums to manifest

### Phase 4: Clean Refresh

- [ ] `adapter refresh` removes obsolete files
- [ ] Preserve USER CUSTOMIZATIONS sections
- [ ] Update manifest after refresh
- [ ] Show diff of changes before applying

---

## Success Criteria

1. `adapter add` on project with existing CLAUDE.md doesn't overwrite
2. All patina context lives in `.patina/context/`, not scattered
3. `.claude/CLAUDE.md` is ≤5 lines (just @includes)
4. `adapter refresh` removes obsolete files
5. Version tracking works for all 3 adapters

---

## References

- Claude Code @include: CHANGELOG 0.2.107
- Gemini hierarchical loading: docs/cli/gemini-md.md
- Session 20251230-180841: adapter sync concept
- spec/llm-frontends: unified 5-command experience
