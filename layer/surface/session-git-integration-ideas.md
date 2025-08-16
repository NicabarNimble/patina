---
id: session-git-integration-ideas
status: active
created: 2025-08-14
session: git-knowledge-evolution
tags: [git, llm-memory, workflow, session-management, pattern-tracking]
---

# Git Integration Ideas from Session: git-knowledge-evolution

A comprehensive capture of ideas discussed while exploring how Git can serve as memory for LLMs and enable pattern survival tracking.

## Core Realization: LLMs Do the Git Work

**Key Insight**: In a 1 person + LLM team, the human doesn't do Git operations - the LLM does. The human just guides with reminders like "git like a scalpel not a shotgun." This fundamentally changes how we should think about Git integration.

## The Problem Space

### Current Git Functionality Analysis

**What Patina Has (80% Complete)**:
1. **Git State Machine** (`indexer/internal/state_machine.rs`)
   - Beautiful lifecycle tracking: Untracked → Modified → Staged → Committed → Pushed → PR → Merged
   - Maps confidence levels to Git states
   - Full PR and merge tracking capabilities
   - **Problem**: Not actually connected to real Git events

2. **Git Detection** (`indexer/internal/git_detection.rs`)
   - Uses shell commands (no git2 dependency - philosophy win!)
   - Detects file states via `git status --porcelain`
   - Batch state detection for efficiency
   - **Problem**: Navigate command doesn't actually use it

3. **Workspace Client** (`workspace_client.rs`)
   - Has Git methods for remote workspace management
   - `get_git_status()`, `create_branch()`, `commit_changes()`
   - **Problem**: Seems designed for different architecture, unused

4. **Navigation Integration** (`commands/navigate.rs`)
   - Displays Git state in results
   - Colors output based on confidence
   - **Problem**: Confidence always "High" after any commit

### The Core Disconnect

**Everything gets committed immediately** → All patterns show "High confidence" → No distinction between "just wrote" vs "survived 6 months"

The insight from git-knowledge-evolution.md: **"Code that SURVIVES = Good patterns"** not "Code that's COMMITTED = High confidence"

## Git Workflow Ideas Explored

### Initial Approach: Session = Branch (Rejected)

```bash
/session-start "retry logic"
→ Auto-creates: session/retry-logic-20250814
→ Makes empty commit
→ Forces workflow

/session-end --pr
→ Creates PR automatically
```

**Why Rejected**: 
- LLMs already create branches when needed
- Empty commits are noise
- Forcing workflow confuses LLMs
- Hundreds of session branches accumulate

### Alternative Branch Strategies Considered

**Option 1: Work → Main**
```bash
main (protected)
└── work (default for sessions)
```
Simple but loses session boundaries

**Option 2: Dev → Main**
```bash
main (releases)
└── dev (active work)
    └── session branches → PR to dev
```
Too much structure for 1 person team

**Option 3: Ephemeral Sessions**
```bash
main
└── session/current (reused name)
```
Loses history tracking

**Option 4: Experiment Tracking**
```bash
main
├── exp/retry-v1 (failed, kept)
├── exp/retry-v2 (failed, kept)
└── impl/retry (worked, merged)
```
Good for memory but complex

### Final Approach: Git as Memory Assistant

**Philosophy**: Provide information, not automation

```bash
/session-start "retry logic"
→ Shows: Previous attempts at retry
→ Shows: Failed experiments (exp/retry-v1)
→ Shows: Current branch status
→ Reminds: Git best practices
→ Does NOT: Create branch or commit

/session-update
→ Shows: Commits made this session
→ Shows: Uncommitted changes
→ Shows: Time elapsed
→ Does NOT: Auto-commit or suggest commands

/session-end
→ Shows: Git summary
→ Asks: Classify outcome (feature/bug/experiment)
→ Archives: With Git context
→ Does NOT: Force commits or PRs
```

## Code Survival Tracking Ideas

### Survival Metrics Design

```rust
struct PatternValidation {
    pattern_file: PathBuf,
    commit_sha: String,
    co_committed_files: Vec<PathBuf>,
    
    // Survival metrics
    files_still_exist: u32,      // 8/10 files still there
    lines_unchanged: f32,         // 80% of code unchanged
    survived_refactors: u32,      // Made it through 3 refactors
    age_days: u32,                // Days since creation
}
```

### Confidence Based on Survival

```rust
// Current (broken):
if committed => High confidence

// Proposed:
if age_days < 7 => Experimental
if age_days < 30 => Low  
if age_days < 90 => Medium
if age_days > 90 && modifications < 3 => High
if accepted_in_pr => Verified
```

### Implementation Approaches

**Quick Win**: Fix confidence scoring
```rust
pub fn get_file_age_days(file: &Path) -> u32
pub fn count_modifications(file: &Path) -> u32
pub fn check_file_survival(file: &Path, from_commit: &str) -> bool
```

**Medium Term**: Connect the plumbing
- Make `navigate` call `git_detection`
- Add survival metrics to pattern display
- Track co-committed files

**Long Term**: Layer as Git submodule
```bash
patina/
└── layer/ → submodule to separate repo
    ├── core/    # Proven patterns
    ├── surface/ # Active development
    └── dust/    # Historical
```

## Session Management Evolution

### Sessions vs Features

**Realization**: Sessions track time/effort, branches track features. They overlap but aren't the same.

- **Sessions**: Documentation boundaries, time-boxed work
- **Branches**: Feature boundaries, code organization
- **Experiments**: Failed attempts kept for memory

### Session Classification

Added outcome tracking to help build memory:

```markdown
## Session Outcome
Type: [feature|bugfix|experiment|research|refactor]
Status: [completed|partial|failed|ongoing]
```

This helps distinguish:
- What worked (completed features)
- What failed (failed experiments)
- What's ongoing (partial work)

## LLM Memory Requirements

### What LLMs Actually Need

1. **Context about previous attempts**
   ```bash
   "Previous work on 'retry logic':
    - exp/retry-v1: simple loop (deleted after 2 days)
    - exp/retry-v2: exponential backoff (survived 2 months)"
   ```

2. **Reminders about best practices**
   ```bash
   "Git Reminders:
    - Commit frequently with descriptive messages
    - Use 'git like a scalpel, not a shotgun'
    - Prefix: feat:, fix:, refactor:"
   ```

3. **Pattern recognition from history**
   ```bash
   git log --grep="retry" --oneline  # What retry logic tried?
   git branch -a | grep exp/          # What experiments failed?
   git blame src/retry.rs             # When was this written?
   ```

### What LLMs Don't Need

- Branches created for them
- Commits made for them
- PRs suggested to them
- Workflow enforcement

## Key Insights and Principles

### For 1 Person + LLM Teams

1. **No merge conflicts** - Single developer advantage
2. **Git as pure memory** - Not negotiation or collaboration
3. **Every commit traceable** - Either human or LLM
4. **Failed experiments valuable** - Keep them for learning

### The Real Value Proposition

**Before**: "This pattern exists in the repo"  
**After**: "This pattern survived 6 months through 3 refactors"

**Before**: Confidence based on commit status  
**After**: Confidence based on survival time

**Before**: LLM forgets between sessions  
**After**: Git provides persistent memory

### Design Principles

1. **Information over Automation** - Show context, don't force workflow
2. **Memory over Version Control** - Git tracks what worked/failed
3. **Survival over Existence** - Age and modifications matter
4. **Experiments are Memory** - Failed attempts teach what not to do

## Implementation Status

### Completed in This Session

✅ Analyzed existing Git integration (state machine, detection, navigation)  
✅ Identified core disconnect (confidence scoring broken)  
✅ Revised session scripts to be memory-focused  
✅ Updated command documentation  
✅ Removed workflow enforcement  

### Ready to Implement

🔧 Fix confidence scoring (use age not commit status)  
🔧 Connect git_detection to navigate command  
🔧 Add survival metrics to pattern display  
🔧 Track co-committed files  

### Future Possibilities

🔮 Layer as Git submodule for portability  
🔮 Automatic pattern extraction from surviving code  
🔮 OSS PR validation tracking  
🔮 Repository mining for pattern discovery  
🔮 Semantic analysis with tree-sitter  

## The Big Picture

We're not building Git workflow for humans. We're building **Git memory for LLMs**.

The session system becomes a memory assistant that:
- Shows what's been tried before
- Tracks what survived vs failed
- Reminds about best practices
- Preserves failed experiments as learning

This aligns with the actual workflow where:
- Human provides guidance ("git like a scalpel")
- LLM does the Git operations
- Sessions document the journey
- Git tracks pattern survival

The result: LLMs can learn from history, patterns evolve through survival, and the codebase becomes self-documenting through Git.