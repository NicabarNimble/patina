---
id: spec-skills-focused-adapter
status: design
created: 2026-01-19
tags: [spec, skills, adapter, architecture]
references: [unix-philosophy, dependable-rust, adapter-pattern]
supersedes: [spec-adapter-non-destructive, spec-skills-universal]
---

# Spec: Skills-Focused Adapter

**Core insight:** All three LLM CLIs use identical SKILL.md format. Skills are the universal connector to Patina.

**Strategy:** Skills deliver Patina's value. Supporting infrastructure (commands, scripts, bootstrap) is minimal scaffolding.

**Official spec:** https://agentskills.io/specification

---

## Core Values Alignment

This spec is anchored in Patina's core principles:

| Core Value | Application | "Do X" Test |
|------------|-------------|-------------|
| **Unix Philosophy** | Each component has one job. Skills guide, commands trigger, scripts execute. | ✅ Clear |
| **Dependable Rust** | Adapter trait is small and stable. Implementation details hidden. | ✅ Clear |
| **Adapter Pattern** | Trait-based adapters, runtime selection, no leaked types. | ✅ Clear |

### Component Responsibilities (One Job Each)

| Component | Do X | Doesn't Do |
|-----------|------|------------|
| **Skills** | Guide LLM to use Patina tools | Execute code, store state |
| **Commands** | Provide explicit `/slash` triggers | Auto-activate, make decisions |
| **Scripts** | Execute operations (Bash) | Guide LLM, store state |
| **MCP Tools** | Query knowledge (Rust) | Store state, manage files |

### Decomposition Principle

```
❌ Bad: One function doing everything
deploy_adapter() → skills + scripts + commands + context

✅ Good: Composable tools
deploy_skills()   → Copy SKILL.md files
deploy_scripts()  → Copy and chmod scripts
deploy_commands() → Copy adapter-specific commands
ensure_context()  → Create context directory

refresh() = deploy_skills() + deploy_scripts() + deploy_commands()
```

---

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                      PATINA CORE (Rust)                      │
│  MCP tools (scry/context/assay), layer/, knowledge.db       │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│                   SKILLS (Universal)                         │
│  SKILL.md files that surface Patina value to LLM context    │
│  Same format works in Claude Code, Gemini CLI, OpenCode     │
└─────────────────────────────────────────────────────────────┘
                            │
              ┌─────────────┼─────────────┐
              ▼             ▼             ▼
┌─────────────────┐ ┌─────────────────┐ ┌─────────────────┐
│  .claude/       │ │  .gemini/       │ │  .opencode/     │
│  ├── skills/    │ │  ├── skills/    │ │  ├── skills/    │
│  ├── commands/  │ │  ├── commands/  │ │  ├── commands/  │
│  ├── bin/       │ │  ├── bin/       │ │  ├── bin/       │
│  └── context/   │ │  └── context/   │ │  └── context/   │
└─────────────────┘ └─────────────────┘ └─────────────────┘
```

**Skills = Primary.** Commands and scripts = supporting infrastructure.

---

## Adapter Trait

Following the adapter-pattern core value, we define a minimal trait for what each adapter provides.

### Design Decision: Enum Implements Trait

**Why enum, not separate structs?**
- `Adapter` enum already exists in `launch.rs`
- Enum is small stable interface (dependable-rust)
- `match` on enum is more explicit than trait dispatch
- Less refactoring than restructuring to structs

```rust
/// Single adapter enum - implements both existing LLMAdapter and new SkillsAdapter
pub enum Adapter {
    Claude,
    Gemini,
    OpenCode,
}

#[derive(Clone, Copy)]
pub enum CommandFormat {
    Markdown,  // Claude, OpenCode
    Toml,      // Gemini
}

impl Adapter {
    pub fn from_name(name: &str) -> Result<Self> {
        match name.to_lowercase().as_str() {
            "claude" => Ok(Adapter::Claude),
            "gemini" => Ok(Adapter::Gemini),
            "opencode" => Ok(Adapter::OpenCode),
            _ => Err(anyhow!("Unknown adapter: {}", name)),
        }
    }
}

/// Trait for skills-focused adapter operations
pub trait SkillsAdapter {
    fn name(&self) -> &'static str;
    fn config_dir(&self) -> &'static str;
    fn bootstrap_file(&self) -> &'static str;
    fn command_format(&self) -> CommandFormat;

    fn skills_dir(&self, project: &Path) -> PathBuf {
        project.join(self.config_dir()).join("skills")
    }

    fn commands_dir(&self, project: &Path) -> PathBuf {
        project.join(self.config_dir()).join("commands").join("patina")
    }

    fn scripts_dir(&self, project: &Path) -> PathBuf {
        project.join(self.config_dir()).join("bin").join("patina")
    }
}

impl SkillsAdapter for Adapter {
    fn name(&self) -> &'static str {
        match self {
            Adapter::Claude => "claude",
            Adapter::Gemini => "gemini",
            Adapter::OpenCode => "opencode",
        }
    }

    fn config_dir(&self) -> &'static str {
        match self {
            Adapter::Claude => ".claude",
            Adapter::Gemini => ".gemini",
            Adapter::OpenCode => ".opencode",
        }
    }

    fn bootstrap_file(&self) -> &'static str {
        match self {
            Adapter::Claude => "CLAUDE.md",
            Adapter::Gemini => "GEMINI.md",
            Adapter::OpenCode => "AGENTS.md",
        }
    }

    fn command_format(&self) -> CommandFormat {
        match self {
            Adapter::Claude | Adapter::OpenCode => CommandFormat::Markdown,
            Adapter::Gemini => CommandFormat::Toml,
        }
    }
}
```

**Note:** This consolidates the existing `Adapter` enum in `launch.rs` with the new `SkillsAdapter` trait. No separate adapter structs needed.

### Integration with Existing Code

**File: `src/adapters/mod.rs`**

Current state:
```rust
pub trait LLMAdapter { ... }  // Lines 15-74
```

Action: **Keep `LLMAdapter` trait.** Add `SkillsAdapter` trait alongside it. The `Adapter` enum implements both:

```rust
// Keep existing
pub trait LLMAdapter { ... }

// Add new
pub trait SkillsAdapter { ... }

// Move from launch.rs, implement both
pub enum Adapter { Claude, Gemini, OpenCode }

impl LLMAdapter for Adapter { ... }      // Existing behavior
impl SkillsAdapter for Adapter { ... }   // New behavior
```

**File: `src/adapters/launch.rs`**

Current state:
```rust
pub enum Adapter { ... }           // Lines 14-80 - MOVE to mod.rs
pub fn generate_bootstrap() { ... } // Lines 167-178 - REPLACE with update_bootstrap()
fn bootstrap_content() { ... }      // Lines 335-360 - DELETE
```

**File: `src/adapters/templates.rs`**

Current state:
```rust
pub fn install_claude_templates() { ... }  // Lines 131-207
```

Action: **Replace** with new `deploy_*` functions. Don't keep old function.

**File: `src/commands/adapter.rs`**

Current state:
```rust
fn refresh() { ... }  // Lines 332-404 - complex backup/restore
const TEMPLATE_COMMANDS: &[&str] = &[...];  // Lines 407-416 - DELETE
```

Action: **Simplify** refresh to use new deploy functions. Remove hardcoded lists.

---

## Language Constraint

**Rust-first, TypeScript acceptable, no Python.**

| Component | Language | Rationale |
|-----------|----------|-----------|
| MCP tools | Rust | Core value, already implemented |
| Session scripts | Bash | Portable, already working |
| Skill instructions | Markdown | LLM reads, no runtime |
| Complex scripts | TypeScript | If needed, Bun available |

---

## Ownership Model

**Principle:** Patina owns `patina-*` prefixed items. Everything else is theirs.

```
.claude/                     # or .gemini/, .opencode/
├── skills/
│   ├── patina-codebase/     ← OURS
│   ├── patina-session/      ← OURS
│   ├── patina-beliefs/      ← OURS
│   ├── patina-review/       ← OURS
│   └── user-skill/          ← THEIRS
├── commands/
│   └── patina/              ← OURS (subdirectory)
├── bin/
│   └── patina/              ← OURS (subdirectory)
├── context/                 ← SHARED (session state)
└── */                       ← THEIRS

CLAUDE.md                    ← THEIRS (we offer @import, never overwrite)
```

**Why `patina-` prefix for skills, `patina/` subdir for commands/bin?**
- Skills: CLIs discover by name, prefix ensures uniqueness
- Commands/bin: Subdirectory groups our files, avoids collision

### Namespace-Based Ownership Detection

**No hardcoded lists.** The namespace IS the ownership marker:

```rust
/// Detect if a path belongs to Patina (should be managed by refresh)
fn is_patina_owned(path: &Path) -> bool {
    // Skills: anything starting with "patina-"
    if let Some(name) = path.file_name() {
        if name.to_string_lossy().starts_with("patina-") {
            return true;
        }
    }

    // Commands/bin: anything inside patina/ subdir
    path.components().any(|c| c.as_os_str() == "patina")
}
```

| Path | Owned? | Why |
|------|--------|-----|
| `skills/patina-codebase/` | ✅ Yes | `patina-` prefix |
| `skills/my-custom-skill/` | ❌ No | No prefix |
| `commands/patina/session-start.md` | ✅ Yes | `patina/` subdir |
| `commands/my-command.md` | ❌ No | Not in subdir |
| `bin/patina/session-start.sh` | ✅ Yes | `patina/` subdir |
| `bin/my-script.sh` | ❌ No | Not in subdir |

---

## Skills (Primary Interface)

### SKILL.md Format (Official Spec)

```yaml
---
name: skill-name           # Required: 1-64 chars, lowercase, hyphens
description: |             # Required: 1-1024 chars, triggers activation
  What this skill does and when to use it.
  Include keywords the LLM will match against.
---

# Instructions

Markdown body loaded when skill activates.
Keep under 500 lines. Use references/ for detail.
```

### Patina Skills

| Skill | Purpose | Key Content |
|-------|---------|-------------|
| `patina-codebase` | MCP tool guidance | When to use scry/context/assay |
| `patina-session` | Session workflow | Commands, state files, best practices |
| `patina-beliefs` | Belief capture | When/how to create epistemic beliefs |
| `patina-review` | History review | Reviewing sessions, git, layer changes |

### Skill: patina-codebase

```yaml
---
name: patina-codebase
description: |
  Codebase knowledge and search. Use for any question about code:
  finding functions, understanding architecture, locating files,
  searching history. Activates for "where is", "how does X work",
  "find the code that", "what calls", "show me".
---

# Codebase Knowledge

This project uses Patina for indexed codebase knowledge.

## MCP Tools

| Tool | Use For | Example |
|------|---------|---------|
| **scry** | Search code, commits, symbols | "where is auth handled" |
| **context** | Get patterns before changes | "what's the error handling pattern" |
| **assay** | Structural queries | "what imports this module" |

## When to Use What

- **"Where is X?"** → `scry` with natural language query
- **"How does Y work?"** → `scry` then `Read` the results
- **"What calls Z?"** → `assay` with `callers` query type
- **"Project conventions?"** → `context` with topic

## Always Try scry First

Scry searches pre-indexed knowledge. Faster than manual file exploration.
If scry doesn't find it, fall back to Glob/Grep.
```

### Skill: patina-session

```yaml
---
name: patina-session
description: |
  Development session management with Git integration. Use when
  starting work, tracking progress, capturing insights, or ending
  a session. Activates for "start session", "begin work", "track
  progress", "end session", "what did we do".
---

# Session Workflow

Sessions track development work with Git tagging.

## Commands

| Command | Purpose |
|---------|---------|
| `/session-start [name]` | Begin session, create Git tag |
| `/session-update` | Capture progress, show Git activity |
| `/session-note [insight]` | Record important learning |
| `/session-end` | Archive session, classify work |

## Session State

- **Active session:** `.claude/context/active-session.md`
- **Last session:** `.claude/context/last-session.md`
- **Archives:** `layer/sessions/`
- **Git tags:** `session-[timestamp]-start`, `session-[timestamp]-end`

## Best Practices

- Start session when beginning focused work
- Update after significant progress or commits
- Capture insights immediately (they fade)
- End session to preserve learnings for next time
```

### Skill: patina-beliefs

```yaml
---
name: patina-beliefs
description: |
  Create epistemic beliefs from session learnings. Use when
  synthesizing project decisions, capturing patterns that worked,
  or when user says "create a belief", "add belief", "capture
  this as a belief". Beliefs have evidence and confidence.
---

# Epistemic Beliefs

Beliefs capture project decisions with evidence and confidence.

## When to Create Beliefs

- After discovering a pattern that worked well
- When making a decision that should persist
- When correcting a misconception
- After successful debugging reveals insight

## Belief Structure

Location: `layer/surface/epistemic/beliefs/[slug].md`

```yaml
---
id: belief-slug
claim: "One sentence core claim"
confidence: high|medium|low
evidence:
  - type: session|commit|outcome
    ref: "reference"
    note: "what it shows"
tags: [pattern, decision, correction]
---

# Belief Title

Expanded explanation...

## Evidence

Details of supporting evidence...

## Relationships

- supports: [other-belief-id]
- attacks: [other-belief-id]
```

## Creating a Belief

Run: `.claude/bin/patina/create-belief.sh "[claim]"`

Or manually create file following the structure above.
```

### Skill: patina-review

```yaml
---
name: patina-review
description: |
  Review recent work and history. Use for "what happened",
  "review last session", "summarize progress", "what did we
  learn", "show recent activity".
---

# Review Recent Work

## Quick Commands

```bash
# Recent sessions
ls -la layer/sessions/ | tail -10

# Recent commits
git log --oneline -20

# Recent beliefs
ls -la layer/surface/epistemic/beliefs/

# Session detail
cat layer/sessions/[session-id].md
```

## Review Dimensions

1. **Git Activity** - commits, branches, tags
2. **Session Progress** - goals achieved, decisions made
3. **Beliefs Captured** - patterns formalized
4. **Layer Changes** - knowledge evolution

## Typical Review Flow

1. Read last session file for context
2. Check Git log since last session
3. Identify patterns worth capturing as beliefs
4. Note open items for next session
```

---

## Component Definitions

Clear terminology following Unix philosophy (one job each):

### Skills (Primary - Guide LLM)

**Do X:** Teach the LLM when and how to use Patina tools.

- Activate automatically via description matching
- Contain instructions, not executable code
- Same SKILL.md format across all adapters

### Commands (Explicit Triggers)

**Do X:** Provide explicit `/slash` triggers for user invocation.

- User types `/session-start` → command executes
- Complement skills (explicit vs automatic activation)
- Format differs by adapter (Markdown vs TOML)

```
.claude/commands/patina/
├── session-start.md
├── session-update.md
├── session-note.md
├── session-end.md
└── patina-review.md
```

**Command format (Claude/OpenCode - Markdown):**

File: `resources/claude/commands/patina/session-start.md`
```markdown
Start a new Patina development session with Git branch creation:

1. First, run a final update to capture recent work:
   - Execute `/session-update` command
   - This captures activity since the last update

2. Then start the new session:
   `.claude/bin/patina/session-start.sh $ARGUMENTS`

   This will:
   - Create session file at `.claude/context/active-session.md`
   - Tag the starting point in Git
   - Display previous session context if available

3. After the script completes, read `.claude/context/active-session.md` to understand:
   - Session ID and metadata
   - Previous session context (what was learned last time)
   - Goals for this session

Note: Session scripts are in `.claude/bin/patina/` (namespaced).
```

**Command format (Gemini - TOML):**

File: `resources/gemini/commands/patina/session-start.toml`
```toml
description = "Start a new Patina development session"
prompt = """
Start a new Patina development session with Git branch creation:

1. Execute: `.gemini/bin/patina/session-start.sh $ARGUMENTS`
2. Read the created `.gemini/context/active-session.md`
3. If `.gemini/context/last-session.md` exists, summarize previous session

Note: Session scripts are in `.gemini/bin/patina/` (namespaced).
"""
```

**Key path change:** Scripts moved from `.claude/bin/session-start.sh` to `.claude/bin/patina/session-start.sh`

### Scripts (Executables)

**Do X:** Execute operations when invoked by commands or skills.

- Bash scripts that do the actual work
- Referenced by skills and commands, never invoked directly by LLM
- Identical across adapters (same scripts, different paths)

```
.claude/bin/patina/
├── session-start.sh     # Create session file, git tag
├── session-update.sh    # Capture progress
├── session-note.sh      # Add note to session
├── session-end.sh       # Archive session
└── create-belief.sh     # Create belief file
```

### Context (Shared State)

**Do X:** Store session state files.

- Not a component, just a directory for state
- Shared between skills/commands/scripts
- Created by scripts, read by LLM

```
.claude/context/
├── active-session.md    # Current session state
└── last-session.md      # Reference to previous session
```

---

## Bootstrap Handling

**Principle:** Bootstrap files (CLAUDE.md, GEMINI.md, AGENTS.md) are user-owned. We only manage a marked section.

### Marker Section Approach (No Prompts)

**No prompts, no interruptions.** We manage content between markers only:

```rust
pub fn update_bootstrap(adapter: &Adapter, project: &Path) -> Result<()> {
    let bootstrap_path = project.join(adapter.bootstrap_file());

    let patina_section = r#"<!-- PATINA:START -->
## Patina

MCP tools: `scry` (search), `context` (patterns), `assay` (structure)
Skills: `patina-codebase`, `patina-session`, `patina-beliefs`, `patina-review`
<!-- PATINA:END -->"#;

    if bootstrap_path.exists() {
        let content = fs::read_to_string(&bootstrap_path)?;

        if content.contains("<!-- PATINA:START -->") {
            // Replace existing section
            let re = Regex::new(r"<!-- PATINA:START -->[\s\S]*?<!-- PATINA:END -->")?;
            let new_content = re.replace(&content, patina_section);
            fs::write(&bootstrap_path, new_content.as_ref())?;
        } else {
            // Append section to end
            let new_content = format!("{}\n\n{}", content.trim_end(), patina_section);
            fs::write(&bootstrap_path, new_content)?;
        }
    } else {
        // Create new file with section
        let content = format!("# Project\n\n{}", patina_section);
        fs::write(&bootstrap_path, content)?;
    }

    Ok(())
}
```

### Behavior

| Bootstrap State | Action |
|-----------------|--------|
| Doesn't exist | Create with Patina section |
| Exists, no markers | Append section at end |
| Exists, has markers | Replace section content only |

**User content outside markers is never touched.**

### When Bootstrap Is Modified

**Clarification:** `update_bootstrap()` is called by BOTH `adapter add` AND `adapter refresh`.

| Command | Skills/Commands/Scripts | Bootstrap |
|---------|------------------------|-----------|
| `adapter add` | Deploy fresh | Create or append section |
| `adapter refresh` | Remove ours, redeploy | Replace section only |

The marker section approach makes this safe:
- User content outside `<!-- PATINA:START -->...<!-- PATINA:END -->` is never touched
- On refresh, we only replace content BETWEEN markers
- This is idempotent - running refresh 10 times produces same result

### On `adapter refresh`

Refresh updates:
- `skills/patina-*` (removed and redeployed)
- `commands/patina/` (removed and redeployed)
- `bin/patina/` (removed and redeployed)
- Bootstrap Patina section (replaced between markers - safe due to markers)

---

## Directory Structure

### Note: Compile-Time vs Runtime Paths

**Two separate concerns - don't conflate:**

| Concern | Mechanism | Changes Needed |
|---------|-----------|----------------|
| **Runtime paths** | `paths::adapters_dir()` → `~/.patina/adapters/` | None - works fine |
| **Compile-time embedding** | `include_str!("../../resources/...")` in `templates.rs` | Update paths once during reorganization |

The `include_str!()` macros require literal string paths (Rust compile-time requirement). When reorganizing `resources/`, we update these paths once. This is an **implementation task**, not a design decision - no architectural change needed.

### Source (in Patina repo)

```
resources/
├── skills/                    # UNIVERSAL - same for all adapters
│   ├── patina-codebase/
│   │   └── SKILL.md
│   ├── patina-session/
│   │   ├── SKILL.md
│   │   └── references/
│   ├── patina-beliefs/
│   │   ├── SKILL.md
│   │   ├── scripts/
│   │   └── references/
│   └── patina-review/
│       └── SKILL.md
├── scripts/                   # UNIVERSAL - Bash scripts
│   ├── session-start.sh
│   ├── session-update.sh
│   ├── session-note.sh
│   ├── session-end.sh
│   └── create-belief.sh
├── claude/                    # ADAPTER-SPECIFIC
│   └── commands/patina/*.md
├── gemini/
│   └── commands/patina/*.toml
└── opencode/
    └── commands/patina/*.md
```

### Deployed (in user's project)

```
.claude/
├── skills/
│   ├── patina-codebase/SKILL.md
│   ├── patina-session/SKILL.md
│   ├── patina-beliefs/SKILL.md
│   └── patina-review/SKILL.md
├── commands/patina/
│   ├── session-start.md
│   └── ...
├── bin/patina/
│   ├── session-start.sh
│   └── ...
└── context/
    ├── active-session.md
    └── last-session.md
```

---

## Deployment Logic

Following Unix philosophy: separate tools that compose.

### Deployment Tools (One Job Each)

```rust
/// Deploy universal skills to adapter's skills directory
/// Do X: Write SKILL.md files from embedded content, preserve user skills
pub fn deploy_skills(adapter: &impl SkillsAdapter, project: &Path) -> Result<()> {
    let skills_dir = adapter.skills_dir(project);
    fs::create_dir_all(&skills_dir)?;

    // Remove old patina- skills only, preserve user skills
    if skills_dir.exists() {
        for entry in fs::read_dir(&skills_dir)? {
            let entry = entry?;
            if entry.file_name().to_string_lossy().starts_with("patina-") {
                fs::remove_dir_all(entry.path())?;
            }
        }
    }

    // Write fresh skills from embedded content
    const PATINA_SKILLS: &[&str] = &[
        "patina-codebase",
        "patina-session",
        "patina-beliefs",
        "patina-review",
    ];

    for skill in PATINA_SKILLS {
        let skill_dir = skills_dir.join(skill);
        fs::create_dir_all(&skill_dir)?;
        fs::write(skill_dir.join("SKILL.md"), get_skill_content(skill))?;
    }

    Ok(())
}

/// Deploy universal scripts to adapter's bin directory
/// Do X: Write Bash scripts from embedded content, make executable
pub fn deploy_scripts(adapter: &impl SkillsAdapter, project: &Path) -> Result<()> {
    let bin_dir = adapter.scripts_dir(project);
    fs::create_dir_all(&bin_dir)?;

    const PATINA_SCRIPTS: &[&str] = &[
        "session-start.sh",
        "session-update.sh",
        "session-note.sh",
        "session-end.sh",
        "create-belief.sh",
    ];

    for script in PATINA_SCRIPTS {
        let content = get_script_content(script);
        let path = bin_dir.join(script);
        fs::write(&path, content)?;
        make_executable(&path)?;
    }

    Ok(())
}

/// Deploy adapter-specific commands
/// Do X: Write commands from embedded content (Markdown or TOML)
pub fn deploy_commands(adapter: &impl SkillsAdapter, project: &Path) -> Result<()> {
    let commands_dir = adapter.commands_dir(project);

    // Remove old patina commands, preserve user commands
    if commands_dir.exists() {
        fs::remove_dir_all(&commands_dir)?;
    }
    fs::create_dir_all(&commands_dir)?;

    // Write adapter-specific commands from embedded content
    // Each adapter has its own command format (Markdown or TOML)
    let commands = get_commands_for_adapter(adapter.name());
    for (filename, content) in commands {
        fs::write(commands_dir.join(filename), content)?;
    }

    Ok(())
}

/// Get embedded command content for adapter
/// Returns vec of (filename, content) pairs
fn get_commands_for_adapter(adapter_name: &str) -> Vec<(&'static str, &'static str)> {
    match adapter_name {
        "claude" | "opencode" => vec![
            ("session-start.md", include_str!("../../resources/claude/commands/patina/session-start.md")),
            ("session-update.md", include_str!("../../resources/claude/commands/patina/session-update.md")),
            ("session-note.md", include_str!("../../resources/claude/commands/patina/session-note.md")),
            ("session-end.md", include_str!("../../resources/claude/commands/patina/session-end.md")),
            ("patina-review.md", include_str!("../../resources/claude/commands/patina/patina-review.md")),
        ],
        "gemini" => vec![
            ("session-start.toml", include_str!("../../resources/gemini/commands/patina/session-start.toml")),
            ("session-update.toml", include_str!("../../resources/gemini/commands/patina/session-update.toml")),
            ("session-note.toml", include_str!("../../resources/gemini/commands/patina/session-note.toml")),
            ("session-end.toml", include_str!("../../resources/gemini/commands/patina/session-end.toml")),
            ("patina-review.toml", include_str!("../../resources/gemini/commands/patina/patina-review.toml")),
        ],
        _ => vec![],
    }
}

/// Ensure context directory exists
/// Do X: Create directory for session state
pub fn ensure_context(adapter: &impl SkillsAdapter, project: &Path) -> Result<()> {
    let context_dir = project.join(adapter.config_dir()).join("context");
    fs::create_dir_all(&context_dir)?;
    Ok(())
}
```

### Composed Operations

```rust
/// Refresh adapter: redeploy all Patina-owned content
/// Composition of: deploy_skills + deploy_scripts + deploy_commands
pub fn refresh(adapter: &impl SkillsAdapter, project: &Path) -> Result<()> {
    println!("Refreshing Patina adapter files...");

    deploy_skills(adapter, project)?;
    println!("  ✓ skills/patina-* refreshed");

    deploy_scripts(adapter, project)?;
    println!("  ✓ bin/patina/ refreshed");

    deploy_commands(adapter, project)?;
    println!("  ✓ commands/patina/ refreshed");

    // Note: bootstrap updated separately via update_bootstrap()
    // Called by handle_refresh() after refresh()

    Ok(())
}

/// Full adapter setup: refresh + ensure context
pub fn setup(adapter: &impl SkillsAdapter, project: &Path) -> Result<()> {
    refresh(adapter, project)?;
    ensure_context(adapter, project)?;
    Ok(())
}
```

### Command Entry Point

```rust
// In src/commands/adapter.rs
pub fn handle_refresh(adapter_name: &str, no_commit: bool) -> Result<()> {
    let adapter = Adapter::from_name(adapter_name)?;
    let project = std::env::current_dir()?;

    refresh(&adapter, &project)?;
    update_bootstrap(&adapter, &project)?;

    if !no_commit {
        // Offer to commit changes
    }

    Ok(())
}

pub fn handle_add(adapter_name: &str) -> Result<()> {
    let adapter = Adapter::from_name(adapter_name)?;
    let project = std::env::current_dir()?;

    setup(&adapter, &project)?;
    update_bootstrap(&adapter, &project)?;

    Ok(())
}
```

---

## Helper Functions

**Location: `src/adapters/templates.rs`**

These utility functions are referenced by deploy functions:

```rust
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

/// Get embedded script content by name
/// Uses include_str!() - paths updated when resources/ is reorganized
fn get_script_content(name: &str) -> &'static str {
    match name {
        "session-start.sh" => include_str!("../../resources/scripts/session-start.sh"),
        "session-update.sh" => include_str!("../../resources/scripts/session-update.sh"),
        "session-note.sh" => include_str!("../../resources/scripts/session-note.sh"),
        "session-end.sh" => include_str!("../../resources/scripts/session-end.sh"),
        "create-belief.sh" => include_str!("../../resources/scripts/create-belief.sh"),
        _ => panic!("Unknown script: {}", name),
    }
}

/// Get embedded skill content by name
fn get_skill_content(skill_name: &str) -> &'static str {
    match skill_name {
        "patina-codebase" => include_str!("../../resources/skills/patina-codebase/SKILL.md"),
        "patina-session" => include_str!("../../resources/skills/patina-session/SKILL.md"),
        "patina-beliefs" => include_str!("../../resources/skills/patina-beliefs/SKILL.md"),
        "patina-review" => include_str!("../../resources/skills/patina-review/SKILL.md"),
        _ => panic!("Unknown skill: {}", skill_name),
    }
}

/// Make file executable (Unix only)
#[cfg(unix)]
fn make_executable(path: &Path) -> Result<()> {
    let mut perms = fs::metadata(path)?.permissions();
    perms.set_mode(0o755);
    fs::set_permissions(path, perms)?;
    Ok(())
}

#[cfg(not(unix))]
fn make_executable(_path: &Path) -> Result<()> {
    Ok(()) // No-op on Windows
}

/// Recursively copy directory
fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}
```

**Note:** Since we use `include_str!()` for embedding, we don't actually need `copy_dir_recursive()` for skills - we write content directly. But keeping it for potential future use with `references/` subdirectories.

---

## CLI Compatibility Matrix

| Feature | Claude Code | Gemini CLI | OpenCode |
|---------|-------------|------------|----------|
| Skills dir | `.claude/skills/` | `.gemini/skills/` | `.opencode/skills/` |
| SKILL.md format | ✓ identical | ✓ identical | ✓ identical |
| Commands dir | `.claude/commands/` | `.gemini/commands/` | `.opencode/commands/` |
| Command format | Markdown | TOML | Markdown |
| Claude fallback | N/A | No | ✓ scans `.claude/` |
| MCP support | ✓ | ✓ | ✓ |

**OpenCode bonus:** Scans `.claude/skills/` as fallback. If user has Claude adapter, OpenCode gets skills free.

---

## Edge Cases

### User Has Files in Our Namespace

**Scenario:** User has `commands/patina/my-custom.md` or `skills/patina-myskill/`

**Behavior:** These get deleted on refresh. The `patina` namespace is OURS.

**Mitigation:**
- Users should use different naming (e.g., `commands/my-patina-extensions/`)
- Document that `patina-*` prefix and `patina/` subdirs are reserved

### Bootstrap Has Malformed Markers

**Scenario:** User has `<!-- PATINA:START -->` but no `<!-- PATINA:END -->`

**Behavior:** Regex won't match. We append a NEW section at end.

**Result:** User ends up with two partial sections. Messy but not destructive.

**Future improvement:** Detect malformed markers and warn user.

### Bootstrap File is Read-Only

**Scenario:** `CLAUDE.md` has read-only permissions

**Behavior:** `fs::write()` fails with permission error.

**Mitigation:** Wrap in proper error handling:
```rust
fs::write(&bootstrap_path, content)
    .with_context(|| format!("Cannot write {}: check permissions", bootstrap_path.display()))?;
```

### Skills Directory Has Permission Issues

**Scenario:** `.claude/skills/` owned by different user

**Behavior:** Deploy fails.

**Mitigation:** Let it fail with clear error message. User needs to fix permissions.

### Concurrent Refresh

**Scenario:** Two processes run `adapter refresh` simultaneously

**Behavior:** Race condition - files may be partially written.

**Mitigation:** Not handled. Unlikely in practice. Could add lockfile if needed.

---

## Migration

### Decision: No Migration Code

**Pre-alpha, single user.** Don't build migration logic. Just switch.

**Manual migration for existing projects:**
```bash
# Remove old layout
rm -rf .claude/commands/session-*.md .claude/commands/patina-review.md
rm -rf .claude/bin/session-*.sh
rm -rf .claude/skills/epistemic-beliefs

# Refresh to get new layout
patina adapter refresh claude
```

**Why no migration code:**
- YAGNI - only one user (you) right now
- Can add migration for beta users if needed
- Simpler codebase, fewer edge cases
- Namespace detection handles "is this ours?" cleanly

### Layout Change Summary

```
# Old (flat)
.claude/
├── commands/session-start.md
├── bin/session-start.sh
└── skills/epistemic-beliefs/

# New (namespaced)
.claude/
├── commands/patina/session-start.md
├── bin/patina/session-start.sh
└── skills/patina-beliefs/
```

---

## Version Tracking

### Decision: Remove Per-Adapter Versioning

**Current state:**
- Claude has `adapter-manifest.json` with version 0.7.0
- Gemini/OpenCode have nothing

**Decision:** Remove adapter version tracking. It's tech debt.

**Why:**
- With namespace-based ownership, version tracking is unnecessary
- `refresh` always replaces `patina-*` content (no need to detect "is version old?")
- Simpler is better (unix-philosophy)
- If we need versioning later, track Patina CLI version in one place

**Removed:**
- `adapter-manifest.json` file
- `VERSION_CHANGES` array in manifest.rs
- Version comparison logic

**Kept:**
- Patina CLI version (in Cargo.toml, accessible via `--version`)

---

## Testing

Following adapter-pattern: use mock adapter for unit tests, real adapters for integration.

### Mock Adapter

```rust
/// Mock adapter for testing - records calls, returns predictable paths
pub struct MockAdapter {
    pub name: &'static str,
    pub config_dir: &'static str,
    pub calls: RefCell<Vec<String>>,
}

impl MockAdapter {
    pub fn new() -> Self {
        Self {
            name: "mock",
            config_dir: ".mock",
            calls: RefCell::new(Vec::new()),
        }
    }
}

impl SkillsAdapter for MockAdapter {
    fn name(&self) -> &'static str { self.name }
    fn config_dir(&self) -> &'static str { self.config_dir }
    fn bootstrap_file(&self) -> &'static str { "MOCK.md" }
    fn command_format(&self) -> CommandFormat { CommandFormat::Markdown }
}
```

### Unit Tests (with Mock)

```rust
#[test]
fn test_deploy_skills_uses_adapter_path() {
    let temp = TempDir::new().unwrap();
    let adapter = MockAdapter::new();

    deploy_skills(&adapter, temp.path()).unwrap();

    // Skills deployed to adapter's skills_dir
    let skills_dir = temp.path().join(".mock/skills");
    assert!(skills_dir.join("patina-codebase/SKILL.md").exists());
}

#[test]
fn test_deploy_preserves_user_skills() {
    let temp = TempDir::new().unwrap();
    let adapter = MockAdapter::new();
    let skills_dir = temp.path().join(".mock/skills");
    fs::create_dir_all(&skills_dir).unwrap();

    // Create user skill
    fs::create_dir_all(skills_dir.join("my-skill")).unwrap();
    fs::write(skills_dir.join("my-skill/SKILL.md"), "user content").unwrap();

    // Deploy
    deploy_skills(&adapter, temp.path()).unwrap();

    // User skill preserved
    assert!(skills_dir.join("my-skill/SKILL.md").exists());
    assert_eq!(
        fs::read_to_string(skills_dir.join("my-skill/SKILL.md")).unwrap(),
        "user content"
    );

    // Patina skills deployed
    assert!(skills_dir.join("patina-codebase/SKILL.md").exists());
}

#[test]
fn test_refresh_never_touches_bootstrap() {
    let temp = TempDir::new().unwrap();
    let adapter = MockAdapter::new();
    let bootstrap = temp.path().join("MOCK.md");
    fs::write(&bootstrap, "user content").unwrap();

    // Refresh (not setup - setup might create bootstrap)
    refresh(&adapter, temp.path()).unwrap();

    // Bootstrap unchanged
    assert_eq!(fs::read_to_string(&bootstrap).unwrap(), "user content");
}
```

### Skill Validation Tests

```rust
#[test]
fn test_all_skills_have_valid_skill_md() {
    for skill in glob("resources/skills/patina-*/SKILL.md").unwrap() {
        let skill = skill.unwrap();
        let content = fs::read_to_string(&skill).unwrap();
        assert!(content.starts_with("---"), "Missing frontmatter: {:?}", skill);
        assert!(content.contains("name:"), "Missing name: {:?}", skill);
        assert!(content.contains("description:"), "Missing description: {:?}", skill);
    }
}

#[test]
fn test_skill_names_match_directories() {
    for skill_dir in glob("resources/skills/patina-*/").unwrap() {
        let skill_dir = skill_dir.unwrap();
        let dir_name = skill_dir.file_name().unwrap().to_string_lossy();
        let content = fs::read_to_string(skill_dir.join("SKILL.md")).unwrap();

        // Extract name from frontmatter
        let name_line = content.lines()
            .find(|l| l.starts_with("name:"))
            .expect("Missing name field");
        let name = name_line.trim_start_matches("name:").trim();

        assert_eq!(name, dir_name, "Skill name must match directory name");
    }
}
```

### Integration Tests (Real Adapters)

```rust
#[test]
fn test_claude_adapter_paths() {
    let adapter = Adapter::Claude;  // Use enum variant, not struct
    let project = Path::new("/tmp/test");

    assert_eq!(adapter.config_dir(), ".claude");
    assert_eq!(adapter.skills_dir(project), project.join(".claude/skills"));
    assert_eq!(adapter.commands_dir(project), project.join(".claude/commands/patina"));
}

#[test]
fn test_gemini_toml_commands_valid() {
    for cmd in glob("resources/gemini/commands/patina/*.toml").unwrap() {
        let cmd = cmd.unwrap();
        let content = fs::read_to_string(&cmd).unwrap();
        // Should parse as valid TOML
        toml::from_str::<toml::Value>(&content)
            .expect(&format!("Invalid TOML: {:?}", cmd));
    }
}

#[test]
fn test_skill_discovered_by_cli_glob_pattern() {
    // Simulate how CLIs discover skills: skills/*/SKILL.md
    let temp = TempDir::new().unwrap();
    let adapter = Adapter::Claude;  // Use enum variant, not struct

    setup(&adapter, temp.path()).unwrap();

    let pattern = temp.path().join(".claude/skills/*/SKILL.md");
    let skills: Vec<_> = glob(pattern.to_str().unwrap()).unwrap().collect();

    assert!(skills.iter().any(|p| {
        p.as_ref().unwrap().to_string_lossy().contains("patina-codebase")
    }));
}
```

---

## File Locations Summary

Quick reference for where new code lives:

| Function/Type | File | Notes |
|---------------|------|-------|
| `Adapter` enum | `src/adapters/mod.rs` | Move from `launch.rs` |
| `SkillsAdapter` trait | `src/adapters/mod.rs` | New |
| `CommandFormat` enum | `src/adapters/mod.rs` | New |
| `is_patina_owned()` | `src/adapters/mod.rs` | New helper |
| `deploy_skills()` | `src/adapters/templates.rs` | Replace old install functions |
| `deploy_scripts()` | `src/adapters/templates.rs` | Replace old install functions |
| `deploy_commands()` | `src/adapters/templates.rs` | Replace old install functions |
| `ensure_context()` | `src/adapters/templates.rs` | New |
| `refresh()` | `src/adapters/templates.rs` | Composed function |
| `setup()` | `src/adapters/templates.rs` | Composed function |
| `update_bootstrap()` | `src/adapters/launch.rs` | Replace `generate_bootstrap()` |
| `get_script_content()` | `src/adapters/templates.rs` | New helper |
| `get_skill_content()` | `src/adapters/templates.rs` | New helper |
| `make_executable()` | `src/adapters/templates.rs` | New helper |
| `get_commands_for_adapter()` | `src/adapters/templates.rs` | New helper |
| `handle_refresh()` | `src/commands/adapter.rs` | Update existing |
| `handle_add()` | `src/commands/adapter.rs` | Update existing |

**Delete these:**
- `src/adapters/manifest.rs` (if exists) - version tracking removal
- `bootstrap_content()` in `launch.rs`
- `TEMPLATE_COMMANDS` constant in `adapter.rs`
- `TEMPLATE_SKILLS` constant in `adapter.rs`
- Old `install_*_templates()` functions in `templates.rs`

---

## Checklist

### Phase 1: Adapter Consolidation
- [ ] Move `Adapter` enum from `launch.rs` to `adapters/mod.rs`
- [ ] Add `SkillsAdapter` trait
- [ ] Implement `SkillsAdapter` for `Adapter` enum
- [ ] Add `CommandFormat` enum
- [ ] Add `is_patina_owned()` helper function
- [ ] Remove `adapter-manifest.json` handling (version tracking removal)

### Phase 2: Consolidate Skills
- [ ] Create `resources/skills/` directory
- [ ] Create `patina-codebase/SKILL.md`
- [ ] Create `patina-session/SKILL.md`
- [ ] Rename `epistemic-beliefs/` → `patina-beliefs/`
- [ ] Create `patina-review/SKILL.md`

### Phase 3: Reorganize Resources
- [ ] Move scripts to `resources/scripts/` (universal)
- [ ] Create `resources/claude/commands/patina/` (update paths in commands)
- [ ] Create `resources/gemini/commands/patina/` (TOML format)
- [ ] Create `resources/opencode/commands/patina/`
- [ ] Update `templates.rs` embeds for new paths

### Phase 4: Deployment Tools
- [ ] Implement `deploy_skills(adapter, project)`
- [ ] Implement `deploy_scripts(adapter, project)`
- [ ] Implement `deploy_commands(adapter, project)`
- [ ] Implement `ensure_context(adapter, project)`
- [ ] Implement composed `refresh()` and `setup()`

### Phase 5: Bootstrap Handling
- [ ] Implement `update_bootstrap()` with marker section approach
- [ ] Remove old `generate_bootstrap()` / `bootstrap_content()`
- [ ] Test: create new, append to existing, replace existing section

### Phase 6: Cleanup
- [ ] Remove `TEMPLATE_COMMANDS` / `TEMPLATE_SKILLS` constants
- [ ] Remove `manifest.rs` and version tracking code
- [ ] Remove old flat-structure install code
- [ ] Update tests for new structure

### Phase 7: Manual Migration (You)
- [ ] Remove old `.claude/` content from patina project
- [ ] Run `patina adapter refresh claude`
- [ ] Verify skills/commands/scripts deployed correctly
- [ ] Verify CLAUDE.md has marker section

---

## References

### Core Values (layer/core/)
- [unix-philosophy.md](../../core/unix-philosophy.md) - One tool, one job, composition
- [dependable-rust.md](../../core/dependable-rust.md) - Small stable interface, hide implementation
- [adapter-pattern.md](../../core/adapter-pattern.md) - Trait-based, runtime selection

### Official Skills Spec
- [Agent Skills Specification](https://agentskills.io/specification)
- [Anthropic Skills Repo](https://github.com/anthropics/skills)

### CLI Implementations (indexed as ref repos)
- `skills` - Official examples, skill-creator patterns
- `gemini-cli` - skillLoader.ts, skillManager.ts source
- `opencode` - skill.ts with Claude fallback logic
- `claude-code` - Docs/issues (closed source)

### Superseded Specs
- `spec-adapter-non-destructive.md` - Broader scope, rules-based approach
- `spec-skills-universal.md` - Skills-only, merged into this
