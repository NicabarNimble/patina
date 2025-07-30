---
id: repository-workflow
version: 1
created_date: 2025-07-30
confidence: high
oxidizer: nicabar
tags: [development, ci, workflow, github]
---

# Repository Workflow

## Branch Protection Rules

The `main` branch is protected via GitHub Rulesets with the following requirements:

### Required Checks
1. **Pull Request Required** - All changes must go through a PR
2. **Status Check: "Test Suite"** - Must pass before merge
3. **No Force Push** - Non-fast-forward merges prevented
4. **No Branch Deletion** - Main branch cannot be deleted

### CI Pipeline (Test Suite)
The "Test Suite" runs on every PR and includes:

**Rust Checks (Always):**
- Rust formatting check (`cargo fmt --all -- --check`)
- Clippy linting (`cargo clippy --workspace -- -D warnings`)
- Rust tests (`cargo test --workspace`)
- Release build verification (`cargo build --release`)

**Go Checks (When workspace/ directory exists):**
- Go module download (`go mod download`)
- Go tests (`go test -v ./...`)
- Go formatting (`gofmt -l -s .`) - fails if any files need formatting

Note: The `workspace/` directory contains the Go service for Dagger agent environments

## Development Workflow

### 1. Create Feature Branch
```bash
git checkout -b feature/description
# or: refactor/description, fix/description
```

### 2. Make Changes
- Run `cargo fmt` before committing
- Run `cargo clippy` to check for warnings
- Run `cargo test` locally

### 3. Push and Create PR
```bash
git push -u origin branch-name
gh pr create --title "type: description" --body "details"
```

### 4. Wait for CI
The Test Suite must pass before merge is allowed. Check status:
```bash
gh pr checks
```

### 5. Merge
Currently manual merge after CI passes. PR creator can merge their own PR once checks pass.

## For AI Assistants

### Key Points
1. **Never push directly to main** - Will be rejected by ruleset
2. **Always create a branch** for any changes
3. **PR is mandatory** - No exceptions
4. **CI must pass** - "Test Suite" is required
5. **Use descriptive branch names** like `refactor/workspace-to-agent`

### Error Messages
If you see:
- "Repository rule violations found" - You tried to push to main
- "Required status check 'Test Suite' is expected" - CI hasn't passed yet
- "Changes must be made through a pull request" - Create a PR

### Quick Reference
```bash
# Create branch and PR
git checkout -b feature/name
git add -A
git commit -m "feat: description"
git push -u origin feature/name
gh pr create

# Check PR status
gh pr checks
gh pr view --web
```

## Merge Methods
All merge methods are allowed:
- Merge commit
- Squash and merge
- Rebase and merge

Choose based on PR complexity and commit history cleanliness.

## Review Requirements
Currently no review required - PR creator can merge once CI passes. This may change as project grows.