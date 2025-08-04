---
id: workflow-foss-contribution
version: 1
status: draft
created_date: 2025-08-03
oxidizer: nicabar
tags: [workflow, foss, open-source, contribution, onlydust]
---

# FOSS Contribution Workflow with Patina

This workflow shows how contributors can use Patina to understand and contribute to open source projects effectively.

## Initial Setup: Fork & Clone with Patina

```bash
# Fork the project on GitHub first, then:
git clone https://github.com/yourusername/rust-analyzer
cd rust-analyzer

# Initialize Patina in a separate branch to avoid conflicts
git checkout -b patina-layer
patina init . --type=contribution --template=rust

# This creates:
# - layer/ directory (gitignored in main project)
# - .patina/ for local state
# - Indexer configured for code exploration

# Add layer/ to the project's .gitignore if needed
echo "layer/" >> .gitignore
git add .gitignore
git commit -m "chore: ignore patina layer directory"
```

## Session 1: Initial Exploration

```bash
/session-start "understand rust-analyzer architecture"

# Start exploring the codebase
patina navigate "How does parsing work?"
> No patterns yet. Let's explore...

# Use indexer to scan the codebase
patina index --scan-code
# This extracts patterns from:
# - README files
# - Architecture docs  
# - Code comments marked with NOTE/TODO
# - Test descriptions

# Now navigate works better
patina navigate "parsing"
> Found in docs/dev/architecture.md
> Found in crates/parser/README.md
> References in crates/syntax/src/parsing.rs

# As you read code, document your understanding
vim layer/surface/explorations/parsing-flow.md
```

```markdown
# Rust Analyzer Parsing Flow

Based on my exploration:

1. Entry point: `crates/parser/src/lib.rs`
2. Token stream created by lexer
3. Parser builds untyped syntax tree
4. Later passes add semantic information

Key insight: It's a multi-pass architecture!

Questions:
- How does error recovery work?
- Where do macros get expanded?
```

```bash
# Found something interesting in the code
/session-note "Parser uses Pratt parsing for expressions - see expr.rs:parse_expr"

# Create a pattern from your learning
patina add topic parsing "pratt-parsing-in-rust-analyzer"

/session-end
```

## Session 2: Deep Dive on Specific Feature

```bash
/session-start "understand error recovery for PR"

# You want to contribute better error recovery
# First understand current implementation
patina navigate "error recovery parsing"
> Surface: parsing-flow.md (Medium confidence, your notes)
> Code: crates/parser/src/event.rs:L89 (found by indexer)

# Document the current approach
vim layer/surface/investigations/error-recovery-current.md
# Document how it currently works

# Design your improvement
vim layer/surface/proposals/better-error-recovery.md
```

```markdown
# Proposal: Better Error Recovery in Parsing

## Current State
- Parser bails on first error
- Limited recovery in expressions

## Proposed Change  
- Add synchronization points
- Resume parsing after errors
- Similar to Roslyn's approach

## Implementation Plan
1. Add sync tokens to parser
2. Modify error handling in expr.rs
3. Add tests for recovery scenarios
```

```bash
# Start implementing in a feature branch
git checkout -b feature/better-error-recovery

# As you code, track patterns
/session-update
# Working on error recovery, found sync points pattern

# Document patterns you discover
vim layer/surface/patterns/sync-point-recovery.md
# This pattern might be useful for other parsers too!

/session-end
```

## Session 3: Creating the PR

```bash
/session-start "submit error recovery PR"

# Your implementation is ready
# Document what you learned for the PR
patina generate pr-context "error recovery"
> Generated context from your patterns:
> - Current implementation notes
> - Design decisions
> - Test scenarios considered

# This helps write a better PR description
gh pr create --title "Improve parser error recovery"

# In PR description, you can reference your research
# "After studying the codebase (see my notes on Pratt parsing...)""

/session-end
```

## Session 4: OnlyDust.com Contribution

```bash
/session-start "tackle onlydust issue #423"

# OnlyDust issue: "Optimize memory usage in parser"
# First, understand current memory usage
patina navigate "memory allocation parser"
> Surface: investigations/parser-memory-profile.md
> Pattern: rust-analyzer/arena-allocation.md

# You've built up knowledge from previous sessions!
# Check your existing patterns
patina list patterns --topic memory
> arena-allocation.md (High confidence)
> string-interning.md (Medium confidence)

# Apply your knowledge to the issue
vim layer/surface/solutions/parser-memory-optimization.md
```

```markdown
# Memory Optimization for Parser (OnlyDust #423)

## Analysis
Based on profiling and my understanding:
- Parser allocates many small nodes
- String duplication in tokens

## Solution
Apply arena allocation pattern (see layer/core/arena-allocation.md)
Combined with string interning

## Measurement
Before: 120MB for large file
After: 89MB (26% reduction)
```

## Advanced: Cross-Project Learning

```bash
# You're working on multiple Rust projects
cd ~/projects

# Create a shared Rust patterns domain
patina init rust-patterns --type=shared-domain

# Now in any Rust project
cd some-rust-project
patina connect ~/projects/rust-patterns

# Navigate across projects
patina navigate "error handling" --domain all
> Local: error-handling.md (this project)
> Shared: rust-patterns/error-handling-best-practices.md
> Shared: rust-patterns/anyhow-vs-thiserror.md
```

## Benefits for FOSS Contributors

1. **Build Understanding Incrementally**
   - Each session adds to your knowledge
   - Patterns accumulate across contributions
   - Never lose hard-won insights

2. **Manage Complexity**
   - Large codebases become navigable
   - Your understanding is searchable
   - Connect patterns across files

3. **Better PRs**
   - Document your reasoning
   - Show you understand the codebase
   - Reference specific patterns

4. **Portfolio Building**
   - Your layer/ becomes a knowledge portfolio
   - Shows deep understanding
   - Demonstrates thoughtful approach

5. **Reusable Knowledge**
   - Patterns from one project help another
   - Build expertise systematically
   - Share patterns with other contributors

## OnlyDust.com Specific Features

```bash
# Track OnlyDust rewards and progress
patina add metadata onlydust-stats
# Tracks which patterns led to successful PRs

# Generate contribution reports
patina report contributions --month 10
> PRs Merged: 5
> Patterns Created: 23  
> Knowledge Areas: parsing, memory, error-handling

# Share successful patterns
patina publish pattern "sync-point-recovery" \
  --to community/parser-patterns
# Help other contributors!
```

This workflow turns chaotic open source exploration into systematic knowledge building. Your Patina layer becomes your personal knowledge base about each project, making you a more effective contributor over time.