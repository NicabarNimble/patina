---
id: spec-adapter-non-destructive
status: design
created: 2026-01-18
tags: [spec, adapter, implementation]
references: [unix-philosophy, dependable-rust, adapter-pattern]
---

# Spec: Non-Destructive Adapter Setup

**Problem:** `adapter add/refresh` overwrites root CLAUDE.md.

**Solution:** Namespace Patina content. Never touch what we don't own.

---

## Why This Matters

All three CLI tools (Claude Code, Gemini CLI, OpenCode) use layered configuration systems where projects, teams, and enterprises contribute settings. When Patina overwrites bootstrap files (CLAUDE.md, GEMINI.md, AGENTS.md), we destroy existing instructions and break workflows.

The fix: operate as a **good citizen** by namespacing our content under `patina/` subdirectories and `patina-*` prefixes. This lets us coexist with:
- Enterprise-managed configurations
- Team-specific commands and rules
- Other tools following the same pattern

**Key insight:** Each CLI has different directory conventions, but all support:
- `@import` syntax for file inclusion
- Subdirectories for organization
- MCP server integration

---

## Core Values Applied

| Pattern | Application |
|---------|-------------|
| **Unix Philosophy** | Clear ownership boundaries. Patina owns `patina/` subdirs, nothing else. |
| **Dependable Rust** | Small interface: one @import line. Implementation hidden in rules/. |
| **Adapter Pattern** | Same namespacing works for claude/gemini/opencode. |

---

## Ownership Model

**Principle:** We own `patina/` subdirectories and `patina-*` prefixed items. Everything else is theirs.

```
# Claude Code
.claude/
├── rules/patina/        ← OURS
├── commands/patina/     ← OURS
├── skills/patina-*/     ← OURS (prefix)
├── bin/patina/          ← OURS
├── context/             ← SHARED
└── */                   ← THEIRS
CLAUDE.md                ← THEIRS

# Gemini CLI
.gemini/
├── patina/              ← OURS (no rules/ in Gemini)
├── commands/patina/     ← OURS
├── bin/patina/          ← OURS
├── context/             ← SHARED
└── */                   ← THEIRS
GEMINI.md                ← THEIRS

# OpenCode
.opencode/
├── skills/patina-*/     ← OURS (same as Claude, prefix pattern)
├── commands/patina/     ← OURS
├── bin/patina/          ← OURS
├── context/             ← SHARED
└── */                   ← THEIRS
AGENTS.md                ← THEIRS
```

---

## Add vs Refresh Behavior

| Operation | Bootstrap exists? | Behavior |
|-----------|-------------------|----------|
| `adapter add` | No | Create with @import to patina content |
| `adapter add` | Yes | Prompt: append PATINA section, skip, or show preview |
| `adapter refresh` | Any | Never touch. User owns it. |

Bootstrap files: `CLAUDE.md`, `GEMINI.md`, `AGENTS.md` (per adapter)

---

## Code Changes

### 1. `resources/claude/` - Reorganize Templates

```
resources/claude/
├── rules/patina/           # NEW
│   ├── README.md           # @import target
│   ├── mcp-tools.md
│   └── sessions.md
├── bin/patina/             # MOVED from bin/
│   ├── session-start.sh
│   ├── session-update.sh
│   ├── session-note.sh
│   └── session-end.sh
├── commands/patina/        # MOVED from commands/
│   ├── session-start.md
│   ├── session-update.md
│   ├── session-note.md
│   ├── session-end.md
│   └── patina-review.md
└── skills/patina-beliefs/  # RENAMED from epistemic-beliefs/
    └── ...
```

#### Rule File Content

**`rules/patina/README.md`**:
```markdown
# Patina Knowledge Management

This project uses Patina for codebase knowledge.

@./mcp-tools.md
@./sessions.md

---
*Managed by Patina. Do not edit - changes will be overwritten on refresh.*
```

**`rules/patina/mcp-tools.md`**:
```markdown
## MCP Tools

Use these tools for codebase questions:

- **`scry`** - Search codebase knowledge. USE FIRST for any code question.
- **`context`** - Get project patterns. USE before architectural changes.
- **`assay`** - Query structure (imports, callers, modules).

These search pre-indexed knowledge - faster than manual file exploration.
```

**`rules/patina/sessions.md`**:
```markdown
## Session Workflow

- `/session-start [name]` - Begin session with Git tracking
- `/session-update` - Capture progress
- `/session-note [insight]` - Record insight
- `/session-end` - Archive & distill learnings

Session state: `.claude/context/active-session.md`
Archives: `layer/sessions/`
```

### 2. `src/adapters/templates.rs`

**Add includes** (~line 35):
```rust
pub const RULES_README_MD: &str = include_str!("../../resources/claude/rules/patina/README.md");
pub const RULES_MCP_TOOLS_MD: &str = include_str!("../../resources/claude/rules/patina/mcp-tools.md");
pub const RULES_SESSIONS_MD: &str = include_str!("../../resources/claude/rules/patina/sessions.md");
```

**Update `install_claude_templates()`** (lines 131-207):
```rust
// NEW: rules directory
let rules_dir = claude_dir.join("rules").join("patina");
fs::create_dir_all(&rules_dir)?;
fs::write(rules_dir.join("README.md"), claude_templates::RULES_README_MD)?;
fs::write(rules_dir.join("mcp-tools.md"), claude_templates::RULES_MCP_TOOLS_MD)?;
fs::write(rules_dir.join("sessions.md"), claude_templates::RULES_SESSIONS_MD)?;

// CHANGED: namespaced commands
let commands_dir = claude_dir.join("commands").join("patina");

// CHANGED: prefixed skills
let beliefs_dir = skills_dir.join("patina-beliefs");
```

### 3. `src/adapters/launch.rs`

**Replace `generate_bootstrap()`** (lines 167-178):
```rust
pub fn generate_bootstrap(name: &str, project_path: &Path) -> Result<()> {
    let adapter = Adapter::from_name(name).ok_or_else(|| anyhow!("Unknown adapter: {}", name))?;
    let bootstrap_path = project_path.join(adapter.bootstrap_file());

    if bootstrap_path.exists() {
        append_patina_section_if_missing(&bootstrap_path)?;
    } else {
        fs::write(&bootstrap_path, new_project_bootstrap(&adapter))?;
    }
    Ok(())
}

fn append_patina_section_if_missing(path: &Path) -> Result<()> {
    let content = fs::read_to_string(path)?;
    if content.contains("<!-- PATINA:START") {
        return Ok(()); // Already present
    }
    // TODO: prompt user, then append PATINA section
    Ok(())
}

const PATINA_SECTION: &str = r#"
<!-- PATINA:START -->
## Patina
@.claude/rules/patina/README.md
<!-- PATINA:END -->
"#;
```

**Remove `bootstrap_content()`** (lines 335-360) - no longer needed.

### 4. `src/commands/adapter.rs`

**Replace refresh logic** (lines 357-380):
```rust
fn refresh(name: &str, no_commit: bool) -> Result<()> {
    // ...

    // CHANGED: Only remove what we own
    remove_patina_owned(&adapter_dir)?;

    // Copy fresh templates (now namespaced)
    patina::adapters::templates::copy_to_project(name, &cwd)?;

    // REMOVED: generate_bootstrap() - don't touch CLAUDE.md on refresh

    println!("CLAUDE.md: Not modified (user-owned)");
}

fn remove_patina_owned(adapter_dir: &Path) -> Result<()> {
    let paths = ["rules/patina", "commands/patina", "bin/patina"];
    for p in paths {
        let full = adapter_dir.join(p);
        if full.exists() {
            fs::remove_dir_all(&full)?;
        }
    }
    // Skills: remove patina-* prefix
    let skills = adapter_dir.join("skills");
    if skills.exists() {
        for entry in fs::read_dir(&skills)? {
            let entry = entry?;
            if entry.file_name().to_string_lossy().starts_with("patina-") {
                fs::remove_dir_all(entry.path())?;
            }
        }
    }
    Ok(())
}
```

**Update `preserve_user_files()`** - simplify since we no longer nuke everything.

---

## Migration

On refresh, detect old layout and migrate:

```rust
fn needs_migration(adapter_dir: &Path) -> bool {
    adapter_dir.join("commands/session-start.md").exists()  // flat = old
}

fn migrate_to_namespaced(adapter_dir: &Path) -> Result<()> {
    // Move commands/*.md → commands/patina/
    // Rename skills/epistemic-beliefs → skills/patina-beliefs
    println!("⚠️  Migrated to namespaced layout");
    println!("   Your CLAUDE.md is unchanged. Add: @.claude/rules/patina/README.md");
}
```

---

## Adapter-Specific Details

### Comparison Matrix

| Feature | Claude Code | Gemini CLI | OpenCode |
|---------|-------------|------------|----------|
| **Config Dir** | `.claude/` | `.gemini/` | `.opencode/` |
| **Bootstrap File** | `CLAUDE.md` | `GEMINI.md` | `AGENTS.md` |
| **Global Config** | `~/.claude/` | `~/.gemini/` | `~/.config/opencode/` |
| **Commands Dir** | `.claude/commands/*.md` | `.gemini/commands/*.toml` | `.opencode/commands/*.md` |
| **Rules/Context** | `.claude/rules/` | Hierarchical GEMINI.md | Via AGENTS.md |
| **Skills** | `.claude/skills/` | Extensions + MCP | `.opencode/skills/` (+ `.claude/skills/` fallback) |
| **Agents** | `.claude/agents/` | N/A | `.opencode/agent/` |
| **@import syntax** | `@file.md` | `@file.md` | `@file` |
| **Hierarchy** | Enterprise→Project→Rules→User | Global→Ancestors→Subdirs | Global→Project |

### Universal Elements

All three adapters share:
- `@import` syntax for file inclusion
- MCP server support
- Shell scripts in `bin/`
- Session context in `context/`

---

### Claude Code Adapter

**Full support for namespacing.** Claude Code discovers `rules/`, `commands/`, `skills/` subdirectories.

```
.claude/
├── rules/patina/           # Auto-discovered by Claude Code
├── commands/patina/        # Slash commands: /patina/session-start
├── skills/patina-beliefs/  # Skill with patina- prefix
├── bin/patina/             # Shell scripts
└── context/                # Session state (shared)

CLAUDE.md                   # User-owned, offer @import
```

**Bootstrap integration:**
```markdown
<!-- PATINA:START -->
@.claude/rules/patina/README.md
<!-- PATINA:END -->
```

---

### Gemini CLI Adapter

**Different architecture.** Gemini CLI doesn't have `rules/` or structured `commands/` directories. It uses:
- Hierarchical GEMINI.md scanning (ancestors + subdirs)
- TOML format for commands
- Extensions system instead of skills

```
.gemini/
├── patina/                 # Our namespace (custom, not auto-discovered)
│   ├── context.md          # @import target with MCP + session info
│   ├── mcp-tools.md
│   └── sessions.md
├── commands/patina/        # TOML commands
│   ├── session-start.toml
│   └── ...
├── bin/patina/             # Shell scripts
└── context/                # Session state (shared)

GEMINI.md                   # User-owned, offer @import
```

**Key difference:** No `rules/` auto-discovery. We create `.gemini/patina/` as our content namespace and rely on @import from GEMINI.md.

**Bootstrap integration:**
```markdown
<!-- PATINA:START -->
@.gemini/patina/context.md
<!-- PATINA:END -->
```

**Command format (TOML):**
```toml
# .gemini/commands/patina/session-start.toml
description = "Start a new Patina development session"
prompt = """
Execute: `.gemini/bin/patina/session-start.sh {{args}}`
...
"""
```

---

### OpenCode Adapter

**Claude-compatible structure.** OpenCode supports `CLAUDE.md` as fallback and has similar directory conventions.

```
.opencode/
├── skills/patina-beliefs/  # Skills (same as Claude, compatible spec)
│   └── SKILL.md
├── commands/patina/        # Markdown commands (like Claude)
│   ├── session-start.md
│   └── ...
├── bin/patina/             # Shell scripts
└── context/                # Session state (shared)

AGENTS.md                   # Primary (or CLAUDE.md fallback)
```

**Key points:**
- Has both `skills/` AND `agent/` (like Claude Code)
- Skills follow Anthropic Agent Skills Specification
- Discovers `.claude/skills/` as fallback → our Claude skills work here too
- Primary file is `AGENTS.md` (falls back to `CLAUDE.md`)
- No `rules/` directory - use AGENTS.md content

**Bootstrap integration:**
```markdown
<!-- PATINA:START -->
## Patina
Use `scry` for code questions, `context` for patterns.
Session commands: /session-start, /session-update, /session-note, /session-end
<!-- PATINA:END -->
```

---

### Template Structure by Adapter

**`resources/claude/`:**
```
├── rules/patina/README.md, mcp-tools.md, sessions.md
├── commands/patina/*.md
├── skills/patina-beliefs/
└── bin/patina/*.sh
```

**`resources/gemini/`:**
```
├── patina/context.md, mcp-tools.md, sessions.md    # Note: patina/ not rules/patina/
├── commands/patina/*.toml                           # TOML format
└── bin/patina/*.sh
```

**`resources/opencode/`:**
```
├── skills/patina-beliefs/                           # Same as Claude (OpenCode is compatible)
├── commands/patina/*.md
└── bin/patina/*.sh
```

---

### `templates.rs` Adapter-Specific Updates

```rust
// Claude: full structure
fn install_claude_templates(adapters_dir: &Path) -> Result<()> {
    let claude_dir = templates_dir.join(".claude");
    let rules_dir = claude_dir.join("rules").join("patina");
    let commands_dir = claude_dir.join("commands").join("patina");
    let skills_dir = claude_dir.join("skills").join("patina-beliefs");
    let bin_dir = claude_dir.join("bin").join("patina");
    // ...
}

// Gemini: no rules/, use patina/ directly
fn install_gemini_templates(adapters_dir: &Path) -> Result<()> {
    let gemini_dir = templates_dir.join(".gemini");
    let patina_dir = gemini_dir.join("patina");           // Not rules/patina
    let commands_dir = gemini_dir.join("commands").join("patina");
    let bin_dir = gemini_dir.join("bin").join("patina");
    // No skills - Gemini uses extensions
}

// OpenCode: same skills/ structure (Claude-compatible)
fn install_opencode_templates(adapters_dir: &Path) -> Result<()> {
    let opencode_dir = templates_dir.join(".opencode");
    let commands_dir = opencode_dir.join("commands").join("patina");
    let skills_dir = opencode_dir.join("skills").join("patina-beliefs");  // Same as Claude
    let bin_dir = opencode_dir.join("bin").join("patina");
    // No rules - use AGENTS.md content
}
```

---

### `remove_patina_owned()` Adapter Variants

```rust
fn remove_patina_owned(adapter_dir: &Path, adapter_name: &str) -> Result<()> {
    // Common paths
    let common = ["commands/patina", "bin/patina"];
    for p in common {
        remove_if_exists(&adapter_dir.join(p))?;
    }

    // Adapter-specific paths
    match adapter_name {
        "claude" => {
            remove_if_exists(&adapter_dir.join("rules/patina"))?;
            remove_prefixed(&adapter_dir.join("skills"), "patina-")?;
        }
        "gemini" => {
            remove_if_exists(&adapter_dir.join("patina"))?;  // .gemini/patina/
        }
        "opencode" => {
            remove_prefixed(&adapter_dir.join("skills"), "patina-")?;  // Same as Claude
        }
        _ => {}
    }
    Ok(())
}
```

---

## Skills vs Agents: Concepts

Both Claude Code and OpenCode have **skills AND agents** as separate concepts.

### What's What

| Concept | Purpose | Location (Claude) | Location (OpenCode) |
|---------|---------|-------------------|---------------------|
| **Skills** | Reusable knowledge/instructions | `.claude/skills/` | `.opencode/skills/` |
| **Agents** | Isolated execution contexts | `.claude/agents/` | `.opencode/agent/` |
| **Commands** | Slash-invoked prompts | `.claude/commands/` | `.opencode/command/` |

### Skills = Knowledge

Skills add expertise to conversation. Auto-discovered via description.
```yaml
# .claude/skills/patina-beliefs/SKILL.md
---
name: patina-beliefs
description: Create epistemic beliefs from session learnings.
             Use when synthesizing decisions into formal beliefs.
allowed-tools: Read, Write, Bash(./scripts/*:*)
---
```

OpenCode skills follow same SKILL.md spec (Anthropic Agent Skills Specification).
OpenCode also discovers `.claude/skills/` as fallback → **our skills work in both**.

### Agents = Isolation

Agents run in separate context with own tools. Explicitly invoked.
```yaml
# .claude/agents/reviewer.md (Claude)
---
name: reviewer
description: Code review with security focus
allowed-tools: Read, Grep, Glob
---

# .opencode/agent/reviewer.md (OpenCode)
---
description: Code review with security focus
mode: subagent
tools: [Read, Grep, Glob]
---
```

### For Patina

We use **skills** (not agents) for beliefs because:
- Skills are reusable knowledge (what beliefs are)
- OpenCode is Claude-compatible for skills
- Same SKILL.md works in both

```
# Both adapters use same structure:
.claude/skills/patina-beliefs/SKILL.md
.opencode/skills/patina-beliefs/SKILL.md  # Or discovers .claude/skills/
```

**Frontmatter differences:**

| Field | Claude | OpenCode |
|-------|--------|----------|
| `name` | Required | Required |
| `description` | Required | Required |
| `allowed-tools` | Claude-specific | Use `tools` in agent, not skill |
| `context: fork` | Claude-specific | N/A (skills don't fork) |

Since OpenCode skills follow the same spec, we may be able to use **identical SKILL.md** for both adapters.

---

## Session Script Path Updates

Verify shell scripts don't have hardcoded paths that break with namespacing.

**Check in `resources/*/bin/patina/*.sh`:**
```bash
# OLD (might exist):
COMMANDS_DIR=".claude/commands"

# NEW (if referenced):
COMMANDS_DIR=".claude/commands/patina"
```

**Files to audit:**
- `session-start.sh` - creates session files
- `session-end.sh` - archives sessions
- `create-belief.sh` - skill script

Most scripts write to `.claude/context/` which is unchanged, but verify.

---

## Test Cases

### Unit Tests (`templates.rs`)

```rust
#[test]
fn test_claude_templates_namespaced() {
    let temp = TempDir::new().unwrap();
    install_claude_templates(temp.path()).unwrap();

    let templates = temp.path().join("claude/templates/.claude");

    // Namespaced structure
    assert!(templates.join("rules/patina/README.md").exists());
    assert!(templates.join("commands/patina/session-start.md").exists());
    assert!(templates.join("bin/patina/session-start.sh").exists());
    assert!(templates.join("skills/patina-beliefs/SKILL.md").exists());

    // Old flat structure should NOT exist
    assert!(!templates.join("commands/session-start.md").exists());
    assert!(!templates.join("bin/session-start.sh").exists());
    assert!(!templates.join("skills/epistemic-beliefs").exists());
}
```

### Integration Tests (`adapter.rs`)

```rust
#[test]
fn test_refresh_preserves_user_content() {
    // Setup: project with user's custom command
    // .claude/commands/deploy.md (user-created)

    // Action: patina adapter refresh claude

    // Assert: deploy.md still exists, patina/ updated
}

#[test]
fn test_refresh_never_touches_claude_md() {
    // Setup: project with custom CLAUDE.md
    let original = fs::read_to_string("CLAUDE.md");

    // Action: patina adapter refresh claude

    // Assert: CLAUDE.md unchanged
    assert_eq!(fs::read_to_string("CLAUDE.md"), original);
}

#[test]
fn test_add_prompts_for_existing_claude_md() {
    // Setup: project with existing CLAUDE.md (no PATINA section)

    // Action: patina adapter add claude

    // Assert: prompted user, didn't auto-overwrite
}

#[test]
fn test_migration_moves_flat_to_namespaced() {
    // Setup: old layout with flat commands/

    // Action: patina adapter refresh claude

    // Assert: files moved to commands/patina/
}
```

---

## Checklist

### Phase 1: Claude Adapter
- [ ] Create `resources/claude/rules/patina/` with README.md, mcp-tools.md, sessions.md
- [ ] Move `resources/claude/bin/*.sh` → `bin/patina/`
- [ ] Move `resources/claude/*.md` → `commands/patina/`
- [ ] Rename `skills/epistemic-beliefs` → `patina-beliefs`
- [ ] Update `templates.rs` includes and `install_claude_templates()`
- [ ] Audit session scripts for hardcoded paths

### Phase 2: Gemini Adapter
- [ ] Create `resources/gemini/patina/` with context.md, mcp-tools.md, sessions.md
- [ ] Move `resources/gemini/bin/*.sh` → `bin/patina/`
- [ ] Move `resources/gemini/*.toml` → `commands/patina/`
- [ ] Update `templates.rs` `install_gemini_templates()`
- [ ] Update TOML commands to reference `bin/patina/` paths

### Phase 3: OpenCode Adapter
- [ ] Create `resources/opencode/skills/patina-beliefs/` (same SKILL.md as Claude)
- [ ] Move `resources/opencode/bin/*.sh` → `bin/patina/`
- [ ] Move `resources/opencode/*.md` → `commands/patina/`
- [ ] Update `templates.rs` `install_opencode_templates()`
- [ ] Verify SKILL.md compatibility between Claude and OpenCode

### Phase 4: Core Logic
- [ ] Update `launch.rs` - non-destructive `generate_bootstrap()` per adapter
- [ ] Update `adapter.rs` - `remove_patina_owned()` with adapter variants
- [ ] Update `adapter.rs` - remove `generate_bootstrap()` from refresh flow
- [ ] Add migration detection and `migrate_to_namespaced()`

### Phase 5: Testing
- [ ] Add unit tests for each adapter's namespaced structure
- [ ] Add integration test for non-destructive refresh
- [ ] Add integration test for migration
- [ ] Manual test: Claude with existing CLAUDE.md
- [ ] Manual test: OpenCode with existing AGENTS.md
- [ ] Manual test: Gemini with existing GEMINI.md

---

## References

- [Gemini CLI GEMINI.md docs](https://google-gemini.github.io/gemini-cli/docs/cli/gemini-md.html)
- [Gemini CLI configuration](https://geminicli.com/docs/get-started/configuration/)
- [OpenCode rules](https://opencode.ai/docs/rules/)
- [OpenCode commands](https://opencode.ai/docs/commands/)
- [OpenCode agents](https://opencode.ai/docs/agents/)
