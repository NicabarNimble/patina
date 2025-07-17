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
- Use `/session-start` to begin work
- Update with `/session-update` for progress tracking
- Add insights with `/session-note` for human context
- End with `/session-end` for summary

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