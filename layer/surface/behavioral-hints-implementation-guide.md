# Behavioral Hints Multi-Language Implementation Plan

## Current State

### Implementation Facts
- **Location**: `src/commands/scrape/code.rs`
- **Function**: `extract_behavioral_hints` (lines 3196-3251)
- **Invocation**: Line 3058-3060 - Only called for Rust
- **Method**: Simple string matching on function body text
- **Table Creation**: Line 3527-3538 in schema string
- **C/C++ Processor**: Line 2260 calls `extract_function_facts` but NOT behavioral hints

### Current Database Schema
```sql
CREATE TABLE IF NOT EXISTS behavioral_hints (
    file VARCHAR NOT NULL,
    function VARCHAR NOT NULL,
    calls_unwrap INTEGER DEFAULT 0,     -- Rust: .unwrap() calls
    calls_expect INTEGER DEFAULT 0,     -- Rust: .expect() calls
    has_panic_macro BOOLEAN,           -- Rust: panic!()
    has_todo_macro BOOLEAN,            -- Rust: todo!()
    has_unsafe_block BOOLEAN,          -- Rust: unsafe {}
    has_mutex BOOLEAN,                 -- Rust: Mutex usage
    has_arc BOOLEAN,                   -- Rust: Arc usage
    PRIMARY KEY (file, function)
);
```

## Implementation Strategy

### Approach: Reinterpret Existing Columns
Following the `function_facts` pattern where columns are reinterpreted per language:
- **One table** for all languages (behavioral_hints)
- **Reuse existing columns** with language-specific meanings
- **Add minimal new columns** for cross-language patterns
- **Maintain backward compatibility**

### Column Reinterpretation Map

| Column | Rust | C/C++ | Python | Go | JS/TS |
|--------|------|-------|--------|-----|--------|
| calls_unwrap | .unwrap() count | unchecked mallocs | bare except count | ignored errors | unhandled promises |
| calls_expect | .expect() count | assert() count | pass in except | panic() count | console.error count |
| has_panic_macro | panic!() | abort/exit | sys.exit() | panic() | throw statements |
| has_todo_macro | todo!() | TODO/FIXME | TODO/FIXME | TODO/FIXME | TODO/FIXME |
| has_unsafe_block | unsafe {} | uses strcpy/gets | uses eval/exec | uses unsafe | uses eval |
| has_mutex | Mutex | pthread_mutex | threading.Lock | sync.Mutex | N/A |
| has_arc | Arc<> | shared_ptr | N/A | N/A | N/A |

## Step-by-Step Implementation

### Step 1: Create Language-Specific Extraction Functions
**Location**: After line 3251 in `src/commands/scrape/code.rs`

Add new functions that follow the reinterpretation strategy:

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
        
        // Reinterpret columns for C/C++
        let calls_unwrap = body_text.matches("malloc(").count() + 
                          body_text.matches("calloc(").count() - 
                          body_text.matches("free(").count(); // Unchecked allocations
        let calls_expect = body_text.matches("assert(").count();
        let has_panic_macro = body_text.contains("abort()") || body_text.contains("exit(");
        let has_todo_macro = body_text.contains("TODO") || body_text.contains("FIXME");
        let has_unsafe_block = body_text.contains("strcpy(") || 
                               body_text.contains("gets(") || 
                               body_text.contains("sprintf(");
        let has_mutex = body_text.contains("pthread_mutex");
        let has_arc = body_text.contains("shared_ptr");
        
        // Only insert if patterns found
        if calls_unwrap > 0 || calls_expect > 0 || has_panic_macro || 
           has_todo_macro || has_unsafe_block || has_mutex || has_arc {
            sql.push_str(&format!(
                "INSERT OR REPLACE INTO behavioral_hints (file, function, calls_unwrap, calls_expect, has_panic_macro, has_todo_macro, has_unsafe_block, has_mutex, has_arc) VALUES ('{}', '{}', {}, {}, {}, {}, {}, {}, {});\n",
                escape_sql(file_path),
                escape_sql(function_name),
                calls_unwrap.max(0),  // Ensure non-negative
                calls_expect,
                has_panic_macro,
                has_todo_macro,
                has_unsafe_block,
                has_mutex,
                has_arc
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
    if let Some(body) = node.child_by_field_name("body") {
        let body_text = body.utf8_text(source).unwrap_or("");
        
        // Reinterpret columns for Python
        let calls_unwrap = body_text.matches("except:").count() + 
                          body_text.matches("except Exception:").count();
        let calls_expect = body_text.matches("except").count() * 
                          body_text.matches("pass").count(); // Pass in except blocks
        let has_panic_macro = body_text.contains("sys.exit(") || body_text.contains("os._exit(");
        let has_todo_macro = body_text.contains("TODO") || body_text.contains("FIXME");
        let has_unsafe_block = body_text.contains("eval(") || 
                               body_text.contains("exec(") || 
                               body_text.contains("__import__(");
        let has_mutex = body_text.contains("threading.Lock") || body_text.contains("threading.RLock");
        let has_arc = false; // No direct equivalent
        
        // Insert if patterns found...
    }
}

/// Extract behavioral hints for Go
fn extract_behavioral_hints_go(
    node: tree_sitter::Node,
    source: &[u8],
    file_path: &str,
    function_name: &str,
    sql: &mut String,
) {
    // Similar pattern...
    // calls_unwrap = ignored errors (_ = or , _ :=)
    // calls_expect = panic() calls
    // has_mutex = sync.Mutex or sync.RWMutex
}

/// Extract behavioral hints for JavaScript/TypeScript
fn extract_behavioral_hints_javascript(
    node: tree_sitter::Node,
    source: &[u8],
    file_path: &str,
    function_name: &str,
    sql: &mut String,
) {
    // Similar pattern...
    // calls_unwrap = .then( without .catch(
    // calls_expect = console.error count
    // has_unsafe_block = eval() or new Function()
}
```

### Step 2: Create Dispatcher Function
**Location**: After the new extraction functions (around line 3300)

```rust
/// Dispatch to appropriate behavioral hint extractor based on language
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
        Language::TypeScript | Language::TypeScriptTSX | 
        Language::JavaScript | Language::JavaScriptJSX => {
            extract_behavioral_hints_javascript(node, source, file_path, function_name, sql)
        },
        _ => {} // No behavioral hints for other languages yet
    }
}
```

### Step 3: Update Invocation Points

#### 3.1: Replace Rust-only check (Line 3058-3060)
```rust
// OLD:
if language == Language::Rust {
    extract_behavioral_hints(node, source, file_path, name, sql);
}

// NEW:
extract_behavioral_hints_for_language(node, source, file_path, name, sql, language);
```

#### 3.2: Add to C/C++ iterative processor (After line 2260)
```rust
// In process_c_cpp_iterative function, after extract_function_facts:
if kind == "function" {
    context.current_function = Some(name.clone());
    
    // Extract function facts for C/C++
    extract_function_facts(node, source, file_path, &name, sql, language);
    
    // ADD THIS LINE:
    extract_behavioral_hints_for_language(node, source, file_path, &name, sql, language);
    
    // ... rest of existing code
}
```

### Step 4: Update Database Schema Comments
**Location**: Line 3527-3538

Update the comments to reflect multi-language use:

```sql
CREATE TABLE IF NOT EXISTS behavioral_hints (
    file VARCHAR NOT NULL,
    function VARCHAR NOT NULL,
    calls_unwrap INTEGER DEFAULT 0,     -- Error suppression: unwrap/unchecked malloc/bare except/ignored errors
    calls_expect INTEGER DEFAULT 0,     -- Assertions: expect/assert/panic calls
    has_panic_macro BOOLEAN,           -- Explicit exit: panic/abort/exit/sys.exit
    has_todo_macro BOOLEAN,            -- TODO/FIXME markers (all languages)
    has_unsafe_block BOOLEAN,          -- Dangerous ops: unsafe/strcpy/eval
    has_mutex BOOLEAN,                 -- Concurrency: Mutex/pthread_mutex/threading.Lock
    has_arc BOOLEAN,                   -- Shared ownership: Arc/shared_ptr (C++/Rust only)
    PRIMARY KEY (file, function)
);
```

### Step 5: Testing

#### 5.1: Create test files
```bash
mkdir -p test_behavioral_hints
cd test_behavioral_hints
```

Create `test.c`:
```c
void risky_function() {
    char* buffer = malloc(100);
    strcpy(buffer, input);  // Dangerous
    // TODO: Fix memory leak
}
```

Create `test.py`:
```python
def risky_function():
    try:
        eval(user_input)
    except:  # Bare except
        pass
    # TODO: Remove eval
```

Create `test.go`:
```go
func riskyFunction() {
    data, _ := ReadFile()  // Ignored error
    panic("not implemented")
    // TODO: Handle error properly
}
```

#### 5.2: Test scraping
```bash
# Initialize and scrape
patina scrape code --init
patina scrape code

# Query results
duckdb .patina/knowledge.db -c "SELECT * FROM behavioral_hints"
```

### Step 6: Run CI Checks
```bash
cargo fmt --all
cargo clippy --workspace
cargo test --workspace
```

## Files to Modify

1. **`src/commands/scrape/code.rs`**:
   - Add language-specific extraction functions (after line 3251)
   - Add dispatcher function (after new functions)
   - Update invocation at line 3058-3060
   - Add invocation in C/C++ processor (after line 2260)
   - Update schema comments (lines 3527-3538)

## Success Criteria

- [ ] Behavioral hints extracted for C/C++ files
- [ ] Behavioral hints extracted for Python files
- [ ] Behavioral hints extracted for Go files
- [ ] Behavioral hints extracted for JavaScript/TypeScript files
- [ ] Existing Rust behavioral hints still work
- [ ] CI checks pass (fmt, clippy, test)
- [ ] Test files show expected patterns in database

## Notes

- Uses same table, reinterprets columns per language (follows `function_facts` pattern)
- Maintains backward compatibility
- Simple string matching (same as current Rust implementation)
- No new dependencies required
- Can be extended to more languages by adding cases to dispatcher