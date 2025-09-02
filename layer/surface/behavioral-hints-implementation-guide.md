# Complete Implementation Guide: Behavioral Hints for Multiple Programming Languages

## Overview
Behavioral hints are code pattern detectors that identify potentially problematic or interesting patterns in functions. They help identify risk areas, technical debt, and complexity indicators. Currently only implemented for Rust, this guide will extend the feature to C/C++, Python, Go, TypeScript/JavaScript, and other supported languages.

## Current State Assessment

### Existing Implementation Location
- **File**: `src/commands/scrape/code.rs`
- **Function**: `extract_behavioral_hints` (lines ~3195-3251)
- **Current Languages**: Rust only
- **Invocation**: Line ~3000: `if language == Language::Rust { extract_behavioral_hints(...) }`

### Database Schema
```sql
TABLE behavioral_hints (
    file VARCHAR NOT NULL,
    function VARCHAR NOT NULL,
    calls_unwrap INTEGER DEFAULT 0,
    calls_expect INTEGER DEFAULT 0,
    has_panic_macro BOOLEAN,
    has_todo_macro BOOLEAN,
    has_unsafe_block BOOLEAN,
    has_mutex BOOLEAN,
    has_arc BOOLEAN,
    PRIMARY KEY (file, function)
)
```

### Current Rust-Specific Patterns
1. `.unwrap()` - Panic risk counter
2. `.expect(` - Panic with message counter  
3. `panic!` - Explicit panic detection
4. `todo!` - Incomplete code marker
5. `unsafe {` - Unsafe block detection
6. `Mutex` - Concurrency primitive
7. `Arc<` or `Arc::` - Reference counting

## Step-by-Step Implementation Guide

### Step 1: Analyze Current Code Structure

First, read the existing implementation to understand the pattern:

```bash
# Read the current implementation
grep -n "extract_behavioral_hints" src/commands/scrape/code.rs
# Should show function definition around line 3195 and invocation around line 3000
```

Key observations:
- Function takes: node, source, file_path, function_name, sql
- Extracts function body text
- Uses simple string matching (`.contains()`, `.matches().count()`)
- Only inserts if hints found
- Escapes SQL strings

### Step 2: Plan Language-Specific Patterns

Create a comprehensive list of patterns per language:

#### C/C++ Patterns
```rust
// Memory/Safety
let uses_malloc = body_text.contains("malloc(") || body_text.contains("calloc(");
let uses_free = body_text.contains("free(");
let malloc_count = body_text.matches("malloc(").count() + body_text.matches("calloc(").count();
let free_count = body_text.matches("free(").count();
let potential_leak = malloc_count > free_count;

// Dangerous functions
let uses_gets = body_text.contains("gets(");
let uses_strcpy = body_text.contains("strcpy(");
let uses_sprintf = body_text.contains("sprintf(");

// Control flow
let uses_goto = body_text.contains("goto ");
let uses_longjmp = body_text.contains("longjmp(");

// C++ specific
let uses_new = body_text.contains("new ");
let uses_delete = body_text.contains("delete ");
let uses_reinterpret_cast = body_text.contains("reinterpret_cast<");

// Common patterns
let has_todo = body_text.contains("TODO") || body_text.contains("FIXME");
let has_fixme = body_text.contains("FIXME");
```

#### Python Patterns
```rust
// Exception handling
let bare_except = body_text.contains("except:") || body_text.contains("except Exception:");
let passes_in_except = body_text.contains("except") && body_text.contains("pass");

// Dangerous functions
let uses_eval = body_text.contains("eval(");
let uses_exec = body_text.contains("exec(");
let uses_compile = body_text.contains("compile(");

// Code quality
let uses_global = body_text.contains("global ");
let has_type_ignore = body_text.contains("# type: ignore");
let has_noqa = body_text.contains("# noqa");

// Common patterns
let has_todo = body_text.contains("TODO") || body_text.contains("FIXME");
let prints_found = body_text.matches("print(").count();
```

#### Go Patterns
```rust
// Error handling
let ignored_errors = body_text.matches("_ =").count() + body_text.matches(", _ :=").count();
let panic_calls = body_text.matches("panic(").count();

// Concurrency
let goroutines_started = body_text.matches("go func").count() + body_text.matches("go ").count();
let has_mutex = body_text.contains("sync.Mutex") || body_text.contains("sync.RWMutex");
let has_waitgroup = body_text.contains("sync.WaitGroup");

// Common patterns
let has_todo = body_text.contains("TODO") || body_text.contains("FIXME");
let uses_unsafe = body_text.contains("unsafe.");
```

#### TypeScript/JavaScript Patterns
```rust
// Type safety (TypeScript)
let uses_any = body_text.matches(": any").count() + body_text.matches("<any>").count();
let uses_ts_ignore = body_text.contains("@ts-ignore");
let uses_ts_nocheck = body_text.contains("@ts-nocheck");

// Dangerous patterns
let uses_eval = body_text.contains("eval(");
let uses_function_constructor = body_text.contains("new Function(");

// Debugging
let console_logs = body_text.matches("console.log(").count();
let console_errors = body_text.matches("console.error(").count();
let debugger_statements = body_text.contains("debugger");

// Promises (basic detection)
let has_promise = body_text.contains("Promise") || body_text.contains(".then(");
let has_catch = body_text.contains(".catch(");
let potential_unhandled = has_promise && !has_catch;

// Common patterns
let has_todo = body_text.contains("TODO") || body_text.contains("FIXME");
```

### Step 3: Update Database Schema

The current schema is Rust-specific. We need a more generic schema:

```sql
-- Option 1: Generic columns
CREATE TABLE behavioral_hints_v2 (
    file VARCHAR NOT NULL,
    function VARCHAR NOT NULL,
    language VARCHAR NOT NULL,
    
    -- Risk indicators (all languages)
    dangerous_operations INTEGER DEFAULT 0,  -- malloc without free, eval, unsafe, etc.
    error_suppression INTEGER DEFAULT 0,     -- unwrap, bare except, ignored errors
    
    -- Code quality
    todo_markers INTEGER DEFAULT 0,          -- TODO, FIXME, todo!
    debug_code INTEGER DEFAULT 0,            -- console.log, print, debugger
    
    -- Language-specific counts stored as JSON
    specific_patterns VARCHAR,               -- JSON: {"uses_eval": 1, "bare_except": 2}
    
    PRIMARY KEY (file, function)
);

-- Option 2: Keep current schema and add language-specific tables
CREATE TABLE behavioral_hints_python (
    file VARCHAR NOT NULL,
    function VARCHAR NOT NULL,
    bare_except BOOLEAN,
    uses_eval BOOLEAN,
    uses_exec BOOLEAN,
    uses_global BOOLEAN,
    type_ignores INTEGER DEFAULT 0,
    print_statements INTEGER DEFAULT 0,
    PRIMARY KEY (file, function)
);
-- Similar tables for other languages...
```

### Step 4: Modify Code Structure

#### 4.1: Create language-specific extraction functions

Add new functions after the current `extract_behavioral_hints`:

```rust
/// Extract behavioral hints for C/C++
fn extract_behavioral_hints_c_cpp(
    node: tree_sitter::Node,
    source: &[u8],
    file_path: &str,
    function_name: &str,
    sql: &mut String,
) {
    if let Some(body) = node.child_by_field_name("body") {
        let body_text = body.utf8_text(source).unwrap_or("");
        
        // Pattern detection (from Step 2)
        let malloc_count = body_text.matches("malloc(").count() + body_text.matches("calloc(").count();
        let free_count = body_text.matches("free(").count();
        // ... more patterns ...
        
        // Only insert if patterns found
        if malloc_count > 0 || uses_gets || uses_goto || has_todo {
            sql.push_str(&format!(
                "INSERT OR REPLACE INTO behavioral_hints_c (file, function, malloc_calls, free_calls, uses_gets, uses_strcpy, uses_goto, has_todo) VALUES ('{}', '{}', {}, {}, {}, {}, {}, {});\n",
                escape_sql(file_path),
                escape_sql(function_name),
                malloc_count,
                free_count,
                uses_gets,
                uses_strcpy,
                uses_goto,
                has_todo
            ));
        }
    }
}

/// Extract behavioral hints for Python
fn extract_behavioral_hints_python(
    node: tree_sitter::Node,
    source: &[u8],
    file_path: &str,
    function_name: &str,
    sql: &mut String,
) {
    // Similar structure...
}

// Add functions for Go, TypeScript, JavaScript
```

#### 4.2: Create a dispatcher function

Replace the single language check with a dispatcher:

```rust
/// Dispatch to appropriate behavioral hint extractor
fn extract_behavioral_hints_for_language(
    node: tree_sitter::Node,
    source: &[u8],
    file_path: &str,
    function_name: &str,
    sql: &mut String,
    language: languages::Language,
) {
    use languages::Language;
    
    match language {
        Language::Rust => extract_behavioral_hints(node, source, file_path, function_name, sql),
        Language::C | Language::Cpp => extract_behavioral_hints_c_cpp(node, source, file_path, function_name, sql),
        Language::Python => extract_behavioral_hints_python(node, source, file_path, function_name, sql),
        Language::Go => extract_behavioral_hints_go(node, source, file_path, function_name, sql),
        Language::TypeScript | Language::TypeScriptTSX => extract_behavioral_hints_typescript(node, source, file_path, function_name, sql),
        Language::JavaScript | Language::JavaScriptJSX => extract_behavioral_hints_javascript(node, source, file_path, function_name, sql),
        _ => {} // No behavioral hints for other languages yet
    }
}
```

### Step 5: Update Invocation Points

#### 5.1: Find all invocation points

```bash
grep -n "extract_behavioral_hints" src/commands/scrape/code.rs
```

Currently at line ~3000:
```rust
// Replace this:
if language == Language::Rust {
    extract_behavioral_hints(node, source, file_path, name, sql);
}

// With this:
extract_behavioral_hints_for_language(node, source, file_path, name, sql, language);
```

#### 5.2: Update C/C++ iterative processor

The C/C++ iterative processor (around line 2260) also needs to call behavioral hints:

```rust
// In process_c_cpp_iterative function, after extracting function facts:
if kind == "function" {
    context.current_function = Some(name.clone());
    
    // Extract function facts for C/C++
    extract_function_facts(node, source, file_path, &name, sql, language);
    
    // Add behavioral hints extraction
    extract_behavioral_hints_for_language(node, source, file_path, &name, sql, language);
    
    // ... rest of code
}
```

### Step 6: Update Database Initialization

Find the database initialization code:

```bash
grep -n "CREATE TABLE behavioral_hints" src/
```

Update the schema in the initialization function to include new tables or modify existing schema.

### Step 7: Testing Strategy

#### 7.1: Create test files

Create test files for each language with known patterns:

```python
# test_python.py
def risky_function():
    try:
        eval(user_input)  # Dangerous
    except:  # Bare except
        pass
    
    global some_var  # Global usage
    # TODO: Fix this later
    print("Debug output")
```

```c
// test_c.c
void memory_leak() {
    char* buffer = malloc(100);  // Malloc without free
    strcpy(buffer, input);       // Dangerous function
    // TODO: Add bounds checking
    
    if (error) goto cleanup;     // Goto usage
cleanup:
    return;  // Missing free!
}
```

#### 7.2: Test process

1. Initialize test database
2. Run scrape on test files
3. Query behavioral hints tables
4. Verify expected patterns detected

### Step 8: Build and Test Commands

```bash
# 1. Make changes to the code
# Edit src/commands/scrape/code.rs as described above

# 2. Build the project
cargo build

# 3. Create test directory with sample files
mkdir -p /tmp/behavior_test
# Add test files from Step 7.1

# 4. Initialize database
cd /tmp/behavior_test
cargo run --manifest-path=/path/to/patina/Cargo.toml -- scrape code --init

# 5. Run scrape
cargo run --manifest-path=/path/to/patina/Cargo.toml -- scrape code

# 6. Check results
duckdb .patina/knowledge.db -c "SELECT * FROM behavioral_hints"
duckdb .patina/knowledge.db -c "SELECT * FROM behavioral_hints_c"  # If using separate tables
```

### Step 9: Edge Cases and Considerations

#### 9.1: Performance Impact
- Behavioral hints add overhead to parsing
- Consider adding a flag: `--with-behavioral-hints`
- Or make it configurable in settings

#### 9.2: False Positives
- String matching can catch patterns in comments
- Example: `// Don't use malloc here` would trigger malloc detection
- Solution: Use AST-based detection for more accuracy (more complex)

#### 9.3: Language Variations
- Python 2 vs 3 differences
- C vs C++ specific patterns
- TypeScript vs JavaScript distinctions

#### 9.4: Large Functions
- Very large function bodies might have performance issues
- Consider adding a size limit or sampling strategy

### Step 10: Documentation and Commit

#### 10.1: Update documentation

Add comments explaining new patterns:

```rust
/// Extract behavioral hints for C/C++ code
/// 
/// Detects patterns including:
/// - Memory management issues (malloc without free)
/// - Dangerous functions (gets, strcpy, sprintf)
/// - Control flow complexity (goto, longjmp)
/// - TODO/FIXME markers
```

#### 10.2: Commit message template

```
feat: add behavioral hints for C/C++, Python, Go, and JS/TS

- Extended behavioral hints beyond Rust to all major languages
- Added language-specific pattern detection:
  * C/C++: Memory leaks, buffer overflows, unsafe functions
  * Python: Bare excepts, eval usage, global mutations
  * Go: Ignored errors, panic calls, goroutine patterns
  * JS/TS: Type safety bypasses, console logs, eval usage
- Updated database schema to accommodate language-specific patterns
- Integrated with both recursive and iterative AST processors

Behavioral hints help identify code quality issues, security risks,
and technical debt across all supported languages.
```

## Complete File Locations Reference

Files that need modification:
1. `src/commands/scrape/code.rs` - Main implementation
2. Database initialization (search for where tables are created)
3. Possibly `src/commands/incremental.rs` if it references behavioral_hints

## Validation Checklist

- [ ] All languages have hint extraction functions
- [ ] Database schema supports new patterns
- [ ] Both recursive and iterative processors call hint extraction
- [ ] Test files demonstrate pattern detection
- [ ] No compilation errors
- [ ] Hints appear in database after scraping
- [ ] Performance impact is acceptable
- [ ] Documentation is updated

## Rollback Plan

If issues arise:
1. Keep original `extract_behavioral_hints` function
2. Use feature flag to enable/disable new hints
3. Can revert to Rust-only with single line change

## Implementation Time Estimate

- **Step 1-2**: Analysis and planning - 30 minutes
- **Step 3**: Database schema design - 30 minutes
- **Step 4**: Code implementation - 2-3 hours
- **Step 5**: Integration points - 30 minutes
- **Step 6**: Database updates - 30 minutes
- **Step 7-8**: Testing - 1 hour
- **Step 9-10**: Edge cases and documentation - 30 minutes

**Total: 5-6 hours** for complete implementation across all languages

## Why This Matters

Behavioral hints provide:
1. **Risk Assessment**: Identify dangerous patterns before they cause issues
2. **Technical Debt Tracking**: Find TODOs, FIXMEs, and incomplete code
3. **Code Quality Metrics**: Measure error handling, type safety, and best practices
4. **Security Scanning**: Detect eval, SQL injection risks, buffer overflows
5. **Cross-Language Insights**: Compare code quality across different parts of the codebase

This guide provides everything needed to implement behavioral hints across all languages, with zero assumed context about the codebase structure or patterns.