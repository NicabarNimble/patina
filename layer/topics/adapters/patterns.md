---
id: patterns
version: 1
created_date: 2025-07-22
confidence: medium
oxidizer: nicabar
tags: []
promoted_from: projects/patina
---

# Claude Adapter Patterns

## Overview
Claude-specific patterns for optimal interaction with Patina projects.

## Key Patterns

### 1. Context File Structure
```
.claude/
├── context/
│   ├── PROJECT_DESIGN.toml  # Project spec
│   ├── infrastructure.toml  # Environment details
│   └── sessions/            # Development history
└── commands/                # Custom commands
```

### 2. Session Management

#### Philosophy: Capture Raw, Distill Later
Sessions follow a two-phase approach:
- **Capture Phase**: Low-friction recording during work
- **Distill Phase**: Intelligence extraction at session end

#### Command Details

**`/session-start [name]`**
- Creates timestamped file: `YYYYMMDD-HHMM-name.md`
- Captures git state (branch, commit, uncommitted files)
- Shows previous session context
- No overwrites - always unique filenames

**`/session-update`**
- Adds time-span marker (e.g., "14:30 - Update (covering since 14:15)")
- Claude fills in what happened during the period
- Tracks: files examined, decisions made, patterns discovered
- Zero friction - just marks the time

**`/session-note "insight"`**
- Captures human insights directly
- High-priority input for distillation
- Examples: "Rails pattern working well", "Consider async here"
- Distinct from updates - these are human judgments

**`/session-end`**
- Runs final update automatically
- Shows git statistics (commits, files changed)
- Requires filling 4 sections:
  - What We Did (factual summary)
  - Key Insights (learnings, especially from Notes)
  - Patterns Identified (reusable wisdom)
  - Next Session Should (continuity)
- Creates archive and last-session pointer

#### Best Practices
- Update every 10-30 minutes during active work
- Use notes for "aha moments" and key decisions
- Let Claude fill updates - don't write them manually
- Always complete distillation sections at session end

### 3. Code Generation Rules
- Generate Rust code directly
- Generate other languages via templates only
- Never modify generated non-Rust files
- Always validate with Rust compiler

### 4. Build Workflow
```bash
# Claude executes these commands
patina build     # Smart build with Dagger/Docker
patina test      # Run tests in container
patina push      # Generate context
```

### 5. Pattern Usage
- Read patterns from brain before implementing
- Commit successful patterns back to brain
- Use escape hatches when stuck

## Claude-Specific Instructions

### For Patina Development
1. Always run `cargo check` after code changes
2. Use `patina build` not direct docker/go commands
3. Read brain patterns before implementing features
4. Document decisions in session files

### For Projects Using Patina
1. Start with PROJECT_DESIGN.toml review
2. Check brain for relevant patterns
3. Use generated pipelines, don't modify
4. Fall back to Docker when needed

## Integration Points

### With Dagger
- Claude can read pipelines/main.go to understand
- Claude runs `patina build` to execute
- Claude never modifies Go code directly

### With Docker
- Claude can modify Dockerfile
- Claude uses docker commands for debugging
- Always available as escape hatch

### With Brain
- Claude reads patterns for context
- Claude commits new patterns via `patina add`
- Claude helps patterns evolve over time