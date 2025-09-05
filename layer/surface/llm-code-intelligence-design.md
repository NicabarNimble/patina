---
id: llm-code-intelligence-design
status: active
created: 2025-09-02
updated: 2025-09-05
tags: [architecture, llm, code-intelligence, scrape, ask, fact-collection, pr-validation]
---

# LLM Code Intelligence: Teaching LLMs Your Codebase's Personality

A fact extraction system that enables LLMs to write native-looking code for PRs and modules.

---

## Mission Statement

**Core Purpose**: Extract sufficient facts from a codebase so an LLM can write code that looks like it was written by a senior developer on the existing team, not an outsider.

**The Problem**: LLMs can write syntactically correct code, but it often feels foreign to the codebase:
- Wrong naming conventions (fetch_user vs getUser vs GetUser)
- Different library usage (raw stdlib vs team's preferred framework)
- Unfamiliar code organization (where files go, how they're structured)
- Missing team idioms (how they handle errors, validate input, structure tests)

**The Solution**: Extract two types of facts during `scrape`:
1. **Direct facts**: What exists (functions, types, imports, signatures)
2. **Statistical facts**: What patterns exist (naming frequencies, common structures)

## Current Table Structure

### Tables REMOVED (Not Useful for LLMs) ✅

```sql
code_fingerprints     -- REMOVED: Meaningless hashes
git_metrics          -- REMOVED: Git already has this, redundant  
behavioral_hints     -- REMOVED: Crude string matching, not semantic
```
*These tables were removed in commit a8bf503 (466 lines deleted)*

### Tables to KEEP & ENHANCE (Foundation for Native Code)

```sql
function_facts (8/10)        -- KEEP: Core truth - signatures, parameters, types
type_vocabulary (7/10)       -- KEEP: All types defined in codebase  
import_facts (8/10)          -- KEEP: Shows what libraries/modules are used
documentation (6/10)         -- KEEP: Provides context and examples
```

### Tables to TRANSFORM (Not Yet Implemented)

```sql
call_graph (4/10)           -- EXISTS but needs transformation
                           -- Currently stores: foo() calls bar() at line 42
                           -- Should extract: validate→transform→save patterns
                           -- TODO: Create common_call_sequences table
```

## NEW Pattern Detection Tables (Implemented & Working)

### 1. Style Patterns - How They Write Code ✅

```sql
CREATE TABLE style_patterns (
    pattern_type VARCHAR,      -- 'function_prefix', 'type_suffix', 'parameter_name'
    pattern VARCHAR,          -- 'get_', 'Error', 'ctx'
    frequency INTEGER,        -- How often this pattern occurs
    context VARCHAR          -- Additional context ('functions', 'types', 'parameters')
);

-- Real Example from dust repo:
-- pattern_type: 'function_prefix', pattern: 'get', frequency: 334
-- pattern_type: 'function_prefix', pattern: 'set', frequency: 162
-- This tells LLMs to write getUser() not fetch_user()
```

### 2. Architectural Patterns - Where Code Lives ✅

```sql
CREATE TABLE architectural_patterns (
    layer VARCHAR,            -- 'handlers', 'services', 'contracts', 'systems'
    typical_location VARCHAR, -- '**/handlers/*' or 'src/contracts/*.sol'
    file_count INTEGER,       -- Number of files in this layer
    example_files VARCHAR[]   -- Real files following this pattern
);

-- Note: Adaptive - detects actual structure, not assumed patterns
-- For REST APIs: finds handlers/, services/
-- For blockchain: finds contracts/, systems/
```

### 3. Codebase Conventions - Inferred Team Preferences ✅

```sql
CREATE TABLE codebase_conventions (
    convention_type VARCHAR,   -- 'error_handling', 'testing', 'async'
    rule TEXT,                -- 'Option-preferred' or 'Tests inline'
    confidence FLOAT,         -- 0.0 to 1.0 confidence in this rule
    context VARCHAR          -- Additional explanation
);

-- Inferred from patterns, not hardcoded
-- Examples: "Option-preferred", "Inline modules", "25% async functions"
```

## Technical Foundation

### Parsing Infrastructure
**Most languages use tree-sitter**:
- Rust, Go, Python, JavaScript/TypeScript, C/C++, Solidity
- Tree-sitter provides detailed syntax trees with all tokens
- Syntactic analysis only - no type information or semantic understanding
- Each language grammar varies in structure and completeness

**Cairo uses native parser**:
- Uses `patina_metal::cairo` module with cairo-lang-parser
- Different extraction approach - may provide different insights
- Not constrained by tree-sitter limitations

### Parsing Limitations
Since we rely on syntactic parsing:
- Cannot determine if `Option` refers to `std::Option` or custom type
- Cannot resolve imports or type aliases
- Cannot understand semantic relationships
- Must infer patterns from naming and structure alone

## Current State: What's Broken

### Pattern Detection is Hardcoded (Major Problem)
```rust
// BROKEN: Looking for hardcoded prefixes
let prefixes = ["get_", "set_", "create_", "is_"];  

// SDL example: SDL_CreateWindow
// Looks for: get_? set_? create_? → NO MATCH ❌
// Should find: SDL_Create → MISSED IT

// Result: SDL has 11,421 functions, we find patterns in ~20
```

### Bad Inferences
```sql
-- Current output for C code:
"Option-preferred" -- C doesn't have Option!
"Inline modules"   -- Based on nothing
```

## How Pattern Detection SHOULD Work

### Adaptive Pattern Discovery
```rust
// Instead of hardcoded prefixes, DISCOVER actual patterns:
fn extract_pattern(name: &str) -> Option<String> {
    // SDL_CreateWindow → "SDL_Create"
    // gtk_widget_show → "gtk_widget_"
    // get_user → "get_"
    // Count what we SEE, not what we EXPECT
}
```

## Multiple Sources of Truth (Evidence-Based Patterns)

### 1. Code Age & Stability
```sql
-- Stable patterns (unchanged for years)
SDL_Init → SDL_CreateWindow → SDL_Quit  -- 10 years unchanged
main() structure                         -- 15 years stable
-- These are proven patterns that work
```

### 2. Usage Frequency  
```sql
-- Common patterns (used frequently)
SDL_CreateWindow: called 47 times
NULL check pattern: 312 instances
SDL_GetError after NULL: 89% of error checks
```

### 3. PR Validation
```sql
-- Recent PRs show what currently works
PR #9961: Fixed PSP renderer crash
  - Shows current error handling style
  - Validates SDL_Destroy patterns
  - Confirms modern logging approach
```

### 4. Bug Fix Patterns
```sql
-- Learn from mistakes
15 PRs fixed missing SDL_DestroyWindow
8 PRs fixed unchecked NULL returns
-- These teach us what NOT to do
```

## Unified Pattern Scoring

```sql
CREATE TABLE validated_patterns (
    pattern VARCHAR,
    
    -- Multiple confidence signals
    stability_score FLOAT,       -- Years unchanged (10yr = 1.0)
    usage_count INT,             -- Times used in codebase
    pr_validations INT,          -- Recent PRs following this
    fix_count INT,              -- Times it needed fixing (bad!)
    
    -- Evidence & examples
    example_code TEXT,          -- Actual working code
    evidence TEXT,              -- "10yr stable, 47 uses, 3 recent PRs"
    confidence FLOAT            -- Weighted score (0.0-1.0)
);
```

### Scoring Formula
```
confidence = stability * 0.4    -- Old stable code is reliable
           + usage * 0.3         -- Common patterns matter
           + pr_validation * 0.2 -- Recent validation
           + (1 - fix_rate) * 0.1 -- Avoid buggy patterns
```

## The Ask Command: Using Pattern Facts

**Note**: The `ask` command is a separate command outside of `code.rs` in the `scrape` module. It will query the DuckDB database created by `scrape` to provide evidence-based answers about codebase patterns.

### Example 1: Writing a New Function (dust repo)

```bash
$ patina ask "how do I write a getter function for player stats?"

Query executed:
SELECT pattern, frequency FROM style_patterns 
WHERE pattern_type = 'function_prefix' 
AND pattern IN ('get', 'fetch', 'load')

> Based on 334 getter functions in this codebase:
>
> NAMING: getPlayerStats() - use camelCase with 'get' prefix
> - 'get' used 334 times (preferred)
> - 'fetch' used 0 times (not used here)
>
> COMMON PATTERNS:
> - getAppConfig()
> - getEntityProgram()  
> - getTotalMassEnergy()
>
> IMPORTS YOU'LL NEED:
> - import { StoreSwitch } from "@latticexyz/store/src/StoreSwitch.sol"
> - import { ResourceId } from "@latticexyz/store/src/ResourceId.sol"
```

### Example 2: Understanding Codebase Style

```bash
$ patina ask "what's the testing pattern here?"

Query executed:
SELECT * FROM codebase_conventions WHERE convention_type = 'testing'
SELECT COUNT(*) FROM function_facts WHERE name LIKE 'test%'

> Testing Patterns:
> - 416 test functions found (all start with 'test')
> - Organization: Inline modules (85% confidence)
> - Naming: test* prefix is mandatory
> - No separate test/ directory pattern detected
```

## What Makes This Work for LLMs

### 1. Facts Over Interpretation
- ❌ "This codebase prefers Option" (interpretation)
- ✅ "get_ functions return Option 9/10 times" (statistical fact)
- ✅ "10 functions return Option, 3 return Result" (direct fact)

### 2. Patterns Over Individual Instances
- ❌ "Function foo() calls bar()" (too specific)
- ✅ "'get' prefix used 334 times" (pattern)
- ✅ "handlers never import from database" (rule)

### 3. Complete Context in Database
The DuckDB database contains everything needed to write native-like code:
- Function signatures (how to call things)
- Import patterns (what libraries to use)
- Naming conventions (how to name things)
- Architectural patterns (where to put things)

No need to access source code - the facts are sufficient.

## Success Metrics

### What We Measure Now (Working)
- **Pattern Detection**: 39 patterns detected in Patina, 147 in dust repo ✅
- **Convention Inference**: Successfully inferring "Option-preferred", "Inline modules" ✅
- **Naming Consistency**: Detecting get/set/delete patterns with frequencies ✅

### What Success Looks Like
- **For PRs**: Generated code uses same naming conventions, imports, and patterns
- **For Modules**: New modules follow existing architectural patterns
- **For LLMs**: Can answer "how do I write X?" with concrete examples and patterns

## Current Reality vs Vision

### What Works Now
- ✅ Extracting function signatures, types, imports
- ✅ Building call graphs
- ✅ Detecting some patterns (though hardcoded)

### What's Broken
- 🔴 Pattern detection uses hardcoded prefixes (misses SDL_*, gtk_*, etc.)
- 🔴 Convention inference makes false claims (Option for C code)
- 🔴 No PR validation implemented
- 🔴 Ask command not yet implemented (planned as separate command)

### What's Missing  
- ❌ Usage examples (actual code snippets)
- ❌ Error handling patterns
- ❌ Lifecycle patterns (Create/Destroy pairs)
- ❌ Evidence tracking for patterns

### Next Critical Steps
1. Fix adaptive pattern detection (stop hardcoding)
2. Add PR pattern extraction for validation
3. Implement ask command as separate module (src/commands/ask.rs)
4. Add evidence requirements (no claims without proof)

## The Philosophy

**Core Insight**: We teach LLMs to write code that will pass PRs and CI by learning from what actually works.

Three levels of knowledge:
1. **Facts**: What exists (functions, types, signatures)
2. **Patterns**: How it's organized (naming, structure, sequences)  
3. **Intelligence**: What works (validated by age, usage, PRs)

When an LLM asks "How do I write code for this repo?", we return:
- Proven patterns with evidence ("47 files do this")
- Actual code examples from successful PRs
- Confidence scores based on multiple signals
- Never make claims we can't prove

---

*"Don't tell LLMs how you think code should be written. Show them what actually gets merged."*

## Summary for LLMs

This document describes the LLM Code Intelligence system in Patina. The goal is to help LLMs write native-like code by extracting facts about how a codebase is written (not what it does).

### Key Points:
1. **Delete**: code_fingerprints, git_metrics, behavioral_hints (not useful)
2. **Keep**: function_facts, type_vocabulary, import_facts (core facts)
3. **New**: style_patterns, architectural_patterns, codebase_conventions (pattern detection)

### Implementation Status:
- ✅ AST extraction (functions, types, imports)
- ✅ Call graph tracking  
- 🔴 Pattern detection (hardcoded, not adaptive)
- 🔴 Convention inference (makes bad guesses)
- ❌ PR pattern validation (not implemented)
- ❌ Ask command integration (not connected)

### Design Principle:
Only store facts you can prove. Validate patterns with evidence. Let patterns emerge from data, not assumptions.

## Implementation Roadmap

### Phase 1: Fix Pattern Detection (Critical)
```rust
// In code.rs, replace hardcoded prefix detection
fn extract_pattern(name: &str, language: Language) -> Option<String> {
    // Adaptive detection based on actual patterns
    // Not looking for "get_" but finding "SDL_Get", "gtk_widget_", etc.
}
```

### Phase 2: Add PR Validation (High Value)
```rust
fn validate_with_prs(repo: &Path) -> ValidationData {
    // Extract patterns from merged PRs
    // Learn from bug fixes
    // Weight patterns by success
}
```

### Phase 3: Implement Ask Command (User Value)
```rust
// New separate command: src/commands/ask.rs
// Queries the DuckDB database created by scrape command
fn handle_ask(query: &str, db_path: &str) -> Response {
    // Query validated_patterns table
    // Return evidence-based examples
    // Include confidence scores
}
```

### Phase 4: Evidence-Based Scoring (Quality)
- Combine AST frequency + Git stability + PR validation
- Show evidence for each pattern claim
- Never claim patterns without proof

## Success Criteria

1. **SDL Test**: Detect "SDL_Create" pattern (currently misses it)
2. **No False Claims**: Stop claiming "Option-preferred" for C code  
3. **PR Validation**: Recent PR patterns weighted higher than old code
4. **Evidence Trail**: Every pattern has "based on X examples" proof