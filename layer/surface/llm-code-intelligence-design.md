---
id: llm-code-intelligence-design
status: active
created: 2025-09-02
updated: 2025-09-03
tags: [architecture, llm, code-intelligence, scrape, ask, fact-collection]
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

## What We Keep vs What We Delete

### Tables to DELETE (Not Useful for LLMs)

```sql
code_fingerprints (0/10)     -- DELETE: Meaningless hashes (pattern: 3847293)
git_metrics (0/10)           -- DELETE: Git already has this, redundant
behavioral_hints (2/10)      -- DELETE: Crude string matching, not semantic
```

### Tables to KEEP & ENHANCE (Foundation for Native Code)

```sql
function_facts (8/10)        -- KEEP: Core truth - signatures, parameters, types
type_vocabulary (7/10)       -- KEEP: All types defined in codebase  
import_facts (8/10)          -- KEEP: Shows what libraries/modules are used
documentation (6/10)         -- KEEP: Provides context and examples
```

### Tables to TRANSFORM (Good Idea, Wrong Implementation)

```sql
call_graph (4/10)           -- TRANSFORM: Into common_call_sequences
                           -- Instead of: foo() calls bar()
                           -- Better: validate→transform→save pattern
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

## How Pattern Detection Works

### During Scrape (Real-time Pattern Detection)

```rust
// As we traverse the AST, we accumulate patterns:
for each file {
    detect_naming_patterns()     // Tracks get_, set_, is_ prefixes
    detect_architectural_layer() // Identifies handlers/, services/, etc.
    track_function_stats()       // Counts for convention inference
}

// After all files, infer conventions:
infer_conventions() {
    if get_* > fetch_* → "getter-style"
    if tests in separate files → "separate test files"
    if 25% async functions → "async-heavy codebase"
}
```

### What Gets Stored

```sql
-- Direct Facts (function_facts table):
getUser() returns Option<User>
deleteEntity() takes (Vec3, uint128)
StoreSwitch.sol is imported 74 times

-- Statistical Facts (style_patterns table):
'get' prefix used 334 times
'set' prefix used 162 times
'ctx' parameter name used 30 times

-- Inferred Conventions (codebase_conventions table):
"Option-preferred" with 75% confidence
"Inline modules" with 85% confidence
```

## The Ask Command: Using Pattern Facts

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

## Limitations & Future Work

### Current Limitations
1. **No function bodies** - We know signatures but not implementations
2. **No error patterns** - Can't detect try/catch vs Result patterns in detail
3. **Language-specific gaps** - Solidity modifiers, Python decorators not captured

### Future Enhancements
1. **Code snippets** - Store key implementation patterns
2. **Common sequences** - Transform call_graph into pattern sequences
3. **Cross-file patterns** - Which files typically change together

## The Philosophy

**Core Insight**: We're not building a code analyzer. We're building a codebase personality learner.

When an LLM asks "How do I write code for this repo?", we return:
- Statistical facts about naming and structure
- Direct facts about what exists
- Inferred conventions with confidence scores

The scrape command learns how this team writes code. The ask command teaches that to LLMs.

---

*"Patina isn't about what the code does. It's about how the code feels."*

## Summary for LLMs

This document describes the LLM Code Intelligence system in Patina. The goal is to help LLMs write native-like code by extracting facts about how a codebase is written (not what it does).

### Key Points:
1. **Delete**: code_fingerprints, git_metrics, behavioral_hints (not useful)
2. **Keep**: function_facts, type_vocabulary, import_facts (core facts)
3. **New**: style_patterns, architectural_patterns, codebase_conventions (pattern detection)

### Implementation Status:
- ✅ Pattern detection during AST traversal
- ✅ Statistical fact accumulation
- ✅ Convention inference
- ❌ Ask command integration (future work)

### Design Principle:
Extract facts, not interpretations. Let the ask command interpret facts based on the question being asked.