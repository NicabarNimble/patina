# Patina Development Workflow: Planning → Micro-PRs → Execution

## Phase 1: Planning Session

### 1. Planning Session (Claude + Human)
```markdown
/session-start planning-auth-system

**Human**: "I want JWT auth with refresh tokens"
**Claude**: Creates requirements...
→ OUTPUT: requirements/auth-system.md
```

### 2. Requirements Breakdown
```markdown
# Auth System Requirements

## User Stories
1. As a user, I can register with email/password
2. As a user, I can login and receive JWT
3. As a user, I can refresh my token

## Technical Requirements
- [ ] JWT generation with RS256
- [ ] Refresh token storage
- [ ] Middleware for validation
- [ ] Rate limiting on auth endpoints
```

### 3. Micro-PR Generation
Claude breaks down into atomic tasks:
```yaml
# .github/issues/auth-tasks.yml
tasks:
  - title: "Create JWT service trait"
    test: "cargo test jwt_service_trait"
    files: ["src/auth/jwt.rs"]
    
  - title: "Implement token generation"
    test: "cargo test generate_token"
    depends_on: ["Create JWT service trait"]
    
  - title: "Add refresh token storage"
    test: "cargo test refresh_token_storage"
    files: ["src/auth/storage.rs"]
```

## Phase 2: Execution Session

### 1. Start Execution
```bash
/session-start execute-auth-plan

# Claude reads the task file and starts working through them
# Each task becomes a micro-PR with:
- Failing test first (Red)
- Implementation (Green)  
- Cleanup (Refactor)
```

### 2. GitHub Actions Runner
```yaml
# .github/workflows/micro-pr-executor.yml
name: Execute Micro PRs

on:
  workflow_dispatch:
    inputs:
      task_file:
        description: 'Task file to execute'
        required: true

jobs:
  execute-tasks:
    runs-on: ubuntu-latest
    steps:
      - name: Read tasks
        run: |
          tasks=$(yq e '.tasks[]' ${{ inputs.task_file }})
          
      - name: For each task
        run: |
          # Create branch
          # Write failing test
          # Create PR
          # Implement solution
          # Run tests
          # Merge if green
```

### 3. Sub-Agent Architecture
```markdown
.claude/agents/
├── planner.md          # Breaks down requirements
├── test-writer.md      # Writes failing tests
├── implementer.md      # Makes tests pass
├── reviewer.md         # Reviews micro-PRs
└── documenter.md       # Updates progress docs
```

## Phase 3: Progress Tracking

### Real-time Dashboard
```markdown
# .github/PROGRESS.md
## Auth System Implementation

### Current Status: 45% Complete

#### Completed ✅
- [x] JWT service trait (PR #101)
- [x] Token generation (PR #102)

#### In Progress 🔄
- [ ] Refresh token storage (PR #103) - Tests failing

#### Blocked 🚫
- [ ] Rate limiting - Waiting on Redis setup

#### Up Next ⏳
- [ ] Middleware implementation
- [ ] Integration tests
```

### Pattern Extraction
After each micro-PR merges:
```bash
# GitHub Action extracts patterns
- "JWT + Rust = Use jsonwebtoken crate"
- "Refresh tokens need Redis for revocation"
→ Adds to layer/topics/auth/
```

## The Beautiful Part

1. **Planning session** produces a concrete artifact (requirements.md)
2. **Requirements** become executable GitHub issues/tasks
3. **Each task** is independently testable
4. **Progress** is visible in GitHub (not hidden in Claude's memory)
5. **Patterns** are extracted automatically from what actually worked

## Tools Integration

**With Dagger:**
```go
// Each micro-PR runs in container
func (m *MicroPR) Execute(ctx context.Context) error {
    return dag.
        Container().
        From("rust:latest").
        WithExec([]string{"cargo", "test", m.TestName}).
        Sync(ctx)
}
```

**With GitHub CLI:**
```bash
# Claude can drive everything with gh commands
gh issue create --title "Implement JWT service"
gh pr create --title "Red: JWT service test"
gh pr merge --auto
```

This turns Claude into a **project manager + developer**, with GitHub as the persistent brain!

Want me to create a concrete example with one of Patina's features?