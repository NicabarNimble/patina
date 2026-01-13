---
id: spec-adapter-selection
status: ready
created: 2026-01-13
updated: 2026-01-13
tags: [spec, adapter, init, launch, ux]
references: [adapter-pattern, unix-philosophy, dependable-rust, rationale-eskil-steenberg]
prerequisite-complete: true
---

# Spec: Adapter Selection at Project Init

**Problem:** When initializing a new project via `patina` (launch in empty dir), the adapter is chosen before the user has any say. The resolution happens at launch/internal.rs:36-38 BEFORE the "Are you lost?" prompt.

**Scope:** This change ONLY affects the "Are you lost?" flow in launch. The standalone `patina init` command remains unchanged (skeleton only, explicit `patina adapter add`).

---

## Core Values Alignment

### Eskil Steenberg

> "Design complete API from day one, implement incrementally."

The adapter selection API should be complete now:
- **Single function**: `select_adapter(available: &[AdapterInfo], preference: Option<&str>) -> Result<String>`
- **Stable signature**: Won't change even if we add 10 more adapters
- **Black box**: Caller doesn't know if it prompted or auto-selected
- **Lives in `adapters/launch.rs`**: Part of the adapter module's public API

> "No adaptive complexity. No magic."

The selection logic is deterministic:
- 0 adapters → error (clear message)
- 1 adapter → use it (no prompt)
- 2+ adapters → prompt user to choose

No "smart" defaults based on heuristics, project type detection, or AI guessing.

### Jon Gjengset

> "Fail fast with clear errors."

If no adapters detected, fail immediately with actionable message:
```
No AI adapters detected on this system.
Install one of: claude, gemini, opencode
```

> "Clean break, no deprecated aliases."

The old behavior (silently defaulting to claude) is removed, not deprecated. No `--legacy-default` flag.

> "Separate functions for separate concerns."

`select_adapter()` is a separate, testable function in `adapters/launch.rs`. It handles ONLY the selection logic - no initialization, no file creation.

However, we do NOT over-split `prompt_are_you_lost()`. It remains one function that internally calls `select_adapter()` when needed. Splitting into 3 tiny functions adds overhead with no reuse benefit.

### Unix Philosophy

> "Adding flags instead of commands is bad."

This is NOT a new flag. The adapter selection happens at the natural decision point (init time) with a simple numbered prompt. No `--choose-adapter` or `--interactive` flags.

> "One tool, one job."

`select_adapter()` does exactly one thing: given available adapters, return one. It doesn't:
- Initialize anything
- Create files
- Modify config
- Validate the adapter is suitable for the project

---

## Two Distinct Flows

The `--adapter` flag creates two distinct paths through launch:

### Flow A: Explicit Adapter (`--adapter=X`)

User knows what they want. Respect their choice.

```
1. Parse --adapter=gemini
2. Validate gemini is installed (adapters::get)
   - If not installed → error immediately, don't prompt
3. Show "Are you lost?" prompt
4. If yes → initialize with gemini (no selection prompt)
5. Continue launch with gemini
```

**Key:** Validation happens BEFORE the prompt. If gemini isn't installed, fail fast - don't waste user's time asking about init.

### Flow B: No Flag (Implicit)

User hasn't chosen. Help them decide.

```
1. No --adapter flag, don't resolve yet
2. Show "Are you lost?" prompt
3. If yes:
   a. Detect all available adapters (adapters::list, filter detected)
   b. Call select_adapter(available, global_default_preference)
   c. Initialize with selected adapter
4. Continue launch with selected adapter
```

**Key:** Validation is deferred. We detect what's available AFTER user commits to init.

---

## Specification

### Function: `select_adapter` (in `adapters/launch.rs`)

```rust
/// Select an adapter from available options.
///
/// Returns the chosen adapter name.
///
/// Behavior:
/// - 0 available: Error with installation instructions
/// - 1 available: Returns it (no prompt)
/// - 2+ available: Prompts user to choose
///
/// If `preference` matches an available adapter, it becomes the default selection.
/// This is used to honor the global config default without forcing it.
pub fn select_adapter(
    available: &[AdapterInfo],
    preference: Option<&str>,
) -> Result<String>
```

**Why this signature:**
- `&[AdapterInfo]` - caller owns detection, we just select
- `Option<&str>` - preference from global config (NOT from --adapter flag, that's Flow A)
- Returns `String` - just the name, not the full AdapterInfo

**Why in `adapters/launch.rs`:**
- Already has `list()` → returns all adapters with detection status
- Already has `default_name()` → returns global default
- Already has `is_available()` → quick check
- `select_adapter()` is the logical next step in this module's API

### Function: `prompt_are_you_lost` (updated signature)

```rust
/// "Are you lost?" prompt - show git context and offer to initialize.
///
/// Returns:
/// - Ok(None) - user declined to init
/// - Ok(Some(adapter_name)) - user accepted, project initialized with this adapter
///
/// If `explicit_adapter` is Some, uses that adapter without prompting for selection.
/// If None, detects available adapters and prompts user to choose.
fn prompt_are_you_lost(
    project_path: &Path,
    explicit_adapter: Option<&str>,
) -> Result<Option<String>>
```

**Why this signature:**
- Returns `Option<String>` so caller knows which adapter was selected
- `explicit_adapter` distinguishes Flow A (Some) from Flow B (None)
- Caller can update its `adapter_name` variable based on return value

### User Experience

**Case 1: Single adapter available (Flow B)**
```
Initialize as patina project? [y/N]: y

🎨 Initializing Patina...
[proceeds with the only available adapter, no mention of selection]
```

**Case 2: Multiple adapters available (Flow B)**
```
Initialize as patina project? [y/N]: y

📱 Available adapters:
  [1] Claude Code (default)
  [2] Gemini CLI

Select adapter [1]: 2

🎨 Initializing Patina...
✓ Added 'gemini' to allowed adapters
```

**Case 3: Explicit adapter (Flow A)**
```
$ patina --adapter=gemini

🚀 Launching Gemini CLI in /path/to/project
  ✓ Mothership running
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
 Are you lost?
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

This is not a patina project.

Initialize as patina project? [y/N]: y

🎨 Initializing Patina...
[proceeds with gemini, no selection prompt]
```

**Case 4: No adapters available**
```
Initialize as patina project? [y/N]: y

Error: No AI adapters detected on this system.
Install one of: claude, gemini, opencode
```

**Case 5: Explicit adapter not installed (Flow A, fail fast)**
```
$ patina --adapter=gemini

Error: Adapter 'gemini' (Gemini CLI) is not installed.
Install it and try again, or use a different adapter.
```

Note: Error happens BEFORE "Are you lost?" prompt.

---

## Changes to Launch Flow

### Current Code (launch/internal.rs)

```rust
// Line 36-38: Resolve adapter BEFORE prompt
let adapter_name = options
    .adapter
    .unwrap_or_else(|| adapters::default_name().unwrap_or_else(|_| "claude".to_string()));

// Line 41-50: Validate adapter installed
let adapter_info = adapters::get(&adapter_name)?;
if !adapter_info.detected {
    bail!("Adapter '{}' is not installed...");
}

// Line 66-79: Prompt and init
if !patina_dir.exists() {
    if options.auto_init {
        let initialized = prompt_are_you_lost(&project_path, &adapter_name)?;
        // continues with original adapter_name
    }
}

// Line 108-118: Validate adapter in allowed list
if !project_config.adapters.allowed.contains(&adapter_name) {
    bail!("Adapter '{}' is not in allowed adapters...");
}
```

### Proposed Code

```rust
// Step 3: Handle explicit vs implicit adapter
let explicit_adapter: Option<String> = options.adapter.clone();

// If explicit, validate it's installed NOW (fail fast)
if let Some(ref name) = explicit_adapter {
    let adapter_info = adapters::get(name)?;
    if !adapter_info.detected {
        bail!(
            "Adapter '{}' ({}) is not installed.\n\
             Install it and try again, or use a different adapter.",
            name,
            adapter_info.display
        );
    }
}

// ... mothership check ...

// Step 6: Check if this is a patina project
if !patina_dir.exists() {
    if options.auto_init {
        // Pass explicit_adapter - if Some, skip selection prompt
        match prompt_are_you_lost(&project_path, explicit_adapter.as_deref())? {
            Some(selected) => {
                // Update adapter_name to what user selected (or explicit)
                adapter_name = selected;
            }
            None => {
                // User declined
                return Ok(());
            }
        }
    } else {
        bail!("Not a patina project...");
    }
}

// Step 7: Validate adapter in allowed list
// This now works because adapter_name was updated to the selected adapter
let project_config = project::load_with_migration(&project_path)?;
if !project_config.adapters.allowed.contains(&adapter_name) {
    bail!("Adapter '{}' is not in allowed adapters...");
}
```

**Key changes:**
1. `explicit_adapter` is `Option<String>` not resolved to default
2. Explicit adapter validated early (fail fast)
3. `prompt_are_you_lost` returns `Option<String>`
4. `adapter_name` updated based on selection
5. Step 7 validation now works correctly

---

## What This Spec Does NOT Cover

1. **Changing adapters later** - Use `patina adapter add/remove/default`
2. **Multiple adapters at init** - One at a time, add more later
3. **Adapter recommendations** - No "for Python projects, we suggest..."
4. **Auto-detection of "best" adapter** - User knows what they want
5. **Standalone `patina init`** - Remains skeleton-only, explicit adapter add
6. **Existing projects** - No selection prompt, use normal adapter management

---

## Prerequisite: Wire OpenCode into Launch System (Complete)

OpenCode is now fully wired into the launch system:

| Component | Status |
|-----------|--------|
| `src/adapters/opencode/` | ✅ Full adapter implementation |
| `src/adapters/mod.rs` | ✅ `get_adapter("opencode")` works |
| `src/adapters/templates.rs` | ✅ Templates embedded |
| `src/workspace/internal.rs` | ✅ Detection during workspace setup |
| `src/commands/adapter.rs` | ✅ Bootstrap filename handled |
| `src/adapters/launch.rs` | ✅ Added to enum and ADAPTERS const |

### Current State in `adapters/launch.rs`

```rust
pub const ADAPTERS: &[&str] = &["claude", "gemini", "opencode"];

pub enum Adapter {
    Claude,
    Gemini,
    OpenCode,
}

impl Adapter {
    pub fn name(&self) -> &'static str {
        match self {
            Adapter::Claude => "claude",
            Adapter::Gemini => "gemini",
            Adapter::OpenCode => "opencode",
        }
    }

    pub fn display(&self) -> &'static str {
        match self {
            Adapter::Claude => "Claude Code",
            Adapter::Gemini => "Gemini CLI",
            Adapter::OpenCode => "OpenCode",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        match name.to_lowercase().as_str() {
            "claude" => Some(Adapter::Claude),
            "gemini" => Some(Adapter::Gemini),
            "opencode" => Some(Adapter::OpenCode),
            _ => None,
        }
    }

    pub fn bootstrap_file(&self) -> &'static str {
        match self {
            Adapter::Claude => "CLAUDE.md",
            Adapter::Gemini => "GEMINI.md",
            Adapter::OpenCode => "OPENCODE.md",
        }
    }

    pub fn detect_commands(&self) -> &'static [&'static str] {
        match self {
            Adapter::Claude => &["claude --version"],
            Adapter::Gemini => &["gemini --version"],
            Adapter::OpenCode => &["opencode --version"],
        }
    }
}
```

### Prerequisite Checklist

- [x] Add `OpenCode` variant to `Adapter` enum
- [x] Add `"opencode"` to `ADAPTERS` const
- [x] Update `name()` match arm
- [x] Update `display()` match arm
- [x] Update `from_name()` match arm
- [x] Update `bootstrap_file()` match arm
- [x] Update `detect_commands()` match arm
- [x] Update `get_mcp_config()` match arm (discovered during implementation)
- [x] Update tests to cover OpenCode
- [x] Verify `patina adapter list` shows opencode
- [x] Commit: `feat(adapters): wire OpenCode into launch system` (42807464)

---

## Implementation Checklist

- [ ] Add `select_adapter()` function to `adapters/launch.rs`
- [ ] Update `prompt_are_you_lost()` signature to return `Option<String>`
- [ ] Update `prompt_are_you_lost()` to accept `explicit_adapter: Option<&str>`
- [ ] Split launch flow into explicit vs implicit adapter paths
- [ ] Update `adapter_name` after selection in launch flow
- [ ] Test: 0 adapters case (error with install instructions)
- [ ] Test: 1 adapter case (no prompt, auto-select)
- [ ] Test: 2+ adapters case (numbered prompt)
- [ ] Test: `--adapter=X` skips selection prompt
- [ ] Test: `--adapter=X` where X not installed (fail fast before prompt)
- [ ] Test: Global default becomes pre-selected option

---

## Rejected Alternatives

### A. Move selection to `patina init`

**Rejected because:** The "Are you lost?" flow in launch is the primary entry point for new users. They run `patina`, not `patina init`. We should optimize for that path.

### B. Interactive wizard with multiple questions

**Rejected because:** Unix philosophy - one question, one answer. "Which adapter?" is the only decision needed.

### C. Environment variable to skip prompt

**Rejected because:** Jon Gjengset - no escape hatches that become the default. If you want non-interactive, use `--adapter=claude` flag.

### D. Remember last choice globally

**Rejected because:** Already exists - global config has `adapter.default`. The prompt respects it as the default selection.

### E. Split `prompt_are_you_lost()` into 3 functions

**Rejected because:** Over-engineering. The function is only called from one place. The three steps happen in immediate sequence. Eskil: "It's faster to write 5 lines of code today than to write 1 line today and edit it later."

We DO extract `select_adapter()` as a separate function because it's independently testable and belongs in the adapter module's public API.

### F. Put `select_adapter()` in `launch/internal.rs`

**Rejected because:** Violates adapter module's responsibility. The adapter module (`adapters/launch.rs`) already has `list()`, `default_name()`, `is_available()`. Selection is the logical next step in that API. Launch is a coordinator, not an implementer.

---

## Open Questions (Resolved)

### Q1: Should `patina init` (standalone) also prompt for adapter?

**Answer: No.** Keep current behavior. `patina init` creates skeleton only, prints "Add an adapter: patina adapter add ...". This maintains separation of concerns and makes the two-step process explicit.

### Q2: If user has `--adapter=foo` but foo isn't installed?

**Answer:** Error immediately, before "Are you lost?" prompt. Fail fast - don't waste user's time asking about init if we can't proceed. This is Flow A behavior.

### Q3: Should `--adapter=X` skip the selection prompt entirely?

**Answer: Yes.** If user explicitly specified an adapter, they don't want to be asked. They made their choice on the command line. This distinguishes Flow A from Flow B.
