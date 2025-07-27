# Dagger-Based Dogfood Testing Strategy

## Overview
Use Dagger containers to test Patina changes in isolation before applying them to our development environment.

## Core Workflow

### 1. Develop on Feature Branches
```bash
# Create feature branch
git checkout -b feat/better-sessions

# Make changes
vim resources/claude/session-start.sh
vim src/adapters/claude.rs  # Bump CLAUDE_VERSION

# Commit changes
git commit -am "feat(claude): improve session timing"
```

### 2. Test in Dagger Container
```bash
# Test the branch in isolation
patina agent test --branch feat/better-sessions

# What happens in container:
# 1. Clones patina at specified branch
# 2. Builds patina from source
# 3. Creates test project using built patina
# 4. Runs update command to test component updates
# 5. Verifies changes work correctly
```

### 3. Merge and Tag
```bash
# After container tests pass
git checkout main
git merge feat/better-sessions

# Tag the component version
git tag -a claude-v0.3.1 -m "Improved session timing"
git push origin main --tags
```

### 4. Update Local Development
```bash
# Now update our working patina
patina update
> Updating claude: v0.3.0 → v0.3.1

# Continue development with latest improvements
```

## Key Benefits

### Isolation
- Each test runs in clean container
- No contamination from dev environment
- Reproducible results

### Safety
- Test updates before applying them
- Verify both code AND update mechanism
- Catch issues before they affect development

### Speed
- Dagger caching makes rebuilds fast
- Parallel testing of multiple branches
- Quick iteration cycles

## Testing Scenarios

### Component Update Test
```bash
# Test that component updates work
patina agent test --branch feat/claude-update --scenario update
# Verifies: version detection, file extraction, manifest update
```

### Cross-Component Test
```bash
# Test that multiple components work together
patina agent test --branch feat/integration --scenario full
# Verifies: claude + dagger + core all function correctly
```

### Rollback Test
```bash
# Test version downgrades
patina agent test --branch main --scenario rollback
# Verifies: can downgrade to previous versions safely
```

## Implementation Notes

The `patina agent test` command should:
1. Spin up fresh Dagger container
2. Clone patina at specified branch
3. Build patina in container
4. Create test project(s)
5. Run test scenario
6. Report results

This enables us to confidently use Patina to develop Patina.