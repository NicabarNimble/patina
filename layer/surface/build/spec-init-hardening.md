# Spec: Init, Adapter, and Launcher

**Status:** Design (Code Review Complete)
**Created:** 2026-01-12
**Last Review:** 2026-01-12 (Session 20260112-154400)
**Blocks:** spec-database-identity.md (UID needs stable init)
**Core References:** [dependable-rust](../../core/dependable-rust.md), [unix-philosophy](../../core/unix-philosophy.md), [adapter-pattern](../../core/adapter-pattern.md)

---

## Quick Start for New Sessions

**Read this first.** This spec has been code-reviewed. The "Code Review Findings" section contains exact file paths and line numbers.

### What This Spec Does

Refactors `patina init` to follow Unix philosophy:
- **Before:** `patina init --llm claude` does 5 things (skeleton + adapter + MCP + scrape + oxidize)
- **After:** `patina init` creates skeleton only, `patina adapter add claude` adds LLM support

### Implementation Order

| Phase | Description | Effort | Dependencies |
|-------|-------------|--------|--------------|
| **1** | Simplify Init | ~2 hours | None |
| **2** | Adapter refresh/doctor | ~1.5 hours | Phase 1 |
| **3** | Launcher polish | ~30 min | Phase 1 |
| **4** | Observability | Optional | Phases 1-3 |

### To Start Implementation

1. Read "Code Review Findings" section for exact line numbers
2. Start with Phase 1: `src/main.rs` → `src/commands/init/`
3. Test: `cargo build --release && cargo install --path . && patina init .`
4. Expected: Creates `.patina/` and `layer/` only, NO `.claude/`

### Key Decisions Already Made

- Keep `adapter default` (not `switch`) - already implemented
- Keep `[frontends]` config key for now - rename deferred to separate PR
- Phase 4 (Observability) is optional - core works without it
- MCP config is best-effort in `adapter add`, auto-fixed silently in launcher

---

## Goals

### Why This Spec Exists

`patina init` currently violates Unix philosophy by doing too much:
- Creates project skeleton (.patina/, layer/)
- Creates adapter files (.claude/)
- Configures MCP
- Runs scrape
- Runs oxidize

This causes real problems:
- **Partial state failures**: Init fails halfway, user has broken project
- **Re-init destroys config**: User customizations wiped on update
- **Can't add adapters later**: Forced to re-init to add gemini to existing claude project
- **Hard to test**: One giant function with many failure modes

### What We Want

Three commands, each doing ONE thing:

| Command | Job | Creates |
|---------|-----|---------|
| `patina init` | Create project skeleton | .patina/, layer/ |
| `patina adapter add` | Add LLM integration | .claude/, CLAUDE.md, MCP |
| `patina` | Launch and work | (nothing - just launches) |

User flow becomes:
```bash
patina init                    # Create skeleton
patina adapter add claude      # Add Claude support
patina                         # Start working
```

### Core Principle

> "Do X test: state what it does in one sentence"

- `init`: "Create minimal Patina project structure"
- `adapter add`: "Add LLM adapter to project"
- `patina`: "Launch configured LLM"

If a command can't be described in one sentence, it's doing too much.

---

## What Exists Today

This is a **refactoring**, not greenfield. Most infrastructure exists.

### `patina init` (Exists - Needs Simplification)

Currently does everything. We strip it down to skeleton only.

**Keep:**
- Hierarchy conflict checks
- Git repository init
- .patina/ and layer/ creation
- .gitignore management
- Git commit

**Remove:**
- `adapter.init_project()` call (move to `adapter add`)
- scrape/oxidize calls (separate commands)
- `--llm` argument (adapters added separately)

**Add:**
- UID creation (`.patina/uid`)
- Config merge strategy (preserve user settings on re-init)
- `--no-commit` flag

### `patina adapter` (Exists - Needs Extension)

Current subcommands in `src/commands/adapter.rs`:

| Subcommand | Status | Notes |
|------------|--------|-------|
| `adapter list` | ✅ Works | Shows available/configured adapters |
| `adapter add <name>` | ✅ Works | Creates .claude/, CLAUDE.md |
| `adapter remove <name>` | ✅ Works | Removes adapter |
| `adapter default <name>` | ✅ Works | Sets default adapter |
| `adapter check <name>` | ✅ Works | Checks CLI installation |
| `adapter mcp <name>` | ✅ Works | Configures MCP server |

**Add:**
| Subcommand | Purpose |
|------------|---------|
| `adapter refresh <name>` | Update templates, preserve sessions |
| `adapter doctor` | Health check all adapters |
| `--no-commit` flag | Skip auto-commit on add/remove/refresh |

### `patina` Launcher (Exists - Needs Polish)

Current flow in `src/commands/launch/`:
1. ✅ Detect if in patina project
2. ✅ "Are you lost?" prompt for non-projects
3. ✅ Check adapter in allowed list
4. ✅ Check LLM CLI installed
5. ⚠️ MCP check (warns but doesn't fix)
6. ✅ Exec into LLM CLI

**Change:**
- Silent MCP auto-configuration (fix instead of warn)
- Remove "🚀 Launching..." output (silent on success)

### Supporting Infrastructure (All Exists)

| Component | Location | Status |
|-----------|----------|--------|
| LLM detection | `src/adapters/launch.rs` | ✅ Complete |
| Template copying | `src/adapters/templates.rs` | ✅ Complete |
| Config load/save | `src/project/internal.rs` | ✅ Complete |
| Session backup | `src/commands/init/internal/backup.rs` | ⚠️ Refactor for reuse |

---

## Code Review Findings (2026-01-12)

**This section is the source of truth for implementation.** It documents exact file locations and line numbers for what exists vs what needs changing.

### Config Schema: `[frontends]` NOT `[adapters]`

**Current state:** Config uses `FrontendsSection` which serializes to `[frontends]`:
- Struct definition: `src/project/internal.rs:98-122`
- All code references `config.frontends.allowed` and `config.frontends.default`

**Spec says:** Rename to `[adapters]` for consistency with `LLMAdapter` trait.

**Migration required:** Yes - existing projects have `[frontends]` in config.toml.

### `patina init` - What To Remove

| What | Location | Line(s) | Action |
|------|----------|---------|--------|
| `adapter.init_project()` | `src/commands/init/internal/mod.rs` | 171-173 | DELETE |
| `adapter.post_init()` | `src/commands/init/internal/mod.rs` | 199-200 | DELETE |
| `scrape::execute_all()` | `src/commands/init/internal/mod.rs` | 246-253 | DELETE |
| `oxidize::oxidize()` | `src/commands/init/internal/mod.rs` | 260-267 | DELETE |
| `ensure_model_available()` | `src/commands/init/internal/mod.rs` | 257 | DELETE |
| `--llm` CLI argument | `src/main.rs` | 96-98 | DELETE |
| `llm: String` parameter | `src/commands/init/internal/mod.rs` | 26 | DELETE |
| `llm` param in config creation | `src/commands/init/internal/config.rs` | 19, 49 | DELETE |

### `patina init` - What To Add

| What | Location | Notes |
|------|----------|-------|
| `--no-commit` flag | `src/main.rs` (Init struct) | Add `#[arg(long)] no_commit: bool` |
| `no_commit` parameter | `src/commands/init/internal/mod.rs:24` | Add to `execute_init()` signature |
| Conditional commit | `src/commands/init/internal/mod.rs:216-241` | Wrap in `if !no_commit { ... }` |
| UID creation | `src/project/internal.rs` | Add `create_uid_if_missing()` function |
| UID file | `.patina/uid` | 8 hex chars, created once, never modified |
| Config merge | `src/commands/init/internal/config.rs` | Preserve immutable/preserved fields on re-init |

### `patina init` - What To Keep (No Changes)

| What | Location | Line(s) |
|------|----------|---------|
| `check_hierarchy_conflicts()` | `src/commands/init/internal/mod.rs` | 35, 637-756 |
| `ensure_git_initialized()` | `src/commands/init/internal/mod.rs` | 46, 470-496 |
| `ensure_gitignore()` | `src/commands/init/internal/mod.rs` | 65, 499-807 |
| `Layer::new().init()` | `src/commands/init/internal/mod.rs` | 137-143 |
| `copy_core_patterns_safe()` | `src/commands/init/internal/mod.rs` | 188-191 |
| Git commit logic | `src/commands/init/internal/mod.rs` | 216-241 |

### `patina adapter` - Existing Subcommands

| Subcommand | Function | Line(s) | Status |
|------------|----------|---------|--------|
| `list` | `list()` | 96-128 | ✅ Works |
| `add` | `add()` | 180-210 | ✅ Works (needs `--no-commit`) |
| `remove` | `remove()` | 213-257 | ✅ Works |
| `default` | `set_default()` | 131-155 | ✅ Works |
| `check` | `check()` | 158-177 | ✅ Works |
| `mcp` | `configure_mcp()` | 290-347 | ✅ Works |

**Note:** Command is `adapter default` (already implemented).

### `patina adapter` - What To Add

| Subcommand | Purpose | Estimated Lines |
|------------|---------|-----------------|
| `refresh` | Backup, update templates, restore sessions | ~50 lines |
| `doctor` | Health check all adapters (CLI version, MCP status) | ~40 lines |

**Enum changes needed in `src/commands/adapter.rs:36-81`:**
```rust
Refresh {
    name: String,
    #[arg(long)]
    no_commit: bool,
},
Doctor,
```

### `patina` Launcher - What To Change

| What | Location | Line(s) | Action |
|------|----------|---------|--------|
| "🚀 Launching..." output | `src/commands/launch/internal.rs` | 52-56 | DELETE |
| "Launching {}..." output | `src/commands/launch/internal.rs` | 412 | DELETE |
| MCP auto-fix | `src/commands/launch/internal.rs` | After line 118 | ADD silent MCP config |

**MCP auto-fix implementation:**
```rust
// After checking adapter in allowed list (line 118), add:
let adapter = patina::adapters::get_adapter(&frontend_name);
if !adapter.is_mcp_configured(&project_path)? {
    // Silent fix - don't print anything
    let _ = adapter.configure_mcp(&project_path);
}
```

**New method needed in adapter trait:**
- `is_mcp_configured(&self, project_path: &Path) -> Result<bool>` in `src/adapters/mod.rs`

### Events Database - New Files

| File | Purpose |
|------|---------|
| `src/db/events.rs` | Schema: `events(id, timestamp, event_type, adapter, data)` |
| `src/db/mod.rs` | Add `pub mod events;` |
| `src/commands/stats.rs` | `patina stats` command |

**Note:** Phase 4 (Observability) can be deferred. Core functionality works without it.

### Summary: Minimal Implementation Path

**To get a working simplified init (Phase 1):**

1. `src/main.rs`: Remove `llm: Llm` from Init, add `no_commit: bool`
2. `src/commands/init/mod.rs`: Update signature
3. `src/commands/init/internal/mod.rs`:
   - Remove lines 171-173 (adapter.init_project)
   - Remove lines 199-200 (adapter.post_init)
   - Remove lines 246-267 (scrape/oxidize)
   - Remove line 257 (ensure_model_available)
   - Wrap commit logic in `if !no_commit`
4. `src/commands/init/internal/config.rs`: Remove `llm` parameter
5. Update user message at end: "Add an adapter: patina adapter add <claude|gemini|opencode>"

**Test:** `patina init .` should create only `.patina/` and `layer/`, no `.claude/`.

---

## What We Need to Build

### New Code

| Item | Location | Purpose |
|------|----------|---------|
| UID creation | `src/project/internal.rs` | Stable project identity |
| Config merge | `src/commands/init/internal/config.rs` | Preserve user settings on re-init |
| `adapter refresh` | `src/commands/adapter.rs` | Update templates safely |
| `adapter doctor` | `src/commands/adapter.rs` | Health check |
| Events database | `src/db/events.rs` | Observability (optional) |

### Refactoring

| Item | Change |
|------|--------|
| `execute_init()` | Remove adapter/scrape/oxidize calls |
| `backup.rs` | Extract for reuse by `adapter refresh` |
| Launcher | Add silent MCP fix |

### Config Schema Change

Rename `[frontends]` → `[adapters]` for consistency:

```toml
# Before
[frontends]
allowed = ["claude"]
default = "claude"

# After
[adapters]
allowed = ["claude"]
default = "claude"
```

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

[adapters]
allowed = []         # Empty! No adapter yet
default = ""

[environment]
os = "macos"
arch = "aarch64"
```

### What It Does NOT Do

- Create adapter directories (.claude/, .gemini/)
- Configure MCP
- Run scrape
- Run oxidize
- Require adapter selection

### Re-init Behavior

```bash
patina init .        # In existing project
```

- Refreshes environment detection
- Preserves ALL config (adapters, upstream, ci)
- Updates version manifest
- Never touches adapter directories

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
    println!("  Add an adapter: patina adapter add <claude|gemini|opencode>");

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

## Command 2: `patina adapter`

### Purpose

Manage LLM adapters. Add, remove, set default, diagnose.

### Subcommands

```bash
patina adapter list              # Show available and configured
patina adapter add <name>        # Add and configure adapter
patina adapter refresh <name>    # Update adapter with backup
patina adapter default <name>    # Change default adapter
patina adapter remove <name>     # Remove adapter
patina adapter doctor            # Health check all adapters
```

### Supported Adapters

| Name | Directory | Context File | MCP |
|------|-----------|--------------|-----|
| claude | `.claude/` | `CLAUDE.md` | Yes |
| gemini | `.gemini/` | `GEMINI.md` | TBD |
| opencode | `.opencode/` | `AGENTS.md` | Yes |

### `adapter add` Flow

```bash
patina adapter add claude
```

```
1. Verify in patina project
   └─ No .patina/? → "Run 'patina init' first"

2. Verify adapter known
   └─ Unknown? → "Available: claude, gemini, opencode"

3. Check if already added
   └─ Already in allowed? → "claude already configured. Use 'adapter refresh' to update."

4. Check for existing directory
   └─ .claude/ exists but not in allowed? → "Found existing .claude/. Use --force to overwrite."

5. Create adapter directory
   └─ .claude/commands/, .claude/context/, .claude/bin/

6. Create context file
   └─ CLAUDE.md with project info

7. Copy session scripts
   └─ .claude/bin/session-*.sh

8. Update config.toml
   └─ adapters.allowed += "claude"
   └─ adapters.default = "claude" (if first)

9. Configure MCP (best effort)
   └─ claude mcp add patina ...
   └─ If fails: log warning, continue (launcher will fix on first run)

10. Log event
    └─ adapter_added to events.db

11. Commit (unless --no-commit)
    └─ "chore: add claude adapter"

12. Done
    └─ "✓ Added claude. Run 'patina' to start."
```

### `adapter default` Flow

```bash
patina adapter default gemini
```

```
1. Verify gemini in adapters.allowed
   └─ Not added? → "Run 'patina adapter add gemini' first"

2. Update config.toml
   └─ adapters.default = "gemini"

3. Verify adapter files exist
   └─ Missing? → Create them

4. Log event
   └─ adapter_switched

5. Done
   └─ "✓ Switched to gemini"
```

### `adapter refresh` Flow

```bash
patina adapter refresh claude
```

```
1. Verify claude in adapters.allowed
   └─ Not configured? → "Run 'patina adapter add claude' first"

2. Extract session files
   └─ .claude/context/*.md → temp
   └─ Print: "✓ Preserved active session" (if session exists)

3. Backup existing adapter
   └─ .claude/ → .backup/claude_<timestamp>

4. Recreate adapter directory
   └─ .claude/commands/, .claude/context/, .claude/bin/

5. Update templates and scripts
   └─ Copy latest from resources

6. Restore session files
   └─ temp → .claude/context/*.md

7. Log event
   └─ adapter_refreshed

8. Commit (unless --no-commit)
   └─ "chore: refresh claude adapter"

9. Done
   └─ "✓ Refreshed claude (backup: .backup/claude_<timestamp>)"
```

### `adapter doctor` Output

```bash
patina adapter doctor
```

```
Adapter Health Check
────────────────────

claude
  CLI:       ✓ v1.0.45
  Adapter:   ✓ v0.3.0
  MCP:       ✓ configured
  Status:    Ready

gemini
  CLI:       ✓ v2.1.0
  Adapter:   ✓ v0.1.0
  MCP:       ✗ not configured
  Status:    Run 'patina adapter add gemini' to configure

opencode
  CLI:       ✗ not found
  Install:   go install github.com/opencode-ai/opencode@latest
```

### Implementation

```rust
pub fn execute_adapter(cmd: AdapterCommand) -> Result<()> {
    match cmd {
        AdapterCommand::List => list_adapters(),
        AdapterCommand::Add { name, no_commit, force } => add_adapter(&name, no_commit, force),
        AdapterCommand::Refresh { name, no_commit } => refresh_adapter(&name, no_commit),
        AdapterCommand::Default { name } => set_default_adapter(&name),
        AdapterCommand::Remove { name, no_commit } => remove_adapter(&name, no_commit),
        AdapterCommand::Doctor => doctor_adapters(),
    }
}

fn add_adapter(name: &str, no_commit: bool, force: bool) -> Result<()> {
    let project_path = require_patina_project()?;
    let adapter = get_adapter(name)?;
    let mut config = load_config(&project_path)?;

    // Check if already configured
    if config.adapters.allowed.contains(&name.to_string()) {
        println!("{} already configured. Use 'adapter refresh' to update.", name);
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
    config.adapters.allowed.push(name.to_string());
    if config.adapters.default.is_empty() {
        config.adapters.default = name.to_string();
    }
    save_config(&project_path, &config)?;

    // Configure MCP (best effort - launcher will fix if this fails)
    if let Err(e) = adapter.configure_mcp(&project_path) {
        log_event("mcp_config_deferred", json!({ "adapter": name, "error": e.to_string() }))?;
    }

    // Log event
    log_event("adapter_added", json!({ "adapter": name }))?;

    // Commit
    if !no_commit {
        git_commit(&[&adapter_dir, "CLAUDE.md"], &format!("chore: add {} adapter", name))?;
    }

    println!("✓ Added {}. Run 'patina' to start.", name);
    Ok(())
}

fn refresh_adapter(name: &str, no_commit: bool) -> Result<()> {
    let project_path = require_patina_project()?;
    let adapter = get_adapter(name)?;
    let config = load_config(&project_path)?;

    // Verify configured
    if !config.adapters.allowed.contains(&name.to_string()) {
        println!("{} not configured. Run 'patina adapter add {}' first.", name, name);
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
    log_event("adapter_refreshed", json!({ "adapter": name }))?;

    // 6. Commit
    if !no_commit {
        git_commit(&[&adapter_dir], &format!("chore: refresh {} adapter", name))?;
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
patina                      # Launch default adapter
patina --adapter gemini     # Launch specific adapter (override)
patina -a claude            # Short form
```

### The Magic

User types `patina`. Launcher:

```
1. In a patina project?
   ├─ No  → "Not a Patina project. Run: patina init"
   └─ Yes → continue

2. Adapter configured?
   ├─ No  → "No adapter configured."
   │        "Run: patina adapter add <claude|gemini|opencode>"
   └─ Yes → continue

3. LLM CLI installed on system?
   ├─ No  → Show install instructions for that CLI
   └─ Yes → continue

4. MCP configured?
   ├─ No  → Configure silently (no user action needed)
   └─ Yes → continue

5. Log event: launch_started

6. Exec into LLM CLI
   └─ Replace process with LLM CLI
```

### Key Principle: Invisible

- No output on success (just launches)
- Only speaks when something needs user action
- Fixes what it can silently (including MCP config that failed during `adapter add`)
- Guides user for what it can't fix (e.g., CLI not installed)

### Implementation

```rust
pub fn execute_launch(adapter_override: Option<String>) -> Result<()> {
    // 1. Require patina project
    let project_path = match find_patina_project() {
        Some(p) => p,
        None => {
            println!("Not a Patina project.");
            println!("Run: patina init");
            return Ok(());
        }
    };

    // 2. Load config, determine adapter
    let config = load_config(&project_path)?;
    let adapter_name = adapter_override
        .unwrap_or_else(|| config.adapters.default.clone());

    if adapter_name.is_empty() {
        println!("No adapter configured.");
        println!("Run: patina adapter add <claude|gemini|opencode>");
        return Ok(());
    }

    // 3. Verify adapter in allowed list
    if !config.adapters.allowed.contains(&adapter_name) {
        println!("Adapter '{}' not configured.", adapter_name);
        println!("Run: patina adapter add {}", adapter_name);
        return Ok(());
    }

    // 4. Check LLM CLI installed
    let cli_info = get_cli_info(&adapter_name)?;
    if !cli_info.detected {
        println!("'{}' not installed.", cli_info.display);
        println!("Install: {}", cli_info.install_cmd);
        return Ok(());
    }

    // 5. Ensure MCP configured (silent)
    let adapter = get_adapter(&adapter_name)?;
    if !adapter.is_mcp_configured(&project_path)? {
        adapter.configure_mcp(&project_path)?;
    }

    // 6. Log and launch
    log_event("launch_started", json!({
        "adapter": adapter_name,
        "version": cli_info.version,
    }))?;

    // 7. Exec (replaces this process)
    exec_cli(&adapter_name, &project_path)
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
| `adapters.allowed` | Preserved | Never overwrite |
| `adapters.default` | Preserved | Never overwrite |
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
        adapters: existing.as_ref()
            .map(|c| c.adapters.clone())
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
    adapter TEXT,
    data TEXT
);
```

Event types:
- `init_completed` - with `reinit` flag
- `adapter_added`, `adapter_refreshed`, `adapter_switched`, `adapter_removed`
- `mcp_config_deferred` - MCP config failed, launcher will fix
- `launch_started`, `launch_failed`
- `error` - with error type and message

### Key Metrics

| Metric | What It Measures |
|--------|------------------|
| `init_completed` → `adapter_added` time | Multi-step flow friction |
| `adapter_refreshed` count | Backup/restore usage |
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
Add an adapter: patina adapter add <claude|gemini|opencode>

$ patina adapter add claude
✓ Created .claude/
✓ Configured MCP
Run 'patina' to start.

$ patina
[Claude Code launches]
```

### Add Second Adapter

```bash
$ patina adapter add gemini
✓ Created .gemini/
✓ Configured MCP

$ patina adapter list
Configured adapters:
  claude (default)
  gemini

$ patina adapter default gemini
✓ Default: gemini

$ patina
[Gemini CLI launches]
```

### Quick Override

```bash
$ patina -a claude
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
| `.claude/` → `.backup/claude_<timestamp>` | Refresh | `patina adapter refresh` |
| Session files preserved | Refresh | `patina adapter refresh` |

**Note**: `patina init` does not touch adapter directories. Backup/restore of `.claude/` is handled by `adapter refresh`.

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
| `patina adapter add` | `.claude/`, `CLAUDE.md` | "chore: add claude adapter" |
| `patina adapter refresh` | Updated `.claude/` | "chore: refresh claude adapter" |
| `patina adapter remove` | Removes `.claude/` | "chore: remove claude adapter" |

**Design**: Each command is self-contained. User can batch operations with `--no-commit` then commit manually.

---

## Implementation Phases

### Phase 1: Simplify Init (Refactor)

**Goal**: Strip init down to skeleton creation only.

**Step-by-step instructions:**

1. **Update CLI arguments** (`src/main.rs`):
   ```rust
   // Change Init struct (around line 92):
   Init {
       name: String,
       // REMOVE: llm: Llm,
       #[arg(long, value_enum)]
       dev: Option<DevEnv>,
       #[arg(long)]
       force: bool,
       #[arg(long)]
       local: bool,
       #[arg(long)]  // ADD THIS
       no_commit: bool,
   },
   ```

2. **Update public API** (`src/commands/init/mod.rs`):
   - Change `execute()` signature to remove `llm`, add `no_commit`

3. **Strip execute_init()** (`src/commands/init/internal/mod.rs`):
   - Delete line 26: remove `llm: String` parameter
   - Delete lines 171-173: `adapter.init_project()` call
   - Delete lines 199-200: `adapter.post_init()` call
   - Delete line 257: `ensure_model_available()` call
   - Delete lines 260-267: `oxidize::oxidize()` call
   - Delete lines 246-253: `scrape::execute_all()` call
   - Wrap lines 216-241 (commit logic) in `if !no_commit { ... }`
   - Update final message: `"Add an adapter: patina adapter add <claude|gemini|opencode>"`

4. **Update config creation** (`src/commands/init/internal/config.rs`):
   - Remove `llm` parameter from `create_project_config()` (line 19)
   - Remove `frontends` initialization (lines 48-51) - set to empty defaults
   - **OR** implement config merge (preserve existing frontends on re-init)

5. **Add UID creation** (`src/project/internal.rs`):
   ```rust
   pub fn create_uid_if_missing(patina_dir: &Path) -> Result<String> {
       let uid_path = patina_dir.join("uid");
       if uid_path.exists() {
           return Ok(fs::read_to_string(&uid_path)?.trim().to_string());
       }
       let uid = format!("{:08x}", rand::random::<u32>());
       fs::write(&uid_path, &uid)?;
       Ok(uid)
   }
   ```

6. **Test**: `cargo build --release && cargo install --path . && patina init .`
   - Should create `.patina/` and `layer/` only
   - Should NOT create `.claude/` or run scrape/oxidize

### Phase 2: Adapter Command (Extend)

**Goal**: Add missing subcommands to existing adapter command.

**Step-by-step instructions:**

1. **Add enum variants** (`src/commands/adapter.rs`):
   ```rust
   // Add to AdapterCommands enum (after Mcp):
   Refresh {
       name: String,
       #[arg(long)]
       no_commit: bool,
   },
   Doctor,
   ```

2. **Add match arms** (`src/commands/adapter.rs:84-93`):
   ```rust
   Some(AdapterCommands::Refresh { name, no_commit }) => refresh(&name, no_commit),
   Some(AdapterCommands::Doctor) => doctor(),
   ```

3. **Implement refresh()** (~50 lines):
   - Extract session files from `.claude/context/`
   - Backup `.claude/` to `.backup/claude_<timestamp>`
   - Call `templates::copy_to_project()`
   - Restore session files
   - Commit if `!no_commit`

4. **Implement doctor()** (~40 lines):
   - For each adapter in `frontends.allowed`:
     - Check CLI installed (reuse `frontend::get()`)
     - Check MCP configured (need new `is_mcp_configured()`)
     - Print status table

5. **Add `--no-commit` to existing add/remove**:
   - Add `#[arg(long)] no_commit: bool` to Add and Remove variants
   - Wrap commit logic in conditionals

6. **Test**: `patina adapter refresh claude`, `patina adapter doctor`

### Phase 3: Launcher (Polish)

**Goal**: Make existing launcher silent and self-healing.

**Step-by-step instructions:**

1. **Remove verbose output** (`src/commands/launch/internal.rs`):
   - Delete lines 52-56: `println!("🚀 Launching...")`
   - Delete line 412: `println!("\nLaunching {}...\n")`

2. **Add is_mcp_configured() to LLMAdapter trait** (`src/adapters/mod.rs`):
   ```rust
   // Add to LLMAdapter trait:
   fn is_mcp_configured(&self, project_path: &Path) -> Result<bool>;

   // For ClaudeAdapter: check ~/.config/claude/config.json for patina server
   // Return true if configured, false if not
   ```

3. **Add silent MCP fix** (`src/commands/launch/internal.rs`):
   ```rust
   // After line 118 (frontend in allowed check), add:
   // Silent MCP auto-configuration
   let adapter = patina::adapters::get_adapter(&frontend_name);
   if !adapter.is_mcp_configured(&project_path)? {
       let _ = adapter.configure_mcp(&project_path);
       // Ignore errors - if it fails, user will notice when MCP doesn't work
   }
   ```

4. **Test**: Launch should be silent on success, only print errors.

### Phase 4: Observability (Optional/Deferred)

**Goal**: Add event logging for measuring friction.

**Can be implemented later.** Core functionality works without it.

1. Create `src/db/events.rs` with SQLite schema
2. Add `log_event()` helper function
3. Sprinkle `log_event()` calls in init, adapter, launch
4. Create `patina stats` command to query events

---

## Files to Modify

### Phase 1: Simplify Init

| File | Lines | Change |
|------|-------|--------|
| `src/main.rs` | 92-111 | Remove `llm: Llm`, add `no_commit: bool` to Init struct |
| `src/commands/init/mod.rs` | 71-78 | Update `execute()` signature |
| `src/commands/init/internal/mod.rs` | 24-30 | Update `execute_init()` signature |
| `src/commands/init/internal/mod.rs` | 171-173 | DELETE `adapter.init_project()` |
| `src/commands/init/internal/mod.rs` | 199-200 | DELETE `adapter.post_init()` |
| `src/commands/init/internal/mod.rs` | 246-267 | DELETE scrape/oxidize calls |
| `src/commands/init/internal/mod.rs` | 216-241 | Wrap commit in `if !no_commit` |
| `src/commands/init/internal/config.rs` | 17-73 | Remove `llm` param, set empty frontends |
| `src/project/internal.rs` | NEW | Add `create_uid_if_missing()` function |

### Phase 2: Adapter Command

| File | Lines | Change |
|------|-------|--------|
| `src/commands/adapter.rs` | 36-81 | Add `Refresh` and `Doctor` to enum |
| `src/commands/adapter.rs` | 84-93 | Add match arms for new subcommands |
| `src/commands/adapter.rs` | NEW | Implement `refresh()` function (~50 lines) |
| `src/commands/adapter.rs` | NEW | Implement `doctor()` function (~40 lines) |
| `src/commands/adapter.rs` | 57-60, 63-69 | Add `no_commit: bool` to Add/Remove |

### Phase 3: Launcher (Polish Existing)

| File | Lines | Change |
|------|-------|--------|
| `src/commands/launch/internal.rs` | 52-56 | DELETE "🚀 Launching..." output |
| `src/commands/launch/internal.rs` | 412 | DELETE "Launching {}..." output |
| `src/commands/launch/internal.rs` | ~119 | ADD silent MCP auto-fix |
| `src/adapters/mod.rs` | Trait | Add `is_mcp_configured()` to LLMAdapter trait |

### Phase 4: Observability (Optional)

| File | Change |
|------|--------|
| `src/db/events.rs` | **New file**: events.db schema and logging |
| `src/db/mod.rs` | Add `pub mod events;` |
| `src/commands/stats.rs` | **New file**: `patina stats` command |
| `src/commands/mod.rs` | Add `pub mod stats;` |
| `src/main.rs` | Add `Stats` command variant |

### Deferred: Config Schema Rename

| File | Change |
|------|--------|
| `src/project/internal.rs` | Rename `FrontendsSection` → `AdaptersSection` |
| `src/project/internal.rs` | Change serde rename to `adapters` |
| `src/commands/adapter.rs` | Update all `config.frontends` → `config.adapters` |
| `src/commands/launch/internal.rs` | Update all `config.frontends` → `config.adapters` |
| Migration | Add `[frontends]` → `[adapters]` migration in `load_with_migration()` |

**Note:** Config rename can be done in a separate PR to avoid scope creep.

### Already Complete (No Changes Needed)

| File | Status |
|------|--------|
| `src/commands/launch/mod.rs` | ✅ Launcher exists |
| `src/adapters/launch.rs` | ✅ LLM CLI detection exists |
| `src/adapters/mod.rs` | ✅ LLMAdapter trait exists |
| `src/adapters/templates.rs` | ✅ Template copying exists |
| `src/project/mod.rs` | ✅ Config load/save exists |
| `src/commands/init/internal/backup.rs` | ✅ Session backup exists |

---

## Success Criteria

| Phase | Test |
|-------|------|
| 1 | `patina init` creates only .patina/ and layer/ |
| 1 | Re-init preserves adapters config |
| 1 | `patina init` auto-commits (unless `--no-commit`) |
| 2 | `adapter add claude` creates .claude/ and configures MCP |
| 2 | `adapter add` on existing shows "use refresh" message |
| 2 | `adapter refresh` backs up and preserves sessions |
| 2 | `adapter default` changes default |
| 3 | `patina` (no args) launches default adapter |
| 3 | Missing adapter shows helpful message |
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
- [adapter-pattern](../../core/adapter-pattern.md) - LLM adapters
- [spec-database-identity.md](spec-database-identity.md) - UID (blocked by this)
