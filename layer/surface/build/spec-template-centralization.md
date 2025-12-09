# Spec: Template Centralization

**Status:** Design Complete
**Session:** 20251209-112825
**Phase:** 5 (Launcher & Adapters)
**Depends On:** spec-launcher-architecture.md

---

## Problem Statement

Templates are currently:
1. Embedded in binary via `include_str!()`
2. Written directly to project's `.claude/` during `patina init`
3. `~/.patina/adapters/` created but left **empty**
4. Gemini has no templates (stub implementation)
5. Init and launch have no shared template source

This causes:
- Claude-centric design (Gemini neglected)
- No way for users to customize templates
- No central source of truth for adapter files
- Duplicate logic between init and launch

---

## Solution

Extract embedded templates to `~/.patina/adapters/{frontend}/templates/` on first run. Both init and launch pull from this central location.

```
┌─────────────────────────────────────────────────────────────────┐
│                    COMPILE TIME                                 │
│                                                                 │
│  resources/claude/.claude/*  ──► include_str!() ──► binary     │
│  resources/gemini/.gemini/*  ──► include_str!() ──► binary     │
└─────────────────────────────────────────────────────────────────┘
                              │
                              │ First run: workspace::setup()
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                 ~/.patina/adapters/                             │
│                 (CENTRAL TEMPLATES)                             │
│                                                                 │
│  claude/templates/                gemini/templates/             │
│  ├── .claude/                     ├── .gemini/                  │
│  │   ├── bin/                     │   ├── bin/                  │
│  │   │   ├── session-start.sh    │   │   ├── session-start.sh │
│  │   │   └── ...                  │   │   └── ...               │
│  │   ├── commands/                │   ├── commands/             │
│  │   │   ├── session-start.md    │   │   ├── session-start.md │
│  │   │   └── ...                  │   │   └── ...               │
│  │   └── CLAUDE.md                │   └── GEMINI.md             │
│  └── context.md.template          └── context.md.template       │
└─────────────────────────────────────────────────────────────────┘
                              │
              ┌───────────────┴───────────────┐
              │                               │
              ▼                               ▼
┌─────────────────────────┐     ┌─────────────────────────┐
│     patina init         │     │     patina claude       │
│                         │     │                         │
│  Copy .claude/ to       │     │  Ensure .claude/ exists │
│  project from central   │     │  Generate CLAUDE.md     │
└─────────────────────────┘     └─────────────────────────┘
```

---

## Directory Structure

### Global Templates (`~/.patina/adapters/`)

```
~/.patina/adapters/
├── claude/
│   └── templates/
│       ├── .claude/
│       │   ├── bin/
│       │   │   ├── session-start.sh
│       │   │   ├── session-update.sh
│       │   │   ├── session-note.sh
│       │   │   ├── session-end.sh
│       │   │   ├── launch.sh
│       │   │   └── persona-start.sh
│       │   ├── commands/
│       │   │   ├── session-start.md
│       │   │   ├── session-update.md
│       │   │   ├── session-note.md
│       │   │   ├── session-end.md
│       │   │   ├── launch.md
│       │   │   ├── persona-start.md
│       │   │   └── patina-review.md
│       │   ├── context/
│       │   │   └── sessions/
│       │   └── CLAUDE.md
│       └── context.md.template
├── gemini/
│   └── templates/
│       ├── .gemini/
│       │   ├── bin/
│       │   │   └── (same scripts, adapted paths)
│       │   └── commands/
│       │       └── *.toml (TOML format, not markdown)
│       ├── GEMINI.md            # NOTE: At root, NOT inside .gemini/
│       └── context.md.template
└── codex/
    └── templates/
        └── (future)
```

### Source Resources (`resources/`)

```
resources/
├── claude/
│   ├── .claude/           # Full template structure
│   │   ├── bin/
│   │   ├── commands/
│   │   └── CLAUDE.md
│   └── context.md.template
├── gemini/                # NEW - must create
│   ├── .gemini/
│   │   ├── bin/           # Shell scripts (same as claude)
│   │   └── commands/      # TOML files (*.toml, not *.md)
│   ├── GEMINI.md          # At root, NOT inside .gemini/
│   └── context.md.template
└── templates/
    └── (docker, devcontainer - unchanged)
```

---

## Implementation

### Step 1: Create Gemini Templates

Create `resources/gemini/` with structure adapted for Gemini CLI conventions.

#### Key Differences: Claude vs Gemini

| Aspect | Claude Code | Gemini CLI |
|--------|-------------|------------|
| Context file | `CLAUDE.md` (root or `.claude/`) | `GEMINI.md` (root only, NOT inside `.gemini/`) |
| Config dir | `.claude/` | `.gemini/` |
| Commands format | Markdown (`.md`) | **TOML (`.toml`)** |
| Global config | `~/.claude/settings.json` | `~/.gemini/settings.json` |
| Scripts dir | `.claude/bin/` | `.gemini/bin/` (same) |

#### Gemini TOML Command Format

```toml
# .gemini/commands/session-start.toml
description = "Start a new patina development session"
prompt = """
Execute the session start script:
.gemini/bin/session-start.sh {{args}}

Then read the active session file and summarize...
"""
```

Features:
- `{{args}}` - placeholder for user input after command
- `!{shell command}` - shell injection (with confirmation prompt)
- Subdirectories create namespaces: `git/commit.toml` → `/git:commit`

#### Files to Create

```bash
# Shell scripts (same logic as claude, different paths)
resources/gemini/.gemini/bin/session-start.sh
resources/gemini/.gemini/bin/session-update.sh
resources/gemini/.gemini/bin/session-note.sh
resources/gemini/.gemini/bin/session-end.sh

# Commands in TOML format (NOT markdown)
resources/gemini/.gemini/commands/session-start.toml
resources/gemini/.gemini/commands/session-update.toml
resources/gemini/.gemini/commands/session-note.toml
resources/gemini/.gemini/commands/session-end.toml
resources/gemini/.gemini/commands/patina-review.toml

# Context file at ROOT level (not inside .gemini/)
resources/gemini/GEMINI.md
```

**Note:** Shell scripts are LLM-agnostic (manage git, files, timestamps). Only change paths and display names. Commands need conversion from markdown prompts to TOML format.

### Step 2: Template Extraction Module

**New file:** `src/adapters/templates.rs`

```rust
//! Template extraction and management
//!
//! Handles extracting embedded templates to ~/.patina/adapters/
//! and copying templates to projects.

use anyhow::Result;
use std::path::Path;

// Embed all templates at compile time
mod embedded {
    // Claude
    pub const CLAUDE_SESSION_START_SH: &str =
        include_str!("../../resources/claude/.claude/bin/session-start.sh");
    // ... all claude templates

    // Gemini
    pub const GEMINI_SESSION_START_SH: &str =
        include_str!("../../resources/gemini/.gemini/bin/session-start.sh");
    // ... all gemini templates
}

/// Extract all templates to ~/.patina/adapters/
pub fn install_all(adapters_dir: &Path) -> Result<()> {
    install_claude_templates(adapters_dir)?;
    install_gemini_templates(adapters_dir)?;
    Ok(())
}

/// Copy adapter templates to project
pub fn copy_to_project(frontend: &str, project_path: &Path) -> Result<()> {
    let templates_dir = crate::workspace::adapters_dir()
        .join(frontend)
        .join("templates");

    let adapter_dir = format!(".{}", frontend);
    let src = templates_dir.join(&adapter_dir);
    let dest = project_path.join(&adapter_dir);

    copy_dir_recursive(&src, &dest)?;
    Ok(())
}

/// Recursive directory copy with executable permissions
fn copy_dir_recursive(src: &Path, dest: &Path) -> Result<()> {
    // Implementation
}
```

### Step 3: Update Workspace Setup

**File:** `src/workspace/internal.rs`

```rust
use crate::adapters::templates;

pub fn setup() -> Result<SetupResult> {
    // ... existing code ...

    // After creating adapter directories:
    println!("  ✓ Installing adapter templates...");
    templates::install_all(&adapters)?;

    // ... rest of setup ...
}
```

### Step 4: Refactor Claude Adapter

**File:** `src/adapters/claude/internal/session_scripts.rs`

Change from direct `include_str!()` writes to:

```rust
use crate::adapters::templates;

pub fn create_session_scripts(project_path: &Path) -> Result<()> {
    templates::copy_to_project("claude", project_path)
}
```

### Step 5: Implement Gemini Adapter

**File:** `src/adapters/gemini/internal/mod.rs`

Mirror claude structure:

```rust
pub fn init_project(
    project_path: &Path,
    project_name: &str,
    environment: &Environment,
) -> Result<()> {
    // Create directory structure
    paths::create_directory_structure(project_path)?;

    // Copy templates from central location
    crate::adapters::templates::copy_to_project("gemini", project_path)?;

    // Generate initial context
    context_generation::generate_initial_context(project_path, project_name, environment)?;

    Ok(())
}
```

### Step 6: Update Launch Command

**File:** `src/commands/launch/internal.rs`

```rust
fn ensure_adapter_files(frontend: &str, project_path: &Path) -> Result<()> {
    let adapter_dir = project_path.join(format!(".{}", frontend));

    if !adapter_dir.exists() {
        println!("  ✓ Installing {} templates", frontend);
        crate::adapters::templates::copy_to_project(frontend, project_path)?;
    }

    Ok(())
}
```

Add to launch flow (after project check, before presentation generation):
```rust
// Step 7: Ensure adapter files exist
ensure_adapter_files(&frontend_name, &project_path)?;

// Step 8: Generate presentation (CLAUDE.md from context.md)
generate_presentation(&frontend_name, &project_path)?;
```

---

## Backward Compatibility

### Existing Projects

Projects initialized before this change have `.claude/` created by old init. Options:

1. **Leave alone** - Old templates continue to work
2. **Detect version** - Check `adapter-manifest.json` version
3. **Offer upgrade** - `patina upgrade` refreshes templates

**Recommendation:** Option 1 for now. Old projects work. New projects get new behavior. Add upgrade command in future phase.

### Migration Path

```bash
# For users who want latest templates in existing project:
rm -rf .claude/
patina claude  # Re-creates from central templates
```

---

## Validation Criteria

| Validation | Status |
|------------|--------|
| First run extracts templates to `~/.patina/adapters/` | [ ] |
| `patina init --llm=claude` copies from central templates | [ ] |
| `patina init --llm=gemini` copies from central templates | [ ] |
| `patina claude` ensures `.claude/` exists | [ ] |
| `patina gemini` ensures `.gemini/` exists | [ ] |
| Gemini has full template parity with Claude | [ ] |
| Session scripts work identically for both frontends | [ ] |
| Users can customize `~/.patina/adapters/` templates | [ ] |

---

## Files Changed

| File | Change |
|------|--------|
| `resources/gemini/` | NEW - Create gemini templates |
| `src/adapters/templates.rs` | NEW - Template extraction module |
| `src/adapters/mod.rs` | Add `pub mod templates` |
| `src/workspace/internal.rs` | Call `templates::install_all()` in setup |
| `src/adapters/claude/internal/session_scripts.rs` | Use `templates::copy_to_project()` |
| `src/adapters/gemini/internal/mod.rs` | Full implementation |
| `src/commands/launch/internal.rs` | Add `ensure_adapter_files()` |

---

## Open Questions

1. **Codex templates** - Create placeholder or wait until Codex CLI exists?
2. **Template versioning** - How to handle template updates across patina versions?
3. **Custom templates** - Document how users can modify `~/.patina/adapters/`?
