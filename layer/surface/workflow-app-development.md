---
id: workflow-app-development
version: 1
status: draft
created_date: 2025-08-03
oxidizer: nicabar
tags: [workflow, app-development, session-management, git-integration]
---

# Complete Workflow: Building an App with Patina

This workflow demonstrates how all Patina systems work together across multiple sessions to build an app.

## Session 1: Project Setup and Initial Design

```bash
# Start your first session
/session-start "todo app initial design"

# Initialize the Patina domain for your project
patina init todo-app --type=app
cd todo-app

# The indexer starts empty - no patterns yet
patina navigate "todo app architecture"
> No patterns found. Start exploring!

# Begin designing
vim layer/surface/todo-app-design.md
# Write initial thoughts about architecture

# As you work, update the session
/session-update
# Claude captures: Created initial design doc, exploring architecture

# Maybe create a quick spike
mkdir -p src/models
vim src/models/todo.rs
# Write some experimental code

# End session - patterns get extracted
/session-end
# Session archived to layer/sessions/
# Key patterns extracted to surface/
```

**What happened behind the scenes:**
- Session file tracked your git state (untracked files)
- Indexer noticed new files in surface/
- Git state = "Untracked" → Confidence = "Experimental"

## Session 2: Implementing Core Features

```bash
# Start new session, continuing from last
/session-start "implement todo CRUD"

# Navigate to find yesterday's work
patina navigate "todo app design"
> Surface: todo-app-design.md (Experimental, untracked)
> Hint: Found in workspace 'main', last modified yesterday

# Read the design
cat layer/surface/todo-app-design.md

# Start implementing based on the design
cargo new --lib todo_core
vim todo_core/src/lib.rs

# Commit your work
git add .
git commit -m "feat: add todo CRUD operations"

# The pattern proves useful - promote to core
patina promote surface/todo-app-design.md --to core/todo-architecture.md

# Add verification to make it truly "core"
vim layer/core/todo-architecture.md
# Add verification section that greps for actual implementation

/session-update
# Claude notes: Implemented CRUD, promoted design to core

/session-end
```

**What happened:**
- Git state changed: Untracked → Committed
- Confidence increased: Experimental → Medium
- Pattern promoted: Surface → Core (with verification)
- Indexer updated: Now shows pattern in core with "Verified" status

## Session 3: Adding Authentication (New Branch)

```bash
/session-start "add authentication feature"

# Create feature branch
git checkout -b feature/auth

# Check what patterns exist
patina navigate "authentication"
> Core: auth-pattern.md (Verified, from main branch)
> Surface: jwt-refresh-experiment.md (Low confidence, 2 weeks old)

# Read the core pattern
cat layer/core/auth-pattern.md
# See the verification code - this pattern is proven!

# Implement auth following the pattern
vim src/auth/mod.rs
# Code following core pattern

# But you need something new - JWT refresh tokens
vim layer/surface/auth/jwt-refresh-implementation.md
# Document your new approach

# Commit on feature branch
git add .
git commit -m "feat: implement JWT refresh tokens"

# Create PR
gh pr create --title "Add authentication with JWT refresh"

/session-end
```

**What happened:**
- Working on feature branch tracked separately
- New patterns get "Medium" confidence (committed but not merged)
- Indexer shows both branch context and confidence
- LLM knows jwt-refresh is experimental, not production

## Session 4: Code Review and Refinement

```bash
/session-start "address auth PR feedback"

# Navigate to see your auth work across branches
patina navigate "authentication" --all-branches
> Core: auth-pattern.md (Verified, main branch)
> Surface: auth/jwt-refresh-implementation.md (High, PR #23 open)
> Dust: oauth-experiment.md (Historical, abandoned branch)

# PR got feedback - need changes
vim src/auth/mod.rs
# Address review comments

# Update the pattern doc too
vim layer/surface/auth/jwt-refresh-implementation.md
# Document learnings from review

git add .
git commit -m "fix: address PR review feedback"
git push

/session-end
```

## Session 5: Merging and Pattern Evolution

```bash
/session-start "merge auth and update patterns"

# PR approved and merged!
git checkout main
git pull

# The JWT refresh pattern proved valuable
# Promote it to core with verification
patina promote surface/auth/jwt-refresh-implementation.md \
  --to core/jwt-refresh-pattern.md \
  --add-verification

vim layer/core/jwt-refresh-pattern.md
# Add bash verification that greps for the implementation

# Old auth pattern still works but is less preferred now
patina deprecate core/auth-pattern.md \
  --reason "Superseded by jwt-refresh-pattern" \
  --move-to dust/deprecated/

# Run navigation again
patina navigate "authentication"
> Core: jwt-refresh-pattern.md (Verified, merged to main ✓)
> Dust: deprecated/auth-pattern.md (Historical, superseded)

/session-end
```

**What happened:**
- Git state: PR Merged → Confidence: Verified
- Pattern lifecycle: Surface → Core (after proving useful)
- Old pattern: Core → Dust (deprecated but preserved)
- Indexer reflects the new reality

## Session 6: Weeks Later - New Developer Joins

```bash
/session-start "onboard new dev sarah"

# Sarah clones the repo and asks about auth
patina navigate "How do we handle authentication?"

> Core: jwt-refresh-pattern.md 
>   Confidence: VERIFIED ✓
>   Status: Merged to main 3 weeks ago
>   Verification: ✓ All checks pass
>   Summary: "JWT with refresh token rotation"
>
> Dust: deprecated/auth-pattern.md
>   Confidence: HISTORICAL 
>   Status: Deprecated 3 weeks ago  
>   Reason: "Superseded by jwt-refresh-pattern"
>   Summary: "Basic JWT without refresh"

# Sarah can immediately see:
# 1. What's currently used (core + verified)
# 2. What was tried before (dust)
# 3. Why things changed (deprecation reason)

# She can even explore abandoned experiments
patina navigate "oauth"
> Dust: experiments/oauth-spike.md
>   From branch: feature/oauth (deleted)
>   Abandoned: 2 months ago
>   Summary: "OAuth2 integration attempt - too complex for our needs"
```

## The Complete Picture

Across these sessions:
1. **Knowledge accumulated** in layers (surface → core → dust)
2. **Git states provided confidence** (untracked → committed → merged)
3. **Indexer helped navigation** ("where is auth knowledge?")
4. **Patterns evolved naturally** (basic auth → jwt refresh)
5. **History was preserved** (can see why OAuth was abandoned)
6. **New developers onboard quickly** (clear what's current vs historical)

The git-aware navigation enhanced the base system by:
- Tracking which branch created patterns
- Showing PR status and merge state  
- Providing confidence based on git lifecycle
- Preserving context about abandoned work

All while the core layer system organized knowledge by actual usage and verification.