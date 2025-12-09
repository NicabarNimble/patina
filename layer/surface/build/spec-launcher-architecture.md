# Spec: Patina Launcher Architecture

**Status:** Design Complete (2025-12-09)
**Session:** 20251209-075138
**Phase:** 5-6 (Adapter & LLM Interface)

---

## Core Concept

**Patina is how you open AI-assisted development.**

```bash
cd my-project
patina              # Open in default frontend
patina claude       # Open in Claude Code
patina gemini       # Open in Gemini CLI
```

Like `code .` for VS Code, but for AI frontends.

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────┐
│                      patina serve                           │
│                     (the mothership)                        │
│                                                             │
│  ┌─────────────────┐         ┌─────────────────────────┐   │
│  │   HTTP Server   │         │     MCP Server          │   │
│  │   :50051        │         │     (stdio)             │   │
│  │                 │         │                         │   │
│  │  • /api/scry    │         │  • patina_context       │   │
│  │  • /api/context │         │  • patina_scry          │   │
│  │  • /api/session │         │  • patina_session_*     │   │
│  │                 │         │  • patina_workspace_*   │   │
│  └─────────────────┘         └─────────────────────────┘   │
│                                                             │
│  ┌─────────────────────────────────────────────────────┐   │
│  │                  Shared State                        │   │
│  │  • Registry (projects, repos)                        │   │
│  │  • Personas                                          │   │
│  │  • Model cache (E5, projections)                     │   │
│  │  • Workspace path                                    │   │
│  └─────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
         ▲                              ▲
         │ HTTP                         │ MCP
         │                              │
   ┌─────┴─────┐                 ┌──────┴──────┐
   │ Containers│                 │ Claude Code │
   │ Scripts   │                 │ Gemini CLI  │
   │ curl      │                 │ Codex       │
   └───────────┘                 └─────────────┘
```

**One process. Two interfaces. All the state.**

---

## Key Principles

### 1. Patina Rules = Source of Truth

Project knowledge lives in `layer/rules/`. Adapters render it to frontend-specific formats.

```
layer/rules/           →  patina adapter render  →  CLAUDE.md
(source of truth)         (transformation)          (ephemeral)
```

### 2. Adapters are Winamp Skins

Same content, different visualization. Easy to switch, no commitment.

```bash
patina claude    # Morning: Claude Code
patina gemini    # Afternoon: try Gemini
patina claude    # Back to Claude
```

No migration. No regeneration. Just switch.

### 3. MCP Does the Heavy Lifting

Bootstrap files (CLAUDE.md, GEMINI.md) are tiny - just "use MCP tools". The actual rules come from MCP dynamically.

### 4. CLI-First, MCP-Enhanced

Everything works via CLI. MCP wraps CLI for LLM frontends.

```bash
# CLI (always works)
patina scry "query"
patina context "query"

# MCP (wraps CLI for LLMs)
patina_scry, patina_context, etc.
```

---

## Commands

### User-Facing (Daily Use)

```bash
patina                      # Open project in default frontend
patina claude               # Open project in Claude Code
patina gemini               # Open project in Gemini CLI
patina codex                # Open project in Codex

patina .                    # Explicit current directory
patina ~/other-project      # Open different project
patina ~/project gemini     # Different project + specific frontend

patina --yolo gemini        # Launch in YOLO container with Gemini
```

### Infrastructure (Usually Automatic)

```bash
patina serve                # Start mothership (HTTP + MCP)
patina serve --daemon       # Start in background
patina serve --status       # Check if running
patina serve --stop         # Stop mothership
```

### Project Management

```bash
patina init                 # Initialize current dir as patina project
patina rebuild              # Rebuild indices from layer/
```

### Adapter Management

```bash
patina adapter list         # Show available frontends
patina adapter default X    # Set default frontend
patina adapter add X        # Install adapter resources
patina adapter check X      # Verify frontend CLI installed
```

### Workspace Management

```bash
patina workspace status     # Overview of all projects
patina workspace list       # List workspace projects
```

---

## Launch Flow

```
patina claude
    │
    ├─► Is mothership running?
    │   └─► Check: curl localhost:50051/health
    │
    ├─► If not running:
    │   └─► Start: patina serve --daemon
    │       Wait: until /health responds
    │
    ├─► Is this a patina project?
    │   ├─► No .patina/? → Prompt: `patina init` first? [y/n]
    │   └─► Yes → continue
    │
    ├─► Ensure bootstrap ready:
    │   └─► Generate CLAUDE.md if missing (tiny, instant)
    │
    └─► Launch frontend:
        └─► exec claude
            (claude connects to MCP via mothership)
```

---

## File Structure

### Global (~/.patina/)

```
~/.patina/
├── config.toml              # Global config (default frontend, etc.)
├── workspace/               # → ~/Projects/Patina (symlink or path)
├── adapters/                # Adapter resources
│   ├── claude/
│   │   ├── manifest.yaml    # Detection, requirements
│   │   └── templates/       # Bootstrap templates
│   ├── gemini/
│   └── codex/
├── personas/
│   └── default/
└── registry.yaml            # All projects and repos
```

### Project

```
my-project/
├── layer/
│   ├── rules/               # LLM instructions (source of truth)
│   │   ├── context.md       # Project overview
│   │   ├── patterns.md      # Code conventions
│   │   └── tools.md         # Available commands
│   ├── sessions/            # Work history
│   ├── core/                # Eternal patterns
│   └── surface/             # Active docs
│
├── .patina/
│   ├── config.toml          # Project config
│   └── data/                # Local indices (gitignored)
│
├── CLAUDE.md                # Bootstrap (tiny, gitignored or committed)
├── GEMINI.md                # Bootstrap (tiny)
└── CODEX.md                 # Bootstrap (tiny)
```

### Bootstrap File (Tiny)

```markdown
# CLAUDE.md (entire file, ~10 lines)

This project uses Patina for knowledge management.

## MCP Tools Available
- `patina_context` - Get project context and rules
- `patina_scry` - Search codebase knowledge
- `patina_session_*` - Track work sessions

Query `patina_context` before making assumptions about this codebase.
```

---

## layer/rules/ Structure

### context.md - Project Overview

```markdown
# Project: my-game

## What This Is
A roguelike game engine using Bevy ECS.

## Architecture
- Entity spawning: src/ecs/spawn.rs
- Game state: GameWorld resource
- Events: src/events/mod.rs

## Getting Started
Run `cargo run` for development build.
```

### patterns.md - Code Conventions

```markdown
## Error Handling
- Use thiserror for custom errors
- Prefer Result<T,E> over panics
- Wrap external errors with .context()

## Naming
- snake_case for functions
- PascalCase for types
- SCREAMING_CASE for constants

## ECS Patterns
Spawn entities via spawn_entity(), never direct world.spawn()
```

### tools.md - Available Commands

```markdown
## Patina Commands
- `patina scry "query"` - search project knowledge
- `patina context "query"` - get LLM-formatted context
- `patina session start "name"` - begin tracked work

## MCP Tools
When connected via MCP, these are available as tool calls:
- patina_context
- patina_scry
- patina_session_start
- patina_session_end
- patina_session_note
```

---

## MCP Tools

### patina_context

Query project context and rules.

```
Input: { "query": "error handling" }
Output: {
  "rules": "Use thiserror, prefer Result<T,E>...",
  "related_code": ["src/error.rs:15", "src/lib.rs:42"],
  "persona": "Prefers explicit error types over anyhow"
}
```

### patina_scry

Search codebase knowledge.

```
Input: { "query": "spawn entity", "limit": 5 }
Output: {
  "results": [
    { "file": "src/ecs/spawn.rs", "score": 0.89, "snippet": "..." },
    ...
  ]
}
```

### patina_session_start

Begin tracked work session.

```
Input: { "name": "fix-auth-bug" }
Output: { "session_id": "20251209-131500", "branch": "patina" }
```

### patina_session_end

End session and capture learnings.

```
Input: { "summary": "Fixed JWT validation bug" }
Output: { "archived": "layer/sessions/20251209-131500.md" }
```

### patina_session_note

Capture insight during session.

```
Input: { "note": "JWT library has footgun with exp validation" }
Output: { "captured": true }
```

### patina_workspace_list

List projects in workspace.

```
Input: {}
Output: {
  "projects": [
    { "name": "my-game", "path": "~/Projects/Patina/my-game", "adapter": "claude" },
    ...
  ]
}
```

---

## First-Run Experience

```bash
cargo install patina

cd my-project
patina

# Output:
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
 Welcome to Patina!
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

First-time setup...
  ✓ Created ~/.patina/
  ✓ Created ~/Projects/Patina workspace
  ✓ Installed adapters: claude, gemini, codex

Detecting LLM frontends...
  ✓ Claude Code (claude v1.0.3)
  ✗ Gemini CLI (not found)
  ✗ Codex (not found)

Setting default: claude

Initializing project...
  ✓ Created .patina/
  ✓ Created layer/
  ✓ Generated CLAUDE.md bootstrap

Starting mothership...
  ✓ patina serve (pid 12345)

Launching Claude Code...

# Claude Code opens, fully configured
```

---

## Switching Frontends

```bash
# No ceremony, just switch
patina claude    # Opens Claude Code
# ... work ...
patina gemini    # Opens Gemini CLI (same project, same rules)
# ... work ...
patina claude    # Back to Claude
```

**Same `layer/rules/`. Same MCP. Different frontend.**

---

## YOLO Multi-LLM

```bash
# On Mac: Claude Code for pair programming
patina claude

# Spin up Gemini container for parallel task
patina --yolo gemini

# Inside container:
# - Project mounted at /work
# - MCP connects to host mothership
# - Full patina access
```

```
┌─────────────────────────────────────────────────────────────┐
│  Mac                                                        │
│  ┌─────────────────┐    ┌─────────────────────────────────┐ │
│  │ Mothership      │◄───│ YOLO Container                  │ │
│  │ (patina serve)  │    │ ┌─────────────────────────────┐ │ │
│  │                 │    │ │ gemini CLI                  │ │ │
│  │ • HTTP + MCP    │    │ │ • MCP → host mothership     │ │ │
│  │ • all projects  │    │ │ • project mounted           │ │ │
│  │ • personas      │    │ └─────────────────────────────┘ │ │
│  └─────────────────┘    └─────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
```

---

## Configuration

### Global Config

```toml
# ~/.patina/config.toml

[workspace]
path = "~/Projects/Patina"

[frontend]
default = "claude"

[serve]
port = 50051
auto_start = true

[frontends.claude]
command = "claude"
detected = true
mcp_config = "~/.claude/settings.json"

[frontends.gemini]
command = "gemini"
detected = false
mcp_config = "~/.gemini/config.yaml"

[frontends.codex]
command = "codex"
detected = false
```

### Adapter Manifest

```yaml
# ~/.patina/adapters/claude/manifest.yaml
name: claude
display: "Claude Code"

detect:
  commands:
    - "claude --version"
  env:
    - "CLAUDE_CODE"

templates:
  - CLAUDE.md
  - .claude/commands/session-start.md
  - .claude/commands/session-end.md

mcp:
  config_path: "~/.claude/settings.json"
  config_format: json
  config_template: |
    {
      "mcpServers": {
        "patina": {
          "command": "patina",
          "args": ["serve", "--mcp-stdio"]
        }
      }
    }
```

---

## Phase Integration

This design restructures Phases 5-6:

### Phase 5: Adapters & Launcher (was Phase 6)

```
5a: Adapter system
    - ~/.patina/adapters/ structure
    - Manifest format
    - Detection logic

5b: Launcher command
    - `patina [path] [frontend]`
    - Auto-start mothership
    - Bootstrap generation

5c: Claude adapter
    - Templates
    - MCP configuration
    - Slash commands (optional enhancement)

5d: Gemini adapter
    - Templates
    - MCP configuration

5e: Codex adapter
    - Templates
    - MCP configuration
```

### Phase 6: MCP Integration (was Phase 5)

```
6a: MCP server in mothership
    - Add MCP to patina serve
    - stdio interface for frontends

6b: Core MCP tools
    - patina_context
    - patina_scry
    - patina_session_*

6c: Workspace MCP tools
    - patina_workspace_list
    - patina_workspace_switch
    - Cross-project queries

6d: `patina context` CLI
    - Designed from MCP learnings
    - Multiple output formats
```

### Phase 7: Capture Automation (unchanged)

### Phase 8: Model Worlds (unchanged)

---

## Key Insights

1. **Patina is the launcher** - not `claude`, not `gemini`, just `patina`

2. **Mothership = HTTP + MCP** - one process, two interfaces

3. **layer/rules/ = source of truth** - adapters render, don't define

4. **Bootstrap files are tiny** - "use MCP", that's it

5. **Switching is instant** - same rules, different frontend

6. **CLI-first** - everything works without MCP, MCP enhances

7. **YOLO integration** - containers connect to host mothership

---

## Validation Criteria

| Validation | Status |
|------------|--------|
| `patina` opens project in default frontend | [ ] |
| `patina claude` opens in Claude Code | [ ] |
| `patina gemini` opens in Gemini CLI | [ ] |
| Mothership auto-starts if not running | [ ] |
| MCP tools work from any frontend | [ ] |
| Switching frontends < 2 seconds | [ ] |
| `patina --yolo X` launches container | [ ] |
| Same project works with all frontends | [ ] |
