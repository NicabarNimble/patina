---
id: git-universal-integration
status: active
created: 2025-08-16
session: learn-new-git-commands
tags: [git, integration, llm-agnostic, universal-tools]
references: [git-hooks-integration.md, git-integration-timeline.md, git-knowledge-evolution.md]
---

# Universal Git Integration: LLM-Agnostic Approach

A simpler, more robust approach to Git integration that works with ANY LLM, not just Claude Code with hooks.

## The Problem with Hooks

- **Fragile**: One syntax error breaks Claude
- **Claude-specific**: Won't work with Gemini, Llama, etc.
- **Complex**: JSON parsing, Ruby/Python scripts, permissions
- **Hidden**: Hard to debug when things go wrong
- **Dangerous**: Auto-execution can cause problems

## The Universal Solution: Git Aliases + Shell Scripts

Instead of hooks, use simple Git aliases and shell scripts that ANY LLM can call:

```bash
# Any LLM can run these
git session-save
git pattern-save
git survival-check file.rs
```

## Implementation: Simple Shell Commands

### 1. Core Git Aliases

Add to `.gitconfig` or run these commands:

```bash
# Save session work
git config --global alias.session-save '!f() { 
  git add .claude/context/ .gemini/context/ .llm/ 2>/dev/null; 
  git commit -m "session: ${1:-checkpoint}" 2>/dev/null || true; 
}; f'

# Save pattern changes
git config --global alias.pattern-save '!f() { 
  git add layer/ 2>/dev/null; 
  git commit -m "patterns: ${1:-updated}" 2>/dev/null || true; 
}; f'

# Check file survival
git config --global alias.survival '!f() { 
  echo "File: $1"; 
  echo -n "Age: "; git log -1 --format="%ar" -- "$1" 2>/dev/null || echo "new"; 
  echo -n "Commits: "; git log --oneline -- "$1" 2>/dev/null | wc -l; 
}; f'

# Show co-modified files
git config --global alias.comodified '!f() { 
  git log --name-only --pretty=format: -- "$1" | sort | uniq -c | sort -rn | head -5; 
}; f'
```

### 2. Patina Commands That Use Git

Update Patina to use Git directly:

```rust
// src/commands/session.rs
pub fn session_save(message: &str) -> Result<()> {
    // Just call git
    Command::new("git")
        .args(&["session-save", message])
        .status()?;
    Ok(())
}

// src/commands/navigate.rs
pub fn get_survival_metrics(file: &Path) -> SurvivalMetrics {
    let output = Command::new("git")
        .args(&["survival", file.to_str().unwrap()])
        .output()?;
    // Parse simple text output
}
```

### 3. Enhanced Session Scripts (LLM-Agnostic)

Update the existing session scripts to use Git:

```bash
#!/bin/bash
# session-start.sh - Works with ANY LLM
SESSION_ID="$(date +%Y%m%d-%H%M%S)"
SESSION_TITLE="${1:-untitled}"

# Create session file (works for any LLM)
mkdir -p .llm/context
cat > .llm/context/active-session.md << EOF
# Session: ${SESSION_TITLE}
**ID**: ${SESSION_ID}
**Started**: $(date -u +"%Y-%m-%dT%H:%M:%SZ")
EOF

# Use git to save (universal command)
git add .llm/context/active-session.md 2>/dev/null
git commit -m "session: start - ${SESSION_TITLE}" 2>/dev/null || true

echo "Session started: ${SESSION_ID}"
echo "Run 'git session-save' periodically to checkpoint"
```

```bash
#!/bin/bash
# session-update.sh - Universal
CURRENT_TIME=$(date +"%H:%M")

# Append to session (works for any context directory)
for dir in .claude .gemini .llm; do
  if [ -f "$dir/context/active-session.md" ]; then
    echo "" >> "$dir/context/active-session.md"
    echo "### $CURRENT_TIME - Update" >> "$dir/context/active-session.md"
    break
  fi
done

# Tell user to save with git
echo "Session updated. Run: git session-save \"your message\""
```

### 4. Universal Pattern Commands

```bash
#!/bin/bash
# patina-git-enhance.sh - Install universal Git commands

# Check survival of patterns
git config --global alias.pattern-age '!f() {
  for file in layer/**/*.md; do
    [ -f "$file" ] || continue
    age=$(git log -1 --format="%ar" -- "$file" 2>/dev/null || echo "new")
    echo "$(basename $file): $age"
  done | sort
}; f'

# Track what changes together
git config --global alias.track-comodified '!f() {
  echo "$1:$2:$(date +%Y%m%d-%H%M%S)" >> .git/comodifications.log
  git add .git/comodifications.log 2>/dev/null
  git commit -m "track: comodified $1 and $2" 2>/dev/null || true
}; f'

# Show Git memory
git config --global alias.memory '!f() {
  echo "=== Recent Sessions ==="
  git log --grep="^session:" --format="%ar: %s" -10
  echo ""
  echo "=== Recent Patterns ==="
  git log --grep="^patterns:" --format="%ar: %s" -10
  echo ""
  echo "=== Survival Champions ==="
  for f in $(git ls-files layer/core/*.md | head -5); do
    age=$(git log -1 --format="%ar" -- "$f")
    echo "  $(basename $f): survived $age"
  done
}; f'
```

## Integration Points

### 1. Manual Triggers (User Controls)

Instead of auto-hooks, users explicitly run:

```bash
# During work
git session-save "implemented auth"
git pattern-save "added retry pattern"

# Check before modifying
git survival src/auth.rs
git comodified src/auth.rs

# Review memory
git memory
```

### 2. LLM Prompts (Any LLM)

Add to CLAUDE.md, GEMINI.md, or any LLM instructions:

```markdown
## Git Memory Commands

When working on this project, use these commands:
- `git session-save "message"` - Save session progress
- `git pattern-save "message"` - Save pattern updates  
- `git survival <file>` - Check file age before modifying
- `git comodified <file>` - See what changes with this file
- `git memory` - Show recent work and patterns
```

### 3. Patina Integration

```bash
# Patina wraps Git commands
patina session save  # Calls: git session-save
patina survival <file>  # Calls: git survival
patina navigate <pattern>  # Uses: git log for confidence
```

## Benefits of Universal Approach

### Works Everywhere
- **Claude Code**: Can call via Bash tool
- **Gemini**: Can call via shell
- **Cursor**: Can call via terminal
- **Human**: Can call directly

### Simple to Debug
```bash
# See what would happen
git session-save --dry-run

# Check if working
git survival README.md

# View all custom commands
git config --get-regexp alias
```

### No Lock-in
- No vendor-specific hooks
- No JSON parsing needed
- No Ruby/Python dependencies
- Just Git commands

### Safe Fallbacks
All commands use `|| true` to prevent failures:
```bash
git commit -m "message" 2>/dev/null || true
# Never breaks, even if nothing to commit
```

## Migration from Hooks

If you have hooks installed:

1. **Disable hooks**: Remove from `.claude/settings.json`
2. **Install aliases**: Run `patina init --git-aliases`
3. **Update workflow**: Use `git session-save` instead of auto-commit
4. **Test**: Verify commands work in terminal first

## Example Workflow

```bash
# Start work (any LLM)
./session-start.sh "implement auth"

# LLM makes changes
# ...

# Save progress (explicitly)
git session-save "added login form"

# Check survival before big changes
git survival src/core/auth.rs
# Output: Age: 6 months, Commits: 45

# Continue work
# ...

# End session
./session-end.sh
git session-save "completed auth implementation"
```

## Patina Command to Install

```rust
// src/commands/init.rs
pub fn install_git_aliases() -> Result<()> {
    let aliases = vec![
        ("session-save", r#"!f() { git add .llm/ .claude/ .gemini/ 2>/dev/null; git commit -m "session: ${1:-checkpoint}" 2>/dev/null || true; }; f"#),
        ("pattern-save", r#"!f() { git add layer/ 2>/dev/null; git commit -m "patterns: ${1:-updated}" 2>/dev/null || true; }; f"#),
        ("survival", r#"!f() { echo "File: $1"; echo -n "Age: "; git log -1 --format="%ar" -- "$1" 2>/dev/null || echo "new"; echo -n "Commits: "; git log --oneline -- "$1" 2>/dev/null | wc -l; }; f"#),
        // ... more aliases
    ];
    
    for (name, command) in aliases {
        Command::new("git")
            .args(&["config", "--global", &format!("alias.{}", name), command])
            .status()?;
    }
    
    println!("✓ Installed Git aliases for universal LLM usage");
    Ok(())
}
```

## Testing

```bash
# Test without any LLM
git session-save "test"
git survival README.md
git memory

# Should all work in plain terminal
```

## Conclusion

Universal Git commands are:
- **Simpler** than hooks
- **Safer** (explicit control)
- **Portable** across all LLMs
- **Debuggable** in terminal
- **Composable** with existing tools

This approach gives you Git memory without the fragility of hooks or vendor lock-in.