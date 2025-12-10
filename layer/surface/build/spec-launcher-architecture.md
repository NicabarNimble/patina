# Spec: Patina Launcher Architecture

**Status:** Design Revised (2025-12-10)
**Session:** 20251210-065208
**Phase:** 1 (Launcher & Adapters)

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

### 1. Orchestrator, Not Generator

Patina is an orchestrator that works WITH existing project files, not a generator that replaces them.

```
Existing CLAUDE.md (project's own)  →  Patina preserves
                                       Augments minimally if needed
                                       Backs up before modifying
```

**Philosophy:** Embrace existing CLAUDE.md/GEMINI.md files. They're productive for their projects. The real value is MCP (Phase 2), where frontend LLMs query patina dynamically.

### 2. Allowed Frontends Model

Projects control which LLM frontends are permitted. Files exist only for allowed frontends.

```toml
# .patina/config.toml
[frontends]
allowed = ["claude", "gemini"]  # Team decision
default = "claude"
```

```
patina claude  → Allowed? Yes → Launch
patina codex   → Allowed? No  → "codex not in allowed frontends"
```

**Switching is parallel:** Allowed frontends coexist (both .claude/ and .gemini/ exist). Switching doesn't remove files - that's explicit via `patina adapter remove`.

### 3. Two-Tier Config

```
Global Config (~/.patina/config.toml)    Project Config (.patina/config.toml)
├── Detected frontends                    ├── Allowed frontends
├── User's default preference             ├── Project's default
├── Serve settings                        ├── Mode (owner/contrib)
└── Workspace path                        └── Embeddings config
```

### 4. The Patina Branch Model

**Rule: Always work on `patina` branch. Push to main via PR.**

```
patina branch              →  PR  →  main branch
(our workspace)                     (clean for project)
```

This protects against overwriting others' repos and provides clear isolation.

### 5. Frontend Coexistence

Allowed frontends exist in parallel:

```bash
patina claude    # Ensures .claude/ exists, launches claude
patina gemini    # Ensures .gemini/ exists, launches gemini
# Both coexist - team can use different frontends
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
patina adapter list         # Show allowed + available frontends
patina adapter add X        # Add frontend to allowed, create files
patina adapter remove X     # Backup, remove files, update config
patina adapter default X    # Set project default frontend
```

---

## Launch Flow

```
patina claude
    │
    ├─► Is claude detected? (global config)
    │   └─► No? → "claude CLI not found"
    │
    ├─► Is this a patina project?
    │   └─► No .patina/? → "Run patina init first"
    │
    ├─► Is claude in allowed frontends? (project config)
    │   └─► No? → "claude not in allowed frontends. Run: patina adapter add claude"
    │
    ├─► Is mothership running?
    │   └─► No? Start: patina serve --daemon
    │
    ├─► Ensure adapter files exist:
    │   ├─► .claude/ missing? Copy from ~/.patina/adapters/claude/templates/
    │   ├─► CLAUDE.md missing? Bootstrap minimal with patina hooks
    │   └─► CLAUDE.md exists? Preserve (maybe add MCP pointer if missing)
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
│   ├── config.toml          # Project config (mode, allowed frontends)
│   ├── context.md           # Patina's project knowledge (optional)
│   └── backups/             # Backups before modifications
├── layer/
│   ├── core/                # Eternal patterns
│   ├── surface/             # Active docs
│   └── sessions/            # Work history
├── CLAUDE.md                # Project's Claude context (committed, preserved)
├── GEMINI.md                # Project's Gemini context (if allowed)
├── .claude/                 # Claude adapter files (if allowed)
└── .gemini/                 # Gemini adapter files (if allowed)
```

**Note:** Frontend files (CLAUDE.md, .claude/) are committed, not gitignored. Patina preserves existing files and only creates what's missing for allowed frontends.

### Project Config Schema

**Note:** Unified config consolidates legacy `config.json` (project metadata) and `config.toml` (embeddings). Migration from old format is automatic.

```toml
# .patina/config.toml - Unified project configuration

[project]
name = "my-project"
mode = "owner"              # owner | contrib
created = "2025-12-05T16:52:27Z"

[dev]
type = "docker"             # docker | native
version = "0.1.0"

[frontends]
allowed = ["claude", "gemini"]
default = "claude"

[embeddings]
model = "e5-base-v2"

# Optional: environment snapshot (for doctor command)
[environment]
os = "macos"
arch = "aarch64"
detected_tools = ["cargo", "git", "docker"]
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

### Branch Safety: Do and Inform

Patina enforces the patina branch model but helps rather than blocks. Philosophy: **do it and inform** rather than **warn and block**.

#### For `patina init`

| Scenario | Action | Output |
|----------|--------|--------|
| On patina, up to date | Continue | "✓ Already on patina branch" |
| On patina, behind main | Auto-rebase | "📥 Rebasing onto main... ✓" |
| On main/other, clean | Create/switch | "🌱 Creating patina... ✓" |
| On main/other, dirty | Stash → create/switch | "📦 Stashing... 🌱 Creating... 💡 restore hint" |
| `--force` flag | Backup → recreate | "🗑️ Backed up patina → patina-backup-{ts}" |

#### For `patina claude` (launcher)

| Scenario | Action | Output |
|----------|--------|--------|
| On patina | Generate + launch | (proceed) |
| On other, clean, patina exists | Switch → generate → launch | "🔀 Switching to patina..." |
| On other, dirty, patina exists | Stash → switch → generate → launch | "📦 Stashing... 🔀 Switching... 💡 restore hint" |
| No patina branch | Error | "Run patina init first" |
| No .patina/ directory | Error | "Run patina init first" |

#### Stash Restore Hint

When auto-stashing, always show restore instructions:

```
────────────────────────────────────────────────
💡 Your changes on 'main' are stashed.
   To restore: git checkout main && git stash pop
────────────────────────────────────────────────
```

#### Why Not Auto-Unstash?

After launch exits, user stays on patina branch. This is intentional:
- Patina branch is where AI work happens
- Stash is waiting if they need it
- Simple, predictable behavior

#### The `--force` Flag

Normal mode preserves existing patina branch. `--force` is for nuclear reset:

```bash
patina init . --force

🗑️  Backing up existing patina branch...
   ✓ Renamed patina → patina-backup-20251209-143022
🌱 Creating fresh patina branch from 'main'...
   ✓ Created and switched to patina branch
```

Use when patina branch is corrupted or you want to start completely fresh.

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

### Phase 1: Launcher & Adapters

```
1a: Template Centralization ✓
    - ~/.patina/adapters/ structure
    - Embedded templates extraction
    - Parity across frontends

1b: First-Run Setup ✓
    - ~/.patina/ structure
    - Frontend detection (enum-based)
    - Default frontend selection

1c: Launcher Command
    - `patina [path] [frontend]` as default behavior
    - Auto-start mothership
    - Allowed frontends enforcement

1d: Patina Context Layer
    - Preserve existing CLAUDE.md/GEMINI.md
    - Minimal augmentation (MCP pointers)
    - Backup before modification

1e: Project Config & Allowed Frontends
    - .patina/config.toml with [project] and [frontends]
    - Allowed list controls which frontends have files
    - Owner vs contrib mode

1f: Branch Model & Safety
    - Always work on patina branch
    - Auto-stash, auto-switch
    - CI stripping for contrib repos

1g: Adapter Commands
    - patina adapter add/remove/list/default
```

### Phase 2: MCP Integration

```
2a: MCP server in mothership
    - Add MCP to patina serve
    - stdio interface for frontends

2b: Core MCP tools
    - patina_context (combines global + project rules)
    - patina_scry
    - patina_session_*

2c: Workspace MCP tools
    - patina_workspace_list
    - Cross-project queries
```

---

## Summary Table

| Aspect | Design |
|--------|--------|
| Launcher | `patina [frontend]` (implicit, no subcommand) |
| Frontends | Enum (claude, gemini, codex) - simple, type-safe |
| Allowed frontends | `.patina/config.toml [frontends].allowed` |
| Existing files | Preserved, not clobbered |
| Global config | `~/.patina/config.toml` (detected frontends, user default) |
| Project config | `.patina/config.toml` (allowed frontends, mode) |
| Branch model | Always `patina` branch, PR to main |
| Owner repos | PR includes patina artifacts |
| Contrib repos | CI strips patina artifacts |
| Mothership | `patina serve` (HTTP + MCP, one process) |
| Switching | Parallel (allowed frontends coexist) |
| YOLO | Container connects to host mothership |

---

## Validation Criteria

| Validation | Status |
|------------|--------|
| `patina` opens project in default frontend (if allowed) | [ ] |
| `patina claude` opens Claude Code (if allowed) | [ ] |
| `patina gemini` opens Gemini CLI (if allowed) | [ ] |
| Non-allowed frontend shows clear error message | [ ] |
| Existing CLAUDE.md preserved, not clobbered | [ ] |
| `patina adapter add/remove` manages allowed list | [ ] |
| Files exist only for allowed frontends | [ ] |
| Mothership auto-starts if not running | [ ] |
| MCP tools work from any frontend | [ ] |
| Owner mode: patina artifacts in main | [ ] |
| Contrib mode: CI strips artifacts | [ ] |
| Backups created before modifying existing files | [ ] |
