---
id: semantic-reality-system
status: exploration
created: 2025-08-20
tags: [architecture, semantic-search, git-integration, tree-sitter, sqlite]
references: [pattern-selection-framework]
---

# Semantic Reality System - Extracting Truth from Code

**Problem**: Our tools create narratives about patterns but can't verify if they're true. We manually grep and read files while our "smart" tools fail at basic tasks.

**Solution**: Stop trying to be intelligent. Extract facts from code structure and Git history, store them queryably, let simple queries reveal patterns.

---

## The Current Failure

### What We Do Manually (Eating Tokens)
1. Read pattern metadata to find references
2. Grep through sessions for discussions
3. Check Git history for evolution
4. Verify module line counts
5. Follow reference chains between patterns

### What Our Tools Actually Do
- `navigate` - Broken text search that doesn't understand relationships
- `trace` - Shows timeline (actually works because it wraps git)
- `recognize` - Finds code patterns but not documentation patterns
- `organize` - Complex scoring that doesn't reflect reality

### The Gap
None of our tools can:
- Follow references between patterns
- Verify if code actually implements patterns
- Show complete context about a topic
- Track pattern adoption vs violation

## The Semantic Reality Approach

### 1. Git as Timeline and Truth

Git already knows everything we need:

```bash
# Git knows references
git log --oneline --grep="references" --all -- "*.md"

# Git knows co-modification (what changes together)
git log --oneline --all -- "*pattern-selection*" "*dependable-rust*"

# Git knows survival (quality metric)
git log --pretty=format:"%ar %f" --all -- "layer/core/*.md"

# Git knows importance (commit frequency)
git shortlog -sn --all -- "layer/surface/*.md"
```

**Key Insight**: Code that survives 6+ months = reality. Patterns never implemented = aspirational.

### 2. GitHub API as Intelligence Layer

GitHub provides semantic understanding we're trying to rebuild:

```bash
# Find all PRs discussing a pattern
gh pr list --search "pattern-selection" --state all --json number,title,body

# Search with GitHub's semantic index
gh api "search/code?q=pattern-selection+repo:owner/patina"

# Get symbol navigation and dependencies
gh api repos/:owner/:repo/dependency-graph
```

### 3. Tree-Sitter for Real Pattern Extraction

Stop grepping for text, start understanding structure:

```rust
// Tree-sitter query to find ACTUAL error handling patterns
(result_expression
  (method_call_expression
    method: (field_identifier) @method
    (#match? @method "context|with_context")))
```

This finds actual implementations, not text matches!

### 4. SQLite as Semantic Storage

Fast analytical queries over code reality:

```sql
-- Schema for semantic reality
CREATE TABLE code_symbols (
    file TEXT,
    symbol TEXT,
    type TEXT,  -- function, struct, trait
    ast_hash TEXT,  -- for structural similarity
    git_commit TEXT,
    survival_days INTEGER
);

CREATE TABLE pattern_implementations (
    pattern_id TEXT,
    file TEXT,
    symbol TEXT,
    compliance FLOAT,  -- 0.0 to 1.0
    verified_at TIMESTAMP
);

CREATE TABLE pattern_references (
    from_pattern TEXT,
    to_pattern TEXT,
    reference_type TEXT,  -- extends, implements, contradicts
    discovered_at TIMESTAMP
);

-- Find all implementations of a pattern
SELECT * FROM pattern_implementations 
WHERE pattern_id = 'dependable-rust'
AND compliance > 0.8;

-- Find patterns that actually survive
SELECT p.name, COUNT(*) as implementations, AVG(s.survival_days) as avg_survival
FROM patterns p
JOIN pattern_implementations pi ON p.id = pi.pattern_id
JOIN code_symbols s ON pi.symbol = s.symbol
WHERE s.survival_days > 180
GROUP BY p.name
ORDER BY implementations DESC;
```

## The Integrated Workflow

### Extract Reality from Code

```bash
# 1. Tree-sitter extracts ALL patterns from surviving code
tree-sitter-rust extract src/ |
  sqlite3 knowledge.db "INSERT INTO discovered_patterns..."

# 2. Git provides timeline and quality metrics
git log --all --format="%H %ar" -- "*.rs" |
  sqlite3 knowledge.db "UPDATE code_symbols SET survival_days = ..."

# 3. GitHub API provides context
gh api graphql -f query="..." |
  sqlite3 knowledge.db "INSERT INTO discussions..."

# 4. Query for reality
sqlite3 knowledge.db "
  SELECT p.name, COUNT(*) as implementations, AVG(s.survival_days)
  FROM patterns p
  JOIN pattern_implementations pi ON p.id = pi.pattern_id
  JOIN code_symbols s ON pi.symbol = s.symbol
  WHERE s.survival_days > 180
  GROUP BY p.name
  ORDER BY implementations DESC
"
```

### The `patina scrape` Concept

A command that reconciles documentation with reality:

```bash
patina scrape

🔍 Analyzing reality vs documentation...

CONFLICT: dependable-rust.md says "≤150 lines"
REALITY: 30 files exceed this (avg 287 lines, survived 180+ days)

❓ How should we reconcile this?
1. Update pattern to match reality (~300 lines OK for commands)
2. Mark pattern as "aspirational" not "enforced"
3. Mark violating files for refactoring
4. Deprecate this pattern

> User answers: 1

✏️ Updated dependable-rust.md to reflect reality
```

## Key Principles

### Facts Over Narratives
- Don't infer relationships, extract them from AST
- Don't guess compliance, measure it
- Don't assume importance, count commits

### Simple Queries Over Complex Code
- SQL everyone understands
- No complex Rust indexer to maintain
- Composable queries instead of monolithic commands

### Git as Source of Truth
- Survival time = quality metric
- Commit frequency = importance metric
- Co-modification = relationship metric

## Why This Works

**Current Patina**: Tries to be smart about text
**This approach**: Extracts facts from structure

**Current**: "Does this file mention pattern X?"
**This**: "Does this code IMPLEMENT pattern X?"

**Current**: Complex tools that we forget we built
**This**: Simple queries over extracted facts

## Implementation Path

1. **Phase 1**: Git-based reality extraction
   - Use git survival metrics
   - Track co-modification patterns
   - Build basic SQLite schema

2. **Phase 2**: Tree-sitter integration
   - Extract AST patterns from Rust code
   - Find structural similarities
   - Identify actual implementations

3. **Phase 3**: GitHub API enhancement
   - Pull in PR discussions
   - Track issue context
   - Use symbol navigation

4. **Phase 4**: Reality reconciliation
   - Compare documented vs actual patterns
   - Interactive reconciliation workflow
   - Update docs to match reality

## The Core Insight

Stop building tools that try to understand meaning. Build tools that extract facts. Let queries reveal meaning.

The semantic system isn't about embeddings or AI - it's about:
- Structured extraction (tree-sitter)
- Relationship tracking (graph in SQL)
- Reality measurement (Git survival)
- Simple queries (SQLite)

This gives us a semantic understanding that's actually grounded in reality, not inference.