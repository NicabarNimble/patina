---
id: llm-code-intelligence-design
status: active
created: 2025-09-02
tags: [architecture, llm, code-intelligence, scrape, ask, fact-collection]
---

# LLM Code Intelligence: Building a Parasitic Code Style Learner

Patina as a parasite that attaches to repositories, enabling LLMs to write native-looking PRs and modules.

---

## Mission Statement

**Core Purpose**: Help LLMs write code that looks like it was written by a senior developer on the existing team, not an outsider.

**The Problem**: LLMs can write syntactically correct code, but it often feels foreign to the codebase:
- Wrong naming conventions
- Different error handling patterns  
- Unfamiliar code organization
- Inconsistent with team idioms

**The Solution**: A fact collection system optimized for teaching LLMs the "personality" of a codebase.

## Current State: Wrong Facts for the Wrong Purpose

### What We're Collecting (Generic Code Analysis)

```sql
-- Current tables and their LLM utility score (0-10)

code_fingerprints (0/10)     -- Pattern hashes mean nothing to LLMs
behavioral_hints (2/10)       -- String matching ".unwrap()" isn't semantic
git_metrics (3/10)           -- Git already has this
function_facts (5/10)        -- Useful but missing context
documentation (6/10)         -- Helpful but not actionable
call_graph (4/10)           -- Too low-level, missing patterns
```

### The Fundamental Mistake

We're trying to be a **semantic analyzer** with **syntactic tools**:
- Counting `.unwrap()` calls instead of learning error handling patterns
- Hashing AST shapes instead of detecting architectural layers
- Building call graphs instead of understanding module boundaries

## What LLMs Actually Need

### 1. Style Intelligence: "How do they write X here?"

```sql
-- Instead of: "function foo has 3 unwraps"
-- LLMs need: "this codebase uses .unwrap() in tests, .expect() in production"

CREATE TABLE style_patterns (
    pattern_type VARCHAR,      -- 'error_handling', 'naming', 'async'
    pattern TEXT,             -- 'Result<T> with ? operator'
    frequency FLOAT,          -- 0.89 (89% of functions follow this)
    examples TEXT[],          -- Real examples from codebase
    violations TEXT[]         -- Counter-examples
);
```

### 2. Structural Intelligence: "Where should I put this?"

```sql
-- Instead of: "file.rs contains function foo"
-- LLMs need: "HTTP handlers go in src/handlers/*.rs"

CREATE TABLE architectural_patterns (
    layer VARCHAR,            -- 'handler', 'service', 'repository'
    typical_location TEXT,    -- 'src/handlers/*.rs'
    imports_from TEXT[],      -- ['service', 'models']
    never_imports TEXT[],     -- ['database', 'repository']
    examples TEXT[]           -- Real files following this pattern
);
```

### 3. Convention Intelligence: "What patterns do they use?"

```sql
-- Instead of: "function has_unsafe = true"
-- LLMs need: "unsafe is only used in src/ffi/* modules"

CREATE TABLE codebase_conventions (
    convention_type VARCHAR,   -- 'testing', 'documentation', 'safety'
    rule TEXT,                -- 'Tests use #[cfg(test)] mod tests'
    confidence FLOAT,         -- 0.95 (95% consistent)
    context TEXT,             -- 'Except for integration tests in tests/'
    examples TEXT[]           -- Concrete examples
);
```

## The New Design: LLM-First Fact Collection

### Phase 1: Pattern Detection (What Tree-Sitter CAN Tell Us)

```rust
// Replace code_fingerprints with pattern_detection
struct PatternFacts {
    // Naming patterns
    function_prefixes: HashMap<String, Vec<String>>,  // "is_" -> ["is_valid", "is_empty"]
    parameter_names: HashMap<String, usize>,         // "ctx" -> 45 occurrences
    type_suffixes: HashMap<String, Vec<String>>,     // "Error" -> ["ParseError", "IoError"]
    
    // Structural patterns
    avg_function_length: usize,
    avg_module_depth: usize,
    test_organization: TestStyle,  // Inline, ModTests, SeparateFiles
    
    // Style patterns
    error_handling: ErrorStyle,    // Result, Option, Panic, Mixed
    async_percentage: f32,
    unsafe_percentage: f32,
    documentation_percentage: f32,
}
```

### Phase 2: Rule Inference (What We Can Learn)

```rust
// New table: codebase_rules
struct CodebaseRule {
    rule_type: RuleType,        // Naming, Structure, Style
    rule: String,               // "Boolean functions start with is_ or has_"
    confidence: f32,            // 0.89 (89% follow this)
    learned_from: Vec<String>,  // Functions that demonstrate this
    exceptions: Vec<String>,    // Functions that violate this
}

// Examples of inferred rules:
"Functions returning Result<T> use ? operator, not match"
"Handlers never directly import database modules"
"Tests are in separate files for public APIs, inline for private"
"Error types implement Display but not Debug"
```

### Phase 3: LLM Prompt Generation (What Ask Should Return)

```rust
// Transform facts into actionable LLM context
struct LLMContext {
    // Style guide
    naming_conventions: Vec<NamingRule>,
    error_handling_patterns: Vec<ErrorPattern>,
    test_patterns: Vec<TestPattern>,
    
    // Architecture guide
    module_structure: ModuleHierarchy,
    layer_boundaries: Vec<LayerRule>,
    dependency_rules: Vec<DependencyRule>,
    
    // Examples
    similar_functions: Vec<Function>,  // "Functions like what you're writing"
    similar_modules: Vec<Module>,      // "Modules organized like yours"
}
```

## The Ask Command: Before and After

### Current (Not Useful for LLMs)

```bash
$ patina ask "how many functions use unwrap?"
> 47 functions use unwrap

$ patina ask "show call graph for process_request"
> process_request -> validate_input -> check_auth -> ...
```

### Desired (LLM-Optimized)

```bash
$ patina ask "how do I add a new API endpoint?"

> Based on analysis of 23 existing endpoints:
>
> STRUCTURE:
> - Handler goes in: src/handlers/{resource}.rs
> - Service goes in: src/services/{resource}_service.rs  
> - Tests go in: tests/api/{resource}_test.rs
>
> PATTERNS:
> - Use #[post("/api/v1/{resource}")] attribute
> - Return Result<HttpResponse, ApiError>
> - Validate input with validate_request() helper
> - Call service layer, never database directly
>
> STYLE:
> - Function name: create_{resource} or update_{resource}
> - Parameter name: 'ctx' for RequestContext
> - Error handling: use ? operator with .context()
> - Log with tracing::info! at entry/exit
>
> SIMILAR EXAMPLES:
> - src/handlers/user.rs::create_user() [Most similar]
> - src/handlers/post.rs::create_post() [Same pattern]
```

```bash
$ patina ask "generate context for PR adding user preferences"

> CONTEXT FOR YOUR PR:
>
> Files that typically change together:
> - src/models/user.rs (add preference fields)
> - src/handlers/user.rs (add update_preferences endpoint)
> - migrations/*.sql (add preferences table)
> - tests/api/user_test.rs (test new endpoint)
>
> This codebase conventions:
> - Preferences are JSON columns, not separate tables (3 examples)
> - Use PATCH /api/v1/users/{id}/preferences for updates
> - Validate preferences with is_valid_preference() helper
> - Cache preferences in Redis with 1hr TTL
>
> Error handling pattern:
> - Return 400 for invalid preferences
> - Return 404 if user doesn't exist
> - Use ApiError::InvalidInput for validation failures
>
> Testing requirements:
> - Unit test the validation logic
> - Integration test the full endpoint
> - Test cache invalidation
```

## Implementation Strategy

### Step 1: Reframe Current Tables

```sql
-- Transform behavioral_hints into pattern_detection
ALTER TABLE behavioral_hints RENAME TO code_patterns;

-- Instead of counting unwraps, detect patterns
-- Old: calls_unwrap = 3
-- New: error_pattern = 'unwrap_in_tests'

-- Reinterpret columns as pattern indicators
-- Old: has_unsafe_block = true  
-- New: safety_context = 'ffi_boundary'
```

### Step 2: Add Pattern Learning

```rust
// New analysis functions in code.rs
fn learn_naming_patterns(ast: &AST) -> NamingPatterns;
fn detect_architectural_layers(files: &[File]) -> Layers;
fn infer_conventions(functions: &[Function]) -> Conventions;
```

### Step 3: Build LLM-Friendly Queries

```sql
-- New views for Ask command
CREATE VIEW llm_style_guide AS
SELECT 
    'error_handling' as category,
    'Use Result<T> with ? operator' as guideline,
    COUNT(*) as examples,
    0.89 as consistency
FROM functions 
WHERE returns_result = true AND uses_question_mark = true;

CREATE VIEW llm_architecture AS
SELECT
    layer,
    typical_imports,
    forbidden_imports,
    example_files
FROM architectural_patterns;
```

## Success Metrics

### Current Metrics (Wrong)
- Number of functions indexed ❌
- Lines of code analyzed ❌
- Complexity scores calculated ❌

### Correct Metrics
- **Pattern Consistency**: Can we detect that 89% follow pattern X?
- **Rule Confidence**: How accurately do we infer conventions?
- **LLM Success Rate**: Do PRs written with our context get accepted?
- **Style Match Score**: Does generated code "feel" native?

## Migration Path

### Phase 1: Keep Everything, Add Pattern Detection (Week 1)
- Keep existing tables
- Add pattern detection alongside
- A/B test current vs pattern-based Ask responses

### Phase 2: Reinterpret Existing Data (Week 2)
- Transform behavioral_hints to pattern indicators
- Convert fingerprints to style signatures
- Reframe call_graph as architectural boundaries

### Phase 3: Delete Waste, Optimize for LLMs (Week 3)
- Remove fingerprints table
- Remove git_metrics (git already has this)
- Consolidate into pattern-focused schema

## The Philosophy Shift

### From: "What is in this code?"
- Count unwraps
- Measure complexity
- Track dependencies

### To: "How do they write code here?"
- Learn unwrap patterns
- Detect complexity preferences
- Understand dependency rules

The goal isn't to analyze code—it's to teach LLMs to be cultural chameleons that blend into any codebase.

## Example: The Unwrap Pattern

### Current (Useless)
```rust
// "This function has 3 unwraps"
calls_unwrap: 3
```

### New (Useful)
```rust
// "This codebase uses unwrap in tests but expect in production"
CodePattern {
    pattern: "error_handling",
    context: "test",
    style: "unwrap_allowed",
    confidence: 0.95,
    examples: ["tests/user_test.rs::test_create", ...],
}

CodePattern {
    pattern: "error_handling",  
    context: "production",
    style: "expect_with_context",
    confidence: 0.87,
    examples: ["src/handlers/user.rs::create", ...],
}
```

## The Bottom Line

**We're not building a code analyzer. We're building a codebase personality learner.**

When an LLM asks "How do I write code for this repo?", we should return a cultural style guide, not a database dump.

The scrape command should be renamed to `learn` because that's what it's actually doing—learning how this team writes code.

---

*"Patina isn't about what the code does. It's about how the code feels."*