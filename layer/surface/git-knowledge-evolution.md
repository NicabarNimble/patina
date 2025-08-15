---
id: git-knowledge-evolution
status: active
created: 2025-08-14
tags: [architecture, git, knowledge-management, pattern-validation, submodules]
references: [git-aware-navigation-design.md, pattern-selection-framework.md, docling-pattern-vector-storage.md]
---

# Git-Based Knowledge Evolution & Pattern Validation

A system for tracking pattern quality through code survival, managing knowledge as Git submodules, and building software faster with persistent LLM memory.

## Core Problem

We need to:
1. **Build software faster** - LLM keeps rewriting the same patterns differently
2. **Maintain LLM memory** - Decisions and patterns persist across sessions
3. **Understand FOSS repos** - Quickly analyze codebases to contribute effectively
4. **Track pattern quality** - Know which patterns actually work vs theoretical ideas

## Key Insight: Code Survival = Pattern Quality

**Patterns that get committed with code that SURVIVES are good patterns.**
**Patterns committed with code that gets deleted/rewritten are bad patterns.**

## Existing Tools & Research

### Repository Mining & Survival Analysis

**Commercial Tools:**
- **Microsoft CODEMINE** - Analyzes code patterns that survive vs die
- **Google Tricorder** - Tracks which code patterns lead to bugs
- **SourceGraph** - Global reference graphs for pattern tracking
- **DeepCode/Snyk** - ML-based pattern learning from commits

**Open Source Tools:**
- **git-of-theseus** - Generates survival curves showing % of code still alive
- **Hercules** - 20x faster Git analysis with burndown tracking
- **git2net** - Tracks co-committed file correlations

**Academic Field:** "Mining Software Repositories" (MSR)
- Kaplan-Meier survival curves for code
- Cohort analysis of patterns
- Clone evolution tracking

### Linux Kernel Approach

Multi-layered validation:
1. Code enters through "staging tree"
2. Static analyzers check it (Sparse, Smatch, Coccinelle)
3. Peer review with "Signed-off-by" chains
4. Only graduates when proven stable

### Spotify's Golden Path

Pattern validation through usage:
- Used 100+ times → "Golden Path"
- Used 10 times → "Supported"
- Used once → "Experimental"

Result: Reduced service setup from 14 days to 5 minutes!

## Pattern Validation Through Git

### Track Pattern Lifecycle

```bash
# When did pattern first appear?
git log -S "mod internal" --reverse --oneline | head -1

# How many times implemented?
git log -S "mod internal" --oneline | wc -l

# Which files currently use it?
git grep -l "mod internal"

# Track survival through refactors
git log --grep="refactor" -S "mod internal"
```

### Co-Commit Analysis

```bash
# Pattern A committed with 10 files
git show abc123 --name-only

# Check survival
git diff abc123..HEAD --name-status | grep "^D"
# Many deletions = bad pattern

# Pattern B committed with 5 files  
git show def456 --name-only

# Still mostly unchanged?
git diff def456..HEAD --stat
# Minimal changes = good pattern
```

### Survival Scoring

```rust
struct PatternValidation {
    pattern_file: PathBuf,
    commit_sha: String,
    co_committed_files: Vec<PathBuf>,
    
    // Survival metrics
    files_still_exist: u32,      // 8/10 files still there
    lines_unchanged: f32,         // 80% of code unchanged
    survived_refactors: u32,      // Made it through 3 refactors
}
```

## Layer as Git Submodule Strategy

### The Architecture

```bash
# Main Patina repo
patina/
├── src/           # Patina code
├── .git/          # Patina's git
└── layer/         # Submodule pointing to knowledge repo

# Separate knowledge repo
patina-knowledge/
├── .git/
├── core/          # Proven patterns
├── surface/       # Active experiments
└── dust/          # Archived wisdom
```

### Benefits

1. **Knowledge portability** - Patterns travel across projects
2. **Privacy control** - Public tool, private patterns
3. **Version independence** - Knowledge evolves separately from tool
4. **Selective sharing** - Share some patterns, keep others private

### Implementation

```bash
# Convert existing layer to submodule
cd patina
mv layer ../patina-knowledge
git rm -r layer
git submodule add git@github.com:YOU/patina-knowledge.git layer

# For private patterns (using Gitea)
git submodule add git@gitea.local:YOU/private-knowledge.git layer
```

### Multiple Knowledge Sources

```bash
# Public patterns
git submodule add https://github.com/YOU/public-patterns layer/public

# Private patterns
git submodule add git@gitea.local/private-patterns layer/private

# Client-specific patterns
git submodule add git@private/client-patterns layer/client
```

## Workflow for Fast Development

### Starting New Projects

```bash
# Clone with knowledge
git clone --recursive patina
cd patina/layer
git pull  # Get latest patterns

# Start feature with context
patina start "add oauth"
# LLM gets:
# - Your proven auth patterns
# - Previous decisions (SQLite not Redis!)
# - Exact code to copy
```

### Persistent LLM Memory

```bash
# Session 1
You: "Use SQLite for storage"
patina note "Decision: SQLite over Redis - simpler, no deps"

# Session 2 (different LLM instance)
patina context
# Output: "Previous decision: Use SQLite (not Redis)"
```

### FOSS Contribution Workflow

```bash
# Analyze target repo
patina explore https://github.com/cool/project
# Output:
# - Architecture: Clean layers
# - Error handling: Result types
# - Testing: Mock-heavy
# - Entry points: src/main.rs

# Fork and add your patterns
gh repo fork cool/project
cd project
git submodule add git@github.com:YOU/patina-knowledge .patina/layer

# Generate matching their style
patina generate --match-style "cool/project" "Add retry logic"

# Track PR success
patina pr track https://github.com/cool/project/pull/123
# Merged = pattern validated by community
```

## Confidence Metrics for Personal Use

### Not Team Metrics, But Personal Evolution

```
Pattern Confidence = 
  (Your reuse count) × 
  (Survival across your experiments) × 
  (OSS acceptance rate)
```

### Tracking Experiments

```markdown
---
experiments: [auth-v1, auth-v2, new-cli]
survived: [auth-v2, new-cli]
failed: [auth-v1]
oss_prs: [merged: 2, rejected: 1]
confidence: proven
---
```

### OSS Validation

```bash
# Patterns that got merged (market validated)
git log --grep="Merged PR" | grep -l "pattern:"

# Patterns that got rejected (need rework)
git log --grep="Closed PR" | grep -l "pattern:"
```

## Implementation Plan

### Phase 1: Git Submodule Setup
1. [ ] Move layer/ to separate repo
2. [ ] Configure as submodule
3. [ ] Set up Gitea for private patterns
4. [ ] Create public pattern repo

### Phase 2: Survival Analysis
1. [ ] Implement co-commit tracking
2. [ ] Add survival scoring to navigation
3. [ ] Create `patina validate` command
4. [ ] Track pattern usage across projects

### Phase 3: FOSS Integration
1. [ ] Build repo analysis tool
2. [ ] Pattern matching for foreign codebases
3. [ ] PR success tracking
4. [ ] Community validation metrics

### Phase 4: Semantic Enhancement
1. [ ] Add Semgrep for pattern detection
2. [ ] Tree-sitter for AST analysis
3. [ ] Vector embeddings for similarity
4. [ ] Automatic pattern extraction

## Key Commands to Build

```bash
# Validate pattern quality
patina pattern survival "dependable-rust"

# Track experiment success
patina experiment evaluate "auth-v3"

# Analyze FOSS repo
patina explore https://github.com/org/repo

# Generate with proven patterns
patina generate --confidence=proven "implement cache"

# Show pattern evolution
patina pattern history "error-handling"
```

## The System You Actually Get

1. **LLM Memory** - Decisions persist in Git
2. **Fast Building** - Exact patterns to copy, not vague principles
3. **Pattern Validation** - Code survival shows what works
4. **FOSS Understanding** - Quick analysis and style matching
5. **Knowledge Portability** - Your patterns travel with you

This isn't theoretical - it's combining:
- What Patina already has (80% there!)
- Proven techniques from Linux, Google, Microsoft
- Git as the timeline/truth source
- Submodules for knowledge management

The result: Build faster, remember everything, contribute effectively.