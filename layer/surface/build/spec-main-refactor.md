# Spec: main.rs Refactor

**Status:** Planned
**Phase:** 0 (Prerequisite to Phase 1)
**Created:** 2025-12-10

---

## Problem Statement

`src/main.rs` has grown to 1000 lines, with approximately 500 lines of business logic embedded in match arms. This violates two core patterns:

1. **dependable-rust.md**: "Keep public interface small and stable"
   - main.rs should be a thin CLI definition, not an implementation file

2. **unix-philosophy.md**: "One tool, one job, done well"
   - main.rs currently does parsing AND execution

### Quantitative Issues

| Metric | Current | Target |
|--------|---------|--------|
| Total lines | 1000 | < 600 |
| Lines in `main()` | 454 | < 100 |
| Match arms > 5 lines | 12 | 0 |
| String-typed enums | 4 | 0 |
| Inline `use` statements | 3 | 0 |

### Worst Offenders

1. **`Commands::Adapter` handler** (lines 824-993): 170 lines of inline business logic
2. **`Commands::Repo` handler** (lines 746-793): 48 lines of enum translation
3. **`Commands::Scrape { None }` handler** (lines 601-618): 15 lines of orchestration
4. **String enums**: `dimension`, `llm`, `dev` should be typed

---

## Design

### Target Architecture

```
src/main.rs           → CLI struct definitions + thin dispatch (< 600 lines)
src/commands/mod.rs   → Module declarations
src/commands/adapter.rs → Full adapter command implementation (NEW)
src/commands/scrape/mod.rs → Add execute_all() function
src/commands/repo/mod.rs → Accept clap types directly
```

### Pattern: Thin Dispatcher

Every match arm should be a single line:

```rust
// BEFORE (current)
Commands::Adapter { command } => {
    use patina::adapters::launch as frontend;
    use patina::project;

    match command {
        None | Some(AdapterCommands::List) => {
            // 30 lines of implementation
        }
        // ... 140 more lines
    }
}

// AFTER (target)
Commands::Adapter { command } => commands::adapter::execute(command)?,
```

---

## Tasks

### 0a: Extract Adapter Command Handler

**Goal:** Move 170 lines from main.rs to dedicated module.

**Create `src/commands/adapter.rs`:**

```rust
//! Adapter command - manage AI frontend configurations

use anyhow::Result;
use crate::AdapterCommands;  // Re-export from main or move enum here

pub fn execute(command: Option<AdapterCommands>) -> Result<()> {
    use patina::adapters::launch as frontend;
    use patina::project;

    match command {
        None | Some(AdapterCommands::List) => list()?,
        Some(AdapterCommands::Default { name, project }) => set_default(&name, project)?,
        Some(AdapterCommands::Check { name }) => check(name.as_deref())?,
        Some(AdapterCommands::Add { name }) => add(&name)?,
        Some(AdapterCommands::Remove { name, no_backup }) => remove(&name, no_backup)?,
    }
    Ok(())
}

fn list() -> Result<()> { /* moved from main.rs */ }
fn set_default(name: &str, is_project: bool) -> Result<()> { /* ... */ }
fn check(name: Option<&str>) -> Result<()> { /* ... */ }
fn add(name: &str) -> Result<()> { /* ... */ }
fn remove(name: &str, no_backup: bool) -> Result<()> { /* ... */ }
```

**Update `src/commands/mod.rs`:**
```rust
pub mod adapter;
```

**Lines saved:** ~165

### 0b: Extract Scrape Orchestration

**Goal:** Move inline orchestration to function.

**Add to `src/commands/scrape/mod.rs`:**

```rust
/// Run all scrapers in sequence
pub fn execute_all() -> Result<()> {
    println!("🔄 Running all scrapers...\n");

    println!("📊 [1/3] Scraping code...");
    execute_code(false, false)?;

    println!("\n📊 [2/3] Scraping git...");
    let git_stats = git::run(false)?;
    println!("  • {} commits", git_stats.items_processed);

    println!("\n📚 [3/3] Scraping sessions...");
    let session_stats = sessions::run(false)?;
    println!("  • {} sessions", session_stats.items_processed);

    println!("\n✅ All scrapers complete!");
    Ok(())
}
```

**Update main.rs:**
```rust
Commands::Scrape { command } => match command {
    None => commands::scrape::execute_all()?,
    Some(ScrapeCommands::Code { args }) => commands::scrape::execute_code(args.init, args.force)?,
    Some(ScrapeCommands::Git { full }) => commands::scrape::execute_git(full)?,
    Some(ScrapeCommands::Sessions { full }) => commands::scrape::execute_sessions(full)?,
}
```

**Lines saved:** ~15

### 0c: Unify Repo Command Types

**Problem:** Two parallel enum types exist:
- `RepoCommands` (main.rs, clap-derived)
- `RepoCommand` (commands/repo/mod.rs, internal)

48 lines translate between them.

**Solution:** Have `commands::repo` accept `Option<RepoCommands>` directly.

**Update `src/commands/repo/mod.rs`:**

```rust
use crate::RepoCommands;  // Import from main

pub fn execute(command: Option<RepoCommands>, url: Option<String>, contrib: bool, with_issues: bool) -> Result<()> {
    match (command, url) {
        (Some(RepoCommands::Add { url, contrib, with_issues }), _) => add(&url, contrib, with_issues),
        (Some(RepoCommands::List), _) => list(),
        // ... direct handling, no translation
        (None, Some(url)) => add(&url, contrib, with_issues),
        (None, None) => list(),
    }
}
```

**Update main.rs:**
```rust
Commands::Repo { command, url, contrib, with_issues } => {
    commands::repo::execute(command, url, contrib, with_issues)?
}
```

**Lines saved:** ~45

### 0d: Type String Enums

**Problem:** Several CLI args use `String` where enums should be used.

**Create `src/cli_types.rs` or add to main.rs:**

```rust
#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum Dimension {
    Semantic,
    Temporal,
    Dependency,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum Llm {
    Claude,
    Gemini,
    Codex,
    Local,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum DevEnv {
    Docker,
    Dagger,
    Native,
}
```

**Update CLI definitions:**

```rust
// BEFORE
#[arg(long)]
dimension: Option<String>,

// AFTER
#[arg(long, value_enum)]
dimension: Option<Dimension>,
```

**Benefits:**
- Compile-time validation of allowed values
- `--help` shows valid options automatically
- No runtime string matching needed

### 0e: Configurable ML Thresholds

**Problem:** Different commands have different `min_score` defaults:
- `scry`: 0.0 (broad search)
- `query semantic`: 0.35 (moderate filtering)
- `belief validate`: 0.50 (strict evidence)

These are hardcoded and undocumented.

**Solution:** Add `[search]` section to `ProjectConfig`.

**Update `src/project/internal.rs`:**

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchSection {
    /// Default threshold for scry command (broad search)
    #[serde(default = "default_scry_threshold")]
    pub scry_threshold: f32,

    /// Default threshold for semantic queries
    #[serde(default = "default_semantic_threshold")]
    pub semantic_threshold: f32,

    /// Default threshold for belief validation (strict)
    #[serde(default = "default_belief_threshold")]
    pub belief_threshold: f32,
}

fn default_scry_threshold() -> f32 { 0.0 }
fn default_semantic_threshold() -> f32 { 0.35 }
fn default_belief_threshold() -> f32 { 0.50 }
```

**Document the reasoning:**

```toml
# .patina/config.toml

[search]
# Scry: 0.0 - Cast wide net, let user filter
scry_threshold = 0.0

# Semantic: 0.35 - Balance relevance vs. recall
semantic_threshold = 0.35

# Belief: 0.50 - Only strong evidence for validation
belief_threshold = 0.50
```

---

## Validation Checklist

| Criteria | Status |
|----------|--------|
| main.rs < 600 lines | [ ] |
| `main()` function < 100 lines | [ ] |
| No match arm > 5 lines | [ ] |
| `Commands::Adapter` delegated to `commands::adapter::execute()` | [ ] |
| `Commands::Scrape { None }` calls `commands::scrape::execute_all()` | [ ] |
| `RepoCommands` enum not duplicated | [ ] |
| `dimension` arg is typed enum | [ ] |
| `llm` arg is typed enum | [ ] |
| `dev` arg is typed enum | [ ] |
| ML thresholds in `[search]` config section | [ ] |
| Threshold defaults documented in config | [ ] |

---

## Risk Assessment

| Risk | Mitigation |
|------|------------|
| Breaking CLI interface | Keep all arg names identical, only move implementation |
| Type enum breaks scripts | Add `serde` aliases for string values if needed |
| Config migration | Use `#[serde(default)]` for backward compatibility |

---

## Future Considerations

### Batch Mode (Not in Phase 0)

For ML workflows, consider adding:

```bash
patina scry --batch queries.txt --output results.jsonl
patina eval --format junit  # For CI integration
```

### Error Handling Consistency (Not in Phase 0)

Currently mixed patterns:
- Most commands: `?` propagation
- Doctor: `std::process::exit()`

Consider standardizing on `std::process::Termination` trait.

---

## References

- [dependable-rust.md](../../core/dependable-rust.md) - Black-box module pattern
- [unix-philosophy.md](../../core/unix-philosophy.md) - Single responsibility
- [build.md](../../core/build.md) - Phase 0 task list
