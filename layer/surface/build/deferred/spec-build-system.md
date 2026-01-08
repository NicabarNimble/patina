# Spec: Build Tracking System

**Status:** Backlog (Never Started)

**Goal:** Track build tasks across sessions with a TOML file, commit trailers for history, and documentation for LLM guidance.

---

> **Why Deferred:**
>
> This was planned as Phase 4 but never started. Current `build.md` + session workflow is working.
>
> **Reason:**
> - Manual markdown tracking is sufficient for current scale
> - Higher priority work: observability, mothership design, retrieval optimization
> - Over-engineering risk: adding structured task tracking before proving manual approach is insufficient
>
> **Resume trigger:** When session/spec workflow friction becomes a bottleneck, or when LLM task querying would provide clear value.

---

## Problem Statement

### Current State

1. **Manual markdown tracking**: `build.md` has phases and checkboxes, manually maintained
2. **No query interface**: LLM can't ask "what's pending?" programmatically
3. **No exploration tracking**: Rabbit holes aren't captured, learnings lost
4. **Disconnected from git**: Task state and commit history aren't linked
5. **LLM can't reliably update**: Markdown editing is fragile

### Key Insight

**State file + commit trailers + documentation = simple, git-native build tracking.**

- TOML for current state (queryable, structured)
- Commit trailers for history (git-native, searchable)
- CLAUDE.md for LLM guidance (teaches proper usage)

---

## Design Principles

### 1. Git is the Database

We don't build custom audit infrastructure. Git provides:
- History: `git log --grep="Task:"`
- Blame: `git blame .patina/build.toml`
- Branching: TOML is branch-local
- Undo: `git revert`

### 2. One File, One Truth

`.patina/build.toml` is the current state. Everything else is derived.

### 3. Commands, Not File Editing

LLMs use `patina build` commands. They don't edit TOML directly. This ensures:
- Consistent format
- Automatic commits with trailers
- Validation

### 4. Capture Learnings, Even From Failures

Explorations (rabbit holes) always end with `--learned`. Dead ends produce knowledge.

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                     .patina/build.toml                               │
│                     (Current State)                                  │
│                                                                      │
│   - What's the current phase?                                       │
│   - What tasks exist and their status?                              │
│   - What's deferred?                                                │
│   - What explorations are active?                                   │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
                                ↑
                                │ updated by
                                │
┌─────────────────────────────────────────────────────────────────────┐
│                     patina build commands                            │
│                                                                      │
│   patina build task done 3a                                         │
│     → Updates .patina/build.toml                                    │
│     → Commits with trailer: Task: 3a, Task-Status: complete         │
│                                                                      │
│   patina build defer "temporal-weighting" --reason "..."            │
│     → Adds to .patina/build.toml                                    │
│     → Commits with trailer: Deferred: temporal-weighting            │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
                                │
                                │ creates
                                ↓
┌─────────────────────────────────────────────────────────────────────┐
│                     Git Log (Audit Trail)                            │
│                                                                      │
│   Every state change is a commit with trailers                      │
│   - When was task 3a completed?                                     │
│   - Who deferred temporal-weighting?                                │
│   - Full history, branch-local                                      │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

---

## State File Schema

```toml
# .patina/build.toml

current_phase = 3

[[phase]]
number = 3
name = "Feedback Loop"
status = "active"  # active | complete
spec = "layer/surface/build/spec-feedback-loop.md"

[[phase]]
number = 4
name = "Build System"
status = "planned"
spec = "layer/surface/build/spec-build-system.md"

[[task]]
id = "3a"
phase = 3
title = "Instrument Scry"
status = "pending"  # pending | active | complete | abandoned

[[task]]
id = "3b"
phase = 3
title = "Session-Commit Linkage"
status = "pending"

[[task]]
id = "3c"
phase = 3
title = "Feedback Views"
status = "pending"

[[task]]
id = "3d"
phase = 3
title = "Eval --feedback"
status = "pending"

[[deferred]]
id = "temporal-weighting"
title = "Temporal weighting for credit assignment"
reason = "Need feedback data first"
target_phase = 5
discovered = "2025-12-17"

[[exploration]]
id = "exp-20251217-103045"
title = "MLP gradient approaches"
status = "active"  # active | completed | abandoned
started = "2025-12-17"
# When ended:
# ended = "2025-12-17"
# outcome = "abandoned"
# learned = "Broken gradient approximation in layer 1"
```

---

## Commands

### Query Commands

```bash
# Show current phase, active task, pending count
patina build status

# List all tasks (optionally filter by phase or status)
patina build tasks
patina build tasks --phase 3
patina build tasks --status pending

# List deferred items
patina build deferred

# List explorations
patina build explorations
```

### Task Commands

```bash
# Start working on a task (pending → active)
patina build task start <id>

# Complete a task (active → complete)
patina build task done <id>

# Abandon a task (active → abandoned)
patina build task abandon <id> --reason "approach doesn't work"

# Add a new task to current phase
patina build task add "<title>"
patina build task add "<title>" --phase 4
```

### Deferred Commands

```bash
# Defer work for later
patina build defer "<title>" --reason "..."
patina build defer "<title>" --reason "..." --target-phase 5

# Pull deferred item into current phase as task
patina build undefer <id>
```

### Exploration Commands (Rabbit Holes)

```bash
# Start exploring
patina build explore "<what you're investigating>"

# End exploration - success
patina build explore done
patina build explore done --learned "discovered X"

# End exploration - dead end (still capture learnings!)
patina build explore abandon --learned "approach doesn't work because Y"
```

### Phase Commands

```bash
# Complete current phase (creates git tag)
patina build phase done

# Start next phase
patina build phase start "<name>"
```

---

## Commit Trailers

Each command creates a commit with structured trailers:

### Task Completion

```
build(task): complete 3a - Instrument Scry

Task: 3a
Task-Status: complete
Phase: 3
```

### Deferring Work

```
build(defer): temporal-weighting

Deferred: temporal-weighting
Deferred-Reason: Need feedback data first
Target-Phase: 5
```

### Exploration

```
build(explore): start exp-20251217-103045

Exploration: exp-20251217-103045
Exploration-Title: MLP gradient approaches
Exploration-Status: started
```

```
build(explore): abandon exp-20251217-103045

Exploration: exp-20251217-103045
Exploration-Status: abandoned
Learned: Broken gradient approximation in layer 1
```

### Querying History

```bash
# When was task 3a completed?
git log --grep="Task: 3a" --format="%h %s (%ai)"

# What got deferred?
git log --grep="Deferred:" --format="%s"

# What was learned from explorations?
git log --grep="Learned:" --format="%b" | grep "Learned:"
```

---

## LLM Integration

### CLAUDE.md Section

Add to project CLAUDE.md:

```markdown
## Build Tracking

Patina tracks work across sessions. Use `patina build` commands:

### At Session Start
Run `patina build status` to see current phase, active tasks, and what's pending.

### When Starting Work
If working on a tracked task:
\`\`\`bash
patina build task start 3a
\`\`\`

### When Completing Work
After committing related changes:
\`\`\`bash
patina build task done 3a
\`\`\`

### When Discovering Future Work
If you find something valuable but out of scope:
\`\`\`bash
patina build defer "temporal-weighting" --reason "need feedback data first"
\`\`\`

### When Going Down Rabbit Holes
If investigating/exploring (not implementing):
\`\`\`bash
patina build explore "investigating MLP gradients"
\`\`\`

When done:
\`\`\`bash
# Success
patina build explore done --learned "found root cause"

# Dead end (still capture learnings!)
patina build explore abandon --learned "approach doesn't work because X"
\`\`\`

### Rule of Thumb
- TodoWrite = tactical, session-scoped, ephemeral
- patina build = strategic, persists across sessions
```

### Session Start Integration

`/session-start` shows build status:

```
$ /session-start "feedback loop work"

Build Status
Phase 3: Feedback Loop (active)

Tasks:
  → 3a: Instrument Scry (active)
    3b: Session-Commit Linkage (pending)
    3c: Feedback Views (pending)

Explorations: none

Deferred: 2 items
```

---

## Implementation

### Task 4a: TOML Schema and Parser

- [ ] Define `BuildState` struct matching schema above
- [ ] Parse/serialize with `toml` crate
- [ ] Validate state transitions (pending → active → complete)

### Task 4b: Query Commands

- [ ] `patina build status` - summary view
- [ ] `patina build tasks` - list with filters
- [ ] `patina build deferred` - list deferred items
- [ ] `patina build explorations` - list explorations

### Task 4c: Mutation Commands

- [ ] `patina build task start/done/abandon/add`
- [ ] `patina build defer/undefer`
- [ ] `patina build explore/explore done/explore abandon`
- [ ] `patina build phase done/start`

### Task 4d: Commit Integration

- [ ] Auto-commit after each mutation
- [ ] Format trailers correctly
- [ ] Use conventional commit format: `build(type): message`

### Task 4e: CLAUDE.md Integration

- [ ] Add build tracking section to CLAUDE.md template
- [ ] Update session-start to show build status
- [ ] Document in project CLAUDE.md

### Task 4f: Migration

- [ ] Create initial `.patina/build.toml` from current `build.md`
- [ ] One-time migration of existing phases/tasks

---

## Validation Checklist

- [ ] `.patina/build.toml` created and parseable
- [ ] `patina build status` shows current state
- [ ] `patina build task done X` updates TOML and commits with trailer
- [ ] `patina build defer` captures deferred work
- [ ] `patina build explore` tracks rabbit holes
- [ ] `git log --grep="Task:"` finds task history
- [ ] CLAUDE.md documents usage
- [ ] Session start shows build status

---

## Files Changed

| File | Change |
|------|--------|
| `src/commands/build/mod.rs` | New command module (~300 lines) |
| `src/commands/build/state.rs` | TOML parsing/serialization (~100 lines) |
| `src/main.rs` | Add Build command enum |
| `.patina/build.toml` | New state file |
| `CLAUDE.md` template | Add build tracking section |
| `resources/claude/commands/session-start.md` | Show build status |

**Total: ~400 lines of new Rust code**

---

## Future Work (Not This Phase)

- **MCP tools**: Expose build commands as MCP tools
- **Build status in context**: Include in `patina context` output
- **Cross-session analytics**: Which tasks take longest? Which get abandoned?
- **Dependency tracking**: Task X blocked by task Y
