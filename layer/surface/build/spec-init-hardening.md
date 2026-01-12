# Spec: Init, Frontend, and Launcher

**Status:** Design
**Created:** 2026-01-12
**Session:** 20260112-093636
**Blocks:** spec-database-identity.md (UID needs stable init)
**Core References:** [dependable-rust](../../core/dependable-rust.md), [unix-philosophy](../../core/unix-philosophy.md), [adapter-pattern](../../core/adapter-pattern.md)

---

## Problem Statement

`patina init` violates Unix philosophy by doing too much:
- Creates project skeleton (.patina/, layer/)
- Creates frontend files (.claude/)
- Configures MCP
- Runs scrape
- Runs oxidize

This breaks the "one tool, one job" principle and causes:
- Complex failure modes (partial state)
- Re-init destroys frontend config
- Can't add frontends without re-init
- Hard to test and measure individual steps

---

## Design: Three Commands

| Command | "Do X" | Responsibility |
|---------|--------|----------------|
| `patina init` | Create project skeleton | .patina/, layer/ |
| `patina frontend` | Manage frontend integrations | .claude/, MCP |
| `patina` | Launch and work | Invisible glue |

Each does ONE thing. Composition creates the full experience.

---

## Core Values Alignment

### Unix Philosophy

> "One tool, one job, done well"

- `init` creates skeleton (one job)
- `frontend add` creates integration (one job)
- `patina` launches (one job)

### Dependable Rust

> "Do X test: state what it does in one sentence"

- `init`: "Create minimal Patina project structure"
- `frontend add`: "Add frontend integration to project"
- `patina`: "Launch configured frontend"

All pass the "Do X" test.

### Adapter Pattern

> "Trait-based adapters for external systems"

Frontend command manages adapters. Launcher uses them. Clean separation.

---

## Command 1: `patina init`

### Purpose

Create minimal project skeleton. Nothing else.

### Usage

```bash
patina init           # In current directory
patina init .         # Explicit current directory
patina init my-proj   # Create new directory
```

### What It Creates

```
.patina/
├── uid              # Project identity (8 hex chars)
└── config.toml      # Minimal config

layer/
├── core/            # Eternal patterns
├── surface/         # Active patterns
├── dust/            # Historical patterns
└── sessions/        # Session records
```

### Config.toml (Minimal)

```toml
[project]
name = "my-project"
created = "2026-01-12T10:30:00Z"

[frontends]
allowed = []         # Empty! No frontend yet
default = ""

[environment]
os = "macos"
arch = "aarch64"
```

### What It Does NOT Do

- Create frontend directories (.claude/, .gemini/)
- Configure MCP
- Run scrape
- Run oxidize
- Require frontend selection

### Re-init Behavior

```bash
patina init .        # In existing project
```

- Refreshes environment detection
- Preserves ALL config (frontends, upstream, ci)
- Updates version manifest
- Never touches frontend directories

### Implementation

```rust
pub fn execute_init(name: String, no_commit: bool) -> Result<()> {
    // 1. Check hierarchy conflicts
    check_hierarchy_conflicts()?;

    // 2. Create or verify .patina/
    let project_path = setup_project_path(&name)?;
    let patina_dir = project_path.join(".patina");
    fs::create_dir_all(&patina_dir)?;

    // 3. Ensure git repository exists (patina is git-native)
    let git_initialized = ensure_git_repo(&project_path)?;
    if git_initialized {
        println!("✓ Initialized git repository");
    }

    // 4. Create UID (if not exists)
    create_uid_if_missing(&patina_dir)?;

    // 5. Create or merge config
    let is_reinit = create_or_merge_config(&project_path)?;

    // 6. Create layer structure
    Layer::new(&project_path.join("layer")).init()?;

    // 7. Update .gitignore
    update_gitignore(&project_path)?;

    // 8. Log event
    log_event("init_completed", json!({ "reinit": is_reinit }))?;

    // 9. Commit (unless --no-commit)
    if !no_commit {
        let msg = if is_reinit {
            "chore: update Patina configuration"
        } else {
            "chore: initialize Patina project"
        };
        git_commit(&[".gitignore", ".patina/", "layer/"], msg)?;
    }

    // 10. Done
    println!("✓ Initialized Patina project");
    println!("  Add a frontend: patina frontend add <claude|gemini|opencode>");

    Ok(())
}

fn ensure_git_repo(path: &Path) -> Result<bool> {
    if path.join(".git").exists() {
        return Ok(false); // Already exists
    }
    Command::new("git").args(["init"]).current_dir(path).output()?;
    Ok(true) // Initialized
}
```

---

## Command 2: `patina frontend`

### Purpose

Manage frontend integrations. Add, switch, remove, diagnose.

### Subcommands

```bash
patina frontend list              # Show available and configured
patina frontend add <name>        # Add and configure frontend
patina frontend refresh <name>    # Update frontend with backup
patina frontend switch <name>     # Change default frontend
patina frontend remove <name>     # Remove frontend
patina frontend doctor            # Health check all frontends
```

### Supported Frontends

| Name | Directory | Context File | MCP |
|------|-----------|--------------|-----|
| claude | `.claude/` | `CLAUDE.md` | Yes |
| gemini | `.gemini/` | `GEMINI.md` | TBD |
| opencode | `.opencode/` | `AGENTS.md` | Yes |

### `frontend add` Flow

```bash
patina frontend add claude
```

```
1. Verify in patina project
   └─ No .patina/? → "Run 'patina init' first"

2. Verify frontend known
   └─ Unknown? → "Available: claude, gemini, opencode"

3. Check if already added
   └─ Already in allowed? → "claude already configured. Use 'frontend refresh' to update."

4. Check for existing directory
   └─ .claude/ exists but not in allowed? → "Found existing .claude/. Use --force to overwrite."

5. Create adapter directory
   └─ .claude/commands/, .claude/context/, .claude/bin/

6. Create context file
   └─ CLAUDE.md with project info

7. Copy session scripts
   └─ .claude/bin/session-*.sh

8. Update config.toml
   └─ frontends.allowed += "claude"
   └─ frontends.default = "claude" (if first)

9. Configure MCP (best effort)
   └─ claude mcp add patina ...
   └─ If fails: log warning, continue (launcher will fix on first run)

10. Log event
    └─ frontend_added to events.db

11. Commit (unless --no-commit)
    └─ "chore: add claude frontend"

12. Done
    └─ "✓ Added claude. Run 'patina' to start."
```

### `frontend switch` Flow

```bash
patina frontend switch gemini
```

```
1. Verify gemini in frontends.allowed
   └─ Not added? → "Run 'patina frontend add gemini' first"

2. Update config.toml
   └─ frontends.default = "gemini"

3. Verify adapter files exist
   └─ Missing? → Create them

4. Log event
   └─ frontend_switched

5. Done
   └─ "✓ Switched to gemini"
```

### `frontend refresh` Flow

```bash
patina frontend refresh claude
```

```
1. Verify claude in frontends.allowed
   └─ Not configured? → "Run 'patina frontend add claude' first"

2. Extract session files
   └─ .claude/context/*.md → temp
   └─ Print: "✓ Preserved active session" (if session exists)

3. Backup existing frontend
   └─ .claude/ → .backup/claude_<timestamp>

4. Recreate adapter directory
   └─ .claude/commands/, .claude/context/, .claude/bin/

5. Update templates and scripts
   └─ Copy latest from resources

6. Restore session files
   └─ temp → .claude/context/*.md

7. Log event
   └─ frontend_refreshed

8. Commit (unless --no-commit)
   └─ "chore: refresh claude frontend"

9. Done
   └─ "✓ Refreshed claude (backup: .backup/claude_<timestamp>)"
```

### `frontend doctor` Output

```bash
patina frontend doctor
```

```
Frontend Health Check
─────────────────────

claude
  Installed: ✓ v1.0.45
  Adapter:   ✓ v0.3.0
  MCP:       ✓ configured
  Status:    Ready

gemini
  Installed: ✓ v2.1.0
  Adapter:   ✓ v0.1.0
  MCP:       ✗ not configured
  Status:    Run 'patina frontend add gemini' to configure

opencode
  Installed: ✗ not found
  Install:   go install github.com/opencode-ai/opencode@latest
```

### Implementation

```rust
pub fn execute_frontend(cmd: FrontendCommand) -> Result<()> {
    match cmd {
        FrontendCommand::List => list_frontends(),
        FrontendCommand::Add { name, no_commit, force } => add_frontend(&name, no_commit, force),
        FrontendCommand::Refresh { name, no_commit } => refresh_frontend(&name, no_commit),
        FrontendCommand::Switch { name } => switch_frontend(&name),
        FrontendCommand::Remove { name, no_commit } => remove_frontend(&name, no_commit),
        FrontendCommand::Doctor => doctor_frontends(),
    }
}

fn add_frontend(name: &str, no_commit: bool, force: bool) -> Result<()> {
    let project_path = require_patina_project()?;
    let adapter = get_adapter(name)?;
    let mut config = load_config(&project_path)?;

    // Check if already configured
    if config.frontends.allowed.contains(&name.to_string()) {
        println!("{} already configured. Use 'frontend refresh' to update.", name);
        return Ok(());
    }

    // Check for existing directory
    let adapter_dir = adapter.directory(&project_path);
    if adapter_dir.exists() && !force {
        println!("Found existing {}. Use --force to overwrite.", adapter_dir.display());
        return Ok(());
    }

    // Create adapter files
    adapter.init_project(&project_path)?;

    // Update config
    config.frontends.allowed.push(name.to_string());
    if config.frontends.default.is_empty() {
        config.frontends.default = name.to_string();
    }
    save_config(&project_path, &config)?;

    // Configure MCP (best effort - launcher will fix if this fails)
    if let Err(e) = adapter.configure_mcp(&project_path) {
        log_event("mcp_config_deferred", json!({ "frontend": name, "error": e.to_string() }))?;
    }

    // Log event
    log_event("frontend_added", json!({ "frontend": name }))?;

    // Commit
    if !no_commit {
        git_commit(&[&adapter_dir, "CLAUDE.md"], &format!("chore: add {} frontend", name))?;
    }

    println!("✓ Added {}. Run 'patina' to start.", name);
    Ok(())
}

fn refresh_frontend(name: &str, no_commit: bool) -> Result<()> {
    let project_path = require_patina_project()?;
    let adapter = get_adapter(name)?;
    let config = load_config(&project_path)?;

    // Verify configured
    if !config.frontends.allowed.contains(&name.to_string()) {
        println!("{} not configured. Run 'patina frontend add {}' first.", name, name);
        return Ok(());
    }

    let adapter_dir = adapter.directory(&project_path);

    // 1. Extract session files
    let sessions = extract_sessions(&adapter_dir)?;

    // 2. Backup existing
    let backup_path = backup_directory(&adapter_dir)?;

    // 3. Recreate adapter
    adapter.init_project(&project_path)?;

    // 4. Restore sessions
    restore_sessions(&adapter_dir, sessions)?;

    // 5. Log event
    log_event("frontend_refreshed", json!({ "frontend": name }))?;

    // 6. Commit
    if !no_commit {
        git_commit(&[&adapter_dir], &format!("chore: refresh {} frontend", name))?;
    }

    println!("✓ Refreshed {} (backup: {})", name, backup_path.display());
    Ok(())
}
```

---

## Command 3: `patina` (Launcher)

### Purpose

Invisible glue. User types `patina`, magic happens, they're working.

### Usage

```bash
patina                      # Launch default frontend
patina --frontend gemini    # Launch specific frontend (override)
patina -f claude            # Short form
```

### The Magic

User types `patina`. Launcher:

```
1. In a patina project?
   ├─ No  → "Not a Patina project. Run: patina init"
   └─ Yes → continue

2. Frontend configured?
   ├─ No  → "No frontend configured."
   │        "Run: patina frontend add <claude|gemini|opencode>"
   └─ Yes → continue

3. Frontend installed on system?
   ├─ No  → Show install instructions for that frontend
   └─ Yes → continue

4. MCP configured?
   ├─ No  → Configure silently (no user action needed)
   └─ Yes → continue

5. Log event: launch_started

6. Exec into frontend
   └─ Replace process with frontend CLI
```

### Key Principle: Invisible

- No output on success (just launches)
- Only speaks when something needs user action
- Fixes what it can silently (including MCP config that failed during `frontend add`)
- Guides user for what it can't fix (e.g., CLI not installed)

### Implementation

```rust
pub fn execute_launch(frontend_override: Option<String>) -> Result<()> {
    // 1. Require patina project
    let project_path = match find_patina_project() {
        Some(p) => p,
        None => {
            println!("Not a Patina project.");
            println!("Run: patina init");
            return Ok(());
        }
    };

    // 2. Load config, determine frontend
    let config = load_config(&project_path)?;
    let frontend_name = frontend_override
        .unwrap_or_else(|| config.frontends.default.clone());

    if frontend_name.is_empty() {
        println!("No frontend configured.");
        println!("Run: patina frontend add <claude|gemini|opencode>");
        return Ok(());
    }

    // 3. Verify frontend in allowed list
    if !config.frontends.allowed.contains(&frontend_name) {
        println!("Frontend '{}' not configured.", frontend_name);
        println!("Run: patina frontend add {}", frontend_name);
        return Ok(());
    }

    // 4. Check frontend installed
    let frontend = get_frontend_info(&frontend_name)?;
    if !frontend.detected {
        println!("'{}' not installed.", frontend.display);
        println!("Install: {}", frontend.install_cmd);
        return Ok(());
    }

    // 5. Ensure MCP configured (silent)
    let adapter = get_adapter(&frontend_name)?;
    if !adapter.mcp_configured(&project_path)? {
        adapter.configure_mcp(&project_path)?;
    }

    // 6. Log and launch
    log_event("launch_started", json!({
        "frontend": frontend_name,
        "version": frontend.version,
    }))?;

    // 7. Exec (replaces this process)
    exec_frontend(&frontend_name, &project_path)
}
```

---

## Config Model

### Field Classification

| Field | Category | Re-init Behavior |
|-------|----------|------------------|
| `project.name` | Immutable | Preserve |
| `project.created` | Immutable | Preserve |
| `project.uid` | Immutable | Preserve (future) |
| `frontends.allowed` | Preserved | Never overwrite |
| `frontends.default` | Preserved | Never overwrite |
| `upstream` | Preserved | Never overwrite |
| `ci` | Preserved | Never overwrite |
| `environment` | Refreshed | Always update |

### Merge Strategy

```rust
fn create_or_merge_config(project_path: &Path) -> Result<ProjectConfig> {
    let existing = ProjectConfig::load_if_exists(project_path)?;
    let env = Environment::detect()?;

    Ok(ProjectConfig {
        project: ProjectSection {
            name: existing.as_ref()
                .map(|c| c.project.name.clone())
                .unwrap_or_else(|| dir_name(project_path)),
            created: existing.as_ref()
                .and_then(|c| c.project.created.clone())
                .or_else(|| Some(now())),
        },
        frontends: existing.as_ref()
            .map(|c| c.frontends.clone())
            .unwrap_or_default(),  // Empty if new
        upstream: existing.and_then(|c| c.upstream.clone()),
        ci: existing.and_then(|c| c.ci.clone()),
        environment: Some(env.into()),
    })
}
```

---

## Observability (Zero Friction)

### Event Logging

All events logged automatically to `.patina/events.db`:

```sql
CREATE TABLE events (
    id INTEGER PRIMARY KEY,
    timestamp TEXT NOT NULL,
    event_type TEXT NOT NULL,
    frontend TEXT,
    data TEXT
);
```

Event types:
- `init_completed` - with `reinit` flag
- `frontend_added`, `frontend_refreshed`, `frontend_switched`, `frontend_removed`
- `mcp_config_deferred` - MCP config failed, launcher will fix
- `launch_started`, `launch_failed`
- `error` - with error type and message

### Key Metrics

| Metric | What It Measures |
|--------|------------------|
| `init_completed` → `frontend_added` time | Multi-step flow friction |
| `frontend_refreshed` count | Backup/restore usage |
| `launch_failed` by reason | What blocks users |
| `--force` usage frequency | Escape hatch overuse (design smell) |
| `error` by type | Common failure modes |

### Reporting (On Demand)

```bash
patina stats
```

Only shows data when user asks. Never prompts.

---

## User Flows

### New Project

```bash
$ cd my-project
$ patina
Not a Patina project.
Run: patina init

$ patina init
✓ Created .patina/
✓ Created layer/
Add a frontend: patina frontend add <claude|gemini|opencode>

$ patina frontend add claude
✓ Created .claude/
✓ Configured MCP
Run 'patina' to start.

$ patina
[Claude Code launches]
```

### Add Second Frontend

```bash
$ patina frontend add gemini
✓ Created .gemini/
✓ Configured MCP

$ patina frontend list
Configured frontends:
  claude (default)
  gemini

$ patina frontend switch gemini
✓ Default: gemini

$ patina
[Gemini CLI launches]
```

### Quick Override

```bash
$ patina -f claude
[Claude Code launches, doesn't change default]
```

---

## Edge Cases

### Hierarchy Conflicts

| Check | What It Prevents | Severity |
|-------|------------------|----------|
| Parent has `.claude/commands/` | Duplicate slash commands in Claude Code | Critical |
| Child has `.patina/` | Nested projects, config conflicts | Critical |
| `--force` flag | Override both checks | Escape hatch |

**Location**: `patina init`

### Git Setup

| Edge Case | Handling |
|-----------|----------|
| Empty repo (no commits) | Rename current branch to patina |
| Dirty working tree | Block unless `--force` |
| patina branch exists (on it) | Check if behind main |
| patina branch exists (not on it) | Block unless `--force` |
| No origin remote | Create GitHub repo OR local-only |
| Fork detection | Auto-fork if external repo |
| `--local` flag | Skip all GitHub integration |
| Local work with existing GitHub | Preserve in timestamped branch |

**Location**: `patina init`

### Gitignore Handling

| Case | Action |
|------|--------|
| No `.gitignore` | Create with sensible defaults |
| Existing `.gitignore` | Add critical entries (`.patina/`, `*.db`, etc.) |

**Location**: `patina init`

### Pattern Copying

| Check | Purpose |
|-------|---------|
| Is Patina source? | Don't self-overwrite |
| Target exists? | Don't overwrite user patterns |
| Multiple source strategies | Find patterns from dev/installed/home |

**Location**: `patina init`

### Backup Behavior

| What | When | Command |
|------|------|---------|
| `.devcontainer/` → `.devcontainer.backup` | Re-init | `patina init` |
| `.claude/` → `.backup/claude_<timestamp>` | Refresh | `patina frontend refresh` |
| Session files preserved | Refresh | `patina frontend refresh` |

**Note**: `patina init` does not touch frontend directories. Backup/restore of `.claude/` is handled by `frontend refresh`.

### Project Detection

| Check | Purpose |
|-------|---------|
| `name != "." && .patina exists` | Warn about nested init |
| Re-init detection | Different behavior (merge config) |
| Project name from `Cargo.toml` | Better naming |

**Location**: `patina init`

---

## Commit Behavior

Each command auto-commits its changes. Use `--no-commit` to skip.

| Command | Commits | Message |
|---------|---------|---------|
| `patina init` | `.gitignore`, `.patina/`, `layer/` | "chore: initialize Patina project" |
| `patina init` (re-run) | Updated config, refreshed env | "chore: update Patina configuration" |
| `patina frontend add` | `.claude/`, `CLAUDE.md` | "chore: add claude frontend" |
| `patina frontend refresh` | Updated `.claude/` | "chore: refresh claude frontend" |
| `patina frontend remove` | Removes `.claude/` | "chore: remove claude frontend" |

**Design**: Each command is self-contained. User can batch operations with `--no-commit` then commit manually.

---

## Implementation Phases

### Phase 1: Simplify Init

1. Remove frontend creation from init
2. Remove MCP configuration from init
3. Remove scrape/oxidize calls from init
4. Implement config merge strategy
5. Add UID creation

### Phase 2: Frontend Command

1. Implement `frontend add` (with `--force`, `--no-commit`)
2. Implement `frontend refresh` (backup, session preservation)
3. Implement `frontend switch`
4. Implement `frontend list`
5. Implement `frontend remove` (with `--no-commit`)
6. Implement `frontend doctor`
7. Move adapter initialization here

### Phase 3: Launcher

1. Implement `patina` (no subcommand) as launcher
2. Project detection
3. Frontend verification
4. Silent MCP configuration
5. Exec into frontend

### Phase 4: Observability

1. Create events.db schema
2. Add event logging to all commands
3. Implement `patina stats`

---

## Files to Modify

| File | Change |
|------|--------|
| `src/main.rs` | Add launcher as default (no subcommand) |
| `src/commands/init/` | Simplify to skeleton only |
| `src/commands/frontend.rs` | New command (rename from adapter.rs) |
| `src/commands/launch.rs` | New launcher implementation |
| `src/adapters/mod.rs` | Add `configure_mcp()` to trait |

---

## Success Criteria

| Phase | Test |
|-------|------|
| 1 | `patina init` creates only .patina/ and layer/ |
| 1 | Re-init preserves frontends config |
| 1 | `patina init` auto-commits (unless `--no-commit`) |
| 2 | `frontend add claude` creates .claude/ and configures MCP |
| 2 | `frontend add` on existing shows "use refresh" message |
| 2 | `frontend refresh` backs up and preserves sessions |
| 2 | `frontend switch` changes default |
| 3 | `patina` (no args) launches default frontend |
| 3 | Missing frontend shows helpful message |
| 4 | Events logged to events.db |
| 4 | `--force` usage tracked as metric |

---

## Non-Goals

- Scrape/oxidize in init (separate commands)
- Automated A/B testing (human decides)
- Cloud telemetry (local only)
- Codex support (delegate UI, future spec)

---

## References

- [dependable-rust](../../core/dependable-rust.md) - "Do X" test
- [unix-philosophy](../../core/unix-philosophy.md) - One tool, one job
- [adapter-pattern](../../core/adapter-pattern.md) - Frontend adapters
- [spec-database-identity.md](spec-database-identity.md) - UID (blocked by this)
