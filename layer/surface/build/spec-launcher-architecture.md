# Spec: Patina Launcher Architecture

**Status:** Design Revised (2025-12-09)
**Session:** 20251209-100946
**Phase:** 5-6 (Launcher & MCP)

---

## Core Concept

**Patina is how you open AI-assisted development.**

```bash
patina              # Open in default frontend
patina claude       # Open in Claude Code
patina gemini       # Open in Gemini CLI
```

Like `code .` for VS Code. Not `claude`, not `gemini` - just `patina`.

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
│  │  • /health      │         │  • patina_context       │   │
│  │  • /api/scry    │         │  • patina_scry          │   │
│  │  • /api/context │         │  • patina_session_*     │   │
│  └─────────────────┘         └─────────────────────────┘   │
│                                                             │
│  ┌─────────────────────────────────────────────────────┐   │
│  │                  Shared State                        │   │
│  │  • Registry (projects, repos)                        │   │
│  │  • Personas (global rules)                           │   │
│  │  • Model cache (E5, projections)                     │   │
│  └─────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
         ▲                              ▲
         │ HTTP                         │ MCP
         │                              │
   ┌─────┴─────┐                 ┌──────┴──────┐
   │ Containers│                 │ Claude Code │
   │ Scripts   │                 │ Gemini CLI  │
   └───────────┘                 └─────────────┘
```

One process. Two interfaces. All the state.

---

## Key Principles

### 1. Source vs Presentation

Project rules live in `.patina/context.md` (committed). Frontend-specific files are generated and gitignored.

```
.patina/context.md     →  patina claude  →  CLAUDE.md (gitignored)
(source of truth)         (generates)       (presentation)
```

### 2. Two-Tier Rules

```
Global Rules (~/.patina/personas/)     Project Rules (.patina/context.md)
├── Light touch                        ├── Detailed architecture
├── "I prefer explicit errors"         ├── Code patterns for THIS codebase
├── General preferences                ├── Project-specific conventions
└── Cross-project patterns             └── The meaty stuff
            │                                    │
            └────────────────┬───────────────────┘
                             │
                             ▼
                    Frontend Adapter
                     (combines + formats)
                             │
                     ┌───────┴───────┐
                     ▼               ▼
                CLAUDE.md        GEMINI.md
                (generated)      (generated)
```

### 3. The Patina Branch Model

**Rule: Always work on `patina` branch. Push to main via PR.**

```
patina branch              →  PR  →  main branch
(our workspace)                     (clean for project)
```

This protects against overwriting others' repos and provides clear isolation.

### 4. Frontend Switching is Instant

Same source, regenerate presentation:

```bash
patina claude    # Generates CLAUDE.md, launches claude
patina gemini    # Generates GEMINI.md from same source, launches gemini
```

---

## Command Structure

### Launcher (Implicit Default)

```bash
patina                      # Default frontend, current dir
patina claude               # Claude Code
patina gemini               # Gemini CLI
patina codex                # Codex
patina ~/project claude     # Path + frontend
patina --yolo gemini        # YOLO container with Gemini
```

**Note:** Frontends are NOT subcommands. They're arguments to the implicit launcher.

### Infrastructure

```bash
patina serve                # Start mothership (HTTP + MCP)
patina serve --daemon       # Start in background
patina serve --status       # Check if running
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
```

---

## Launch Flow

```
patina claude
    │
    ├─► Is mothership running?
    │   └─► No? Start: patina serve --daemon
    │
    ├─► Is this a patina project?
    │   └─► No .patina/? → "Run patina init first"
    │
    ├─► Generate presentation files:
    │   ├─► Read .patina/context.md (source of truth)
    │   ├─► Read ~/.patina/personas/ (global rules)
    │   ├─► Generate CLAUDE.md (combined, formatted)
    │   └─► Ensure .claude/ exists (from adapter templates)
    │
    └─► Launch: exec claude
```

---

## File Structure

### Global (~/.patina/)

```
~/.patina/
├── config.toml              # Global config (default frontend, etc.)
├── adapters/
│   ├── claude/
│   │   └── templates/       # .claude/, slash commands, scripts
│   ├── gemini/
│   │   └── templates/
│   └── codex/
│       └── templates/
├── personas/
│   └── default/             # Global rules, preferences
├── registry.yaml            # All known projects
└── workspace/               # → ~/Projects/Patina
```

### Project (Committed)

```
project/
├── .patina/
│   ├── config.toml          # Project config (mode: owner/contrib)
│   └── context.md           # PROJECT RULES (source of truth)
├── layer/
│   ├── core/                # Eternal patterns
│   ├── surface/             # Active docs
│   └── sessions/            # Work history
└── .gitignore               # Ignores presentation files
```

### Project (Generated, Gitignored)

```
project/
├── CLAUDE.md                # Generated from context.md + persona
├── GEMINI.md                # Generated (if using gemini)
├── .claude/                 # Copied from ~/.patina/adapters/claude/
└── .gemini/                 # Copied from ~/.patina/adapters/gemini/
```

### .gitignore (on patina branch)

```gitignore
# Frontend presentation (generated by patina)
CLAUDE.md
GEMINI.md
CODEX.md
.claude/
.gemini/
.codex/
```

---

## The Branch Model

### Owner Repos (Your Projects)

```
patina branch:                    main (via PR):
├── .patina/           ──────►    ├── .patina/        ✓ included
├── layer/             ──────►    ├── layer/          ✓ included
├── .gitignore         ──────►    ├── .gitignore      ✓ included
├── src/               ──────►    ├── src/            ✓ included

CI: Simple merge (branches are ~identical)
```

### Contrib Repos (Other People's Projects)

```
patina branch:                    main (via PR):
├── .patina/           ──────►    (stripped)          ✗ removed
├── layer/             ──────►    (stripped)          ✗ removed
├── .gitignore         ──────►    (stripped)          ✗ removed
├── src/ (changes)     ──────►    ├── src/            ✓ only code

CI: Strips patina artifacts, only code changes go through
```

### Project Config

```toml
# .patina/config.toml

[project]
name = "linux-kernel"
mode = "contrib"              # or "owner"
upstream = "torvalds/linux"

[frontend]
default = "claude"

[ci]
# For contrib mode: strip from PRs
strip_paths = [".patina/", "layer/"]
```

---

## .patina/context.md (Source of Truth)

This file contains all project rules in frontend-agnostic markdown:

```markdown
# Project: my-game

## Overview
Bevy ECS roguelike game engine.

## Architecture
- Entity spawning: src/ecs/spawn.rs (use spawn_entity(), never direct)
- Game state: GameWorld resource
- Events: src/events/mod.rs

## Patterns
- Error handling: thiserror, Result<T,E>, wrap with .context()
- Naming: snake_case functions, PascalCase types
- ECS: Components are data-only, systems have logic

## Commands
- `cargo run` - development build
- `cargo test` - run tests
- `patina scry "query"` - search knowledge

## Key Decisions
- Using Bevy 0.12 for ECS
- Custom event system over bevy_eventlistener
- Sessions tracked in layer/sessions/
```

Adapters combine this with global persona and format for their frontend.

---

## YOLO Containers

```
┌─────────────────────────────────────────────────────────────┐
│  Mac                                                        │
│  ┌─────────────────┐    ┌─────────────────────────────────┐ │
│  │ Mothership      │◄───│ YOLO Container                  │ │
│  │ (patina serve)  │    │                                 │ │
│  │                 │    │ patina gemini runs:             │ │
│  │ • personas      │    │ ├─► Reads mounted context.md    │ │
│  │ • registry      │    │ ├─► Generates GEMINI.md locally │ │
│  │ • MCP server    │    │ ├─► Copies .gemini/ templates   │ │
│  │                 │    │ └─► Launches gemini             │ │
│  └─────────────────┘    └─────────────────────────────────┘ │
│                                │                            │
│                         mount: /work ← project/             │
└─────────────────────────────────────────────────────────────┘
```

Container generates its own presentation files. Source is mounted from host.

```bash
patina --yolo gemini
# 1. Spins up YOLO container with Gemini CLI
# 2. Mounts current project at /work
# 3. Container connects to host mothership via MCP
# 4. Runs: patina gemini (generates GEMINI.md locally)
# 5. Full patina access via mothership
```

---

## MCP Tools

### patina_context

Query project context and rules (combines global + project).

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
    { "file": "src/ecs/spawn.rs", "score": 0.89, "snippet": "..." }
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
    { "name": "my-game", "path": "~/Projects/Patina/my-game" }
  ]
}
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
```

### Frontend Detection

Frontends are detected via simple enum (not manifest files):

```rust
pub enum Frontend {
    Claude,   // detect: claude --version
    Gemini,   // detect: gemini --version
    Codex,    // detect: codex --version
}
```

Simple, type-safe, matches dependable-rust philosophy.

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
  ✓ Installed adapter templates

Detecting frontends...
  ✓ Claude Code (claude v1.0.3)
  ✗ Gemini CLI (not found)
  ✗ Codex (not found)

Setting default: claude

This directory is not a patina project.
Initialize? [y/n]: y

Initializing...
  ✓ Created .patina/
  ✓ Created layer/
  ✓ Generated context.md template

Starting mothership...
  ✓ patina serve (background)

Launching Claude Code...
  ✓ Generated CLAUDE.md
  ✓ Installed .claude/ templates

# Claude Code opens, fully configured
```

---

## Phase Integration

### Phase 5: Launcher & Adapters

```
5a: First-run setup
    - ~/.patina/ structure
    - Adapter templates installation
    - Frontend detection (enum-based)

5b: Launcher command
    - `patina [path] [frontend]` as default behavior
    - Auto-start mothership
    - Presentation file generation

5c: Source/Presentation model
    - .patina/context.md as source of truth
    - Generate CLAUDE.md/GEMINI.md on launch
    - .gitignore for presentation files

5d: Branch model
    - Always work on patina branch
    - Owner vs contrib mode
    - CI stripping for contrib repos
```

### Phase 6: MCP Integration

```
6a: MCP server in mothership
    - Add MCP to patina serve
    - stdio interface for frontends

6b: Core MCP tools
    - patina_context (combines global + project rules)
    - patina_scry
    - patina_session_*

6c: Workspace MCP tools
    - patina_workspace_list
    - Cross-project queries
```

---

## Summary Table

| Aspect | Design |
|--------|--------|
| Launcher | `patina [frontend]` (implicit, no subcommand) |
| Frontends | Enum (claude, gemini, codex) - simple, type-safe |
| Source of truth | `.patina/context.md` (committed) |
| Presentation | `CLAUDE.md`, `.claude/` (gitignored, generated) |
| Global rules | `~/.patina/personas/` (light touch) |
| Project rules | `.patina/context.md` (detailed) |
| Branch model | Always `patina` branch, PR to main |
| Owner repos | PR includes patina artifacts |
| Contrib repos | CI strips patina artifacts |
| Mothership | `patina serve` (HTTP + MCP, one process) |
| Switching | Instant (regenerate from same source) |
| YOLO | Container generates locally, source mounted |

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
| Same source works with all frontends | [ ] |
| Presentation files are gitignored | [ ] |
| Owner mode: patina artifacts in main | [ ] |
| Contrib mode: CI strips artifacts | [ ] |
