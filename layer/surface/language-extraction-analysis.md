---
id: language-extraction-analysis
status: active
created: 2025-01-09
tags: [scrape, extraction, languages, analysis, implementation]
---

# Language Extraction Analysis: Current State & Missing Facts

A comprehensive analysis of what each language processor extracts and what critical facts are missing for LLM code generation.

## Overview

The `patina scrape code` command uses language-specific processors to extract semantic information from codebases. Each processor returns `ExtractedData` structs that populate DuckDB tables. This analysis documents the current extraction capabilities and missing facts for each language, based on real database analysis of production repositories.

**Repositories Analyzed:**
- **SDL** - C codebase with 11,997 functions
- **DuckDB** - C++ codebase with 52,424 functions  
- **Dagger** - Go (+ TypeScript/Python/Rust) with 9,402 functions
- **Dust** - Solidity/TypeScript with 2,746 functions
- **Dojo** - Rust/Cairo with 2,376 functions

## Current Database Schema

All languages populate these core tables:
```sql
code_search      -- Symbol index (name, kind, line, context)
function_facts   -- Function metadata (parameters, return types, flags)
type_vocabulary  -- Type definitions (structs, classes, enums)
import_facts     -- Import/include statements
call_graph       -- Function call relationships
```

## C Language (src/commands/scrape/code/languages/c.rs)

### Currently Extracts ✅
- **Functions**: Names, parameters, return types
- **Types**: struct, union, enum, typedef
- **Imports**: #include statements
- **Call graph**: Direct function calls
- **Public/private**: Inferred from .h vs .c files

### Missing Critical Facts ❌
1. **Preprocessor Macros**
   - `#define MAX_SIZE 100` - Not captured
   - `#define SDL_INIT_VIDEO 0x00000020` - Missing API constants
   - Impact: Can't use C APIs correctly without macro values

2. **Global/Static Variables**
   - `static int cache_size = 1024;` - Not extracted
   - `const char* VERSION = "1.0.0";` - Missing
   - Impact: Don't know about global state

3. **Enum Values**
   - Captures `enum Status` but not `STATUS_OK = 0, STATUS_ERROR = -1`
   - Impact: Can't use enums properly

4. **Struct Fields**
   - Captures struct names but not member fields
   - Impact: Can't construct or access structs

### Implementation Changes Needed

**File: `src/commands/scrape/code/languages/c.rs`**

Add new node type handlers in `extract_c_symbols()`:
```rust
match node.kind() {
    // ADD: Preprocessor macro extraction
    "preproc_def" => {
        // Extract #define NAME VALUE
        // Add to new constants collection
    }
    
    // ADD: Global variable extraction  
    "declaration" => {
        // Check for static/const/extern
        // Extract variable declarations
    }
    
    // MODIFY: Enum handling
    "enum_specifier" => {
        // Currently only captures enum name
        // Need to iterate through enumerator_list
        // Extract each enum value
    }
    
    // ADD: Struct field extraction
    "field_declaration_list" => {
        // Extract struct/union fields
        // Add as MemberFact with kind="field"
    }
}
```

## C++ Language (src/commands/scrape/code/languages/cpp.rs)

### Currently Extracts ✅
- **Functions**: Including namespace-qualified names
- **Classes**: Class names with namespace context
- **Types**: struct, class, enum, typedef, using
- **Namespaces**: Captured in function/type names (e.g., `duckdb::Connection`)
- **Templates**: Basic template detection
- **Imports**: #include statements

### Missing Critical Facts ❌
1. **Class Members**
   - No distinction between methods and free functions
   - Missing member fields
   - No public/private/protected visibility
   - No virtual/static/const method markers
   - No constructor/destructor special handling

2. **Constants and Statics**
   - `static constexpr int MAX = 100;` - Not captured
   - Class static members not extracted
   - Global constants missing

3. **Inheritance Relationships**
   - `class Derived : public Base` - Relationship not captured
   - Impact: Can't understand class hierarchies

4. **Template Parameters**
   - `template<typename T, int N>` - Parameters not extracted
   - Impact: Can't use templates correctly

### Implementation Changes Needed

**File: `src/commands/scrape/code/languages/cpp.rs`**

Enhance class processing:
```rust
"class_specifier" | "struct_specifier" => {
    // Current: Only captures class name
    
    // ADD: Process base_clause for inheritance
    if let Some(base) = node.child_by_field_name("base_clause") {
        // Extract parent classes
    }
    
    // ADD: Process field_declaration_list
    if let Some(body) = node.child_by_field_name("body") {
        // Track current access level (public/private/protected)
        // Extract fields with visibility
        // Distinguish methods from fields
        // Mark virtual/static/const
    }
}

// ADD: Method vs function distinction
"function_definition" => {
    // Check if inside class context
    let is_method = !class_stack.is_empty();
    
    // For methods, check for special names
    let is_constructor = name == class_name;
    let is_destructor = name.starts_with('~');
    
    // Extract method qualifiers (const, virtual, override)
}
```

## Go Language (src/commands/scrape/code/languages/go.rs)

### Currently Extracts ✅
- **Functions**: With public/private based on capitalization
- **Methods**: Separate from functions (detected via receivers)
- **Structs**: Type definitions
- **Interfaces**: Interface types
- **Imports**: Import statements
- **Error Returns**: Detects `error` and tuple returns `(string, error)`

### Missing Critical Facts ❌
1. **Constants**
   - `const MaxSize = 100` - Not captured despite being common
   - `const (...)` blocks with iota - Pattern not recognized
   - Impact: Missing important API constants

2. **Package Declarations**
   - `package main` - Not extracted
   - Impact: Don't know module organization

3. **Global Variables**
   - `var globalCache map[string]string` - Not captured
   - Impact: Missing state management patterns

4. **Init Functions**
   - `func init()` - Special initialization not marked
   - Impact: Don't understand startup sequences

5. **Goroutine Usage**
   - `go processAsync()` - Concurrency not tracked
   - Impact: Can't identify concurrent patterns

### Implementation Changes Needed

**File: `src/commands/scrape/code/languages/go.rs`**

Add constant and package extraction:
```rust
match node.kind() {
    // ADD: Package declaration
    "package_clause" => {
        // Extract package name
        // Add as NamespaceFact
    }
    
    // ADD: Const declaration
    "const_declaration" | "const_spec" => {
        // Extract constant name and value
        // Handle const blocks with iota
    }
    
    // ADD: Var declaration (globals)
    "var_declaration" => {
        // Check if at package level (global)
        // Extract as ConstantFact with kind="global"
    }
    
    // MODIFY: Call expression
    "call_expression" => {
        // Check for "go" keyword prefix
        // Mark as CallType::Concurrent
    }
}
```

## Rust Language (src/commands/scrape/code/languages/rust.rs)

### Currently Extracts ✅
- **Functions**: With async/unsafe/pub markers
- **Constants**: `const NAME: type = value` ✅ (WORKING!)
- **Statics**: Static variables captured
- **Types**: struct, enum, trait, type aliases
- **Impl blocks**: Methods within implementations
- **Modules**: `mod` declarations
- **Mutability**: Detects `&mut self`, `&mut` params

### Missing Facts (Relatively Complete) ⚠️
1. **Generic Parameters**
   - `fn foo<T: Display, U>()` - Constraints not captured
   - Impact: Can't use generics properly

2. **Lifetime Annotations**
   - `fn bar<'a>(x: &'a str)` - Lifetimes ignored
   - Impact: Missing borrowing patterns

3. **Trait Implementations**
   - `impl Display for MyType` - Relationship not captured
   - Impact: Don't know what traits are implemented

4. **Macro Definitions**
   - `macro_rules!` not extracted (only usage tracked)
   - Impact: Can't understand DSLs

### Implementation Changes Needed

**File: `src/commands/scrape/code/languages/rust.rs`**

Enhance impl block handling:
```rust
"impl_item" => {
    // Current: Processes methods inside
    
    // ADD: Check for trait implementation
    if let Some(trait_node) = node.child_by_field_name("trait") {
        // Extract trait being implemented
        // Store as impl_facts
    }
    
    // ADD: Extract generic parameters
    if let Some(params) = node.child_by_field_name("type_parameters") {
        // Extract generics with bounds
    }
}
```

## Python Language (src/commands/scrape/code/languages/python.rs)

### Currently Extracts ✅
- **Functions**: Including async functions
- **Classes**: Class definitions with methods
- **Methods**: Distinguished from functions (have `self`)
- **Imports**: from/import statements
- **Decorators**: Tracked in call graph
- **Docstrings**: Captured for functions/classes

### Missing Facts ❌
1. **Global Variables/Constants**
   - `MAX_SIZE = 100` - Convention-based constants not tracked
   - Impact: Missing configuration values

2. **Class Inheritance**
   - `class Child(Parent):` - Parent not captured
   - Impact: Can't understand class hierarchies

3. **Type Hints**
   - `def foo(x: int) -> str:` - Type annotations ignored
   - Impact: Missing type information

4. **Class Variables vs Instance Variables**
   - Can't distinguish `Class.var` from `self.var`
   - Impact: Confusion about scope

### Implementation Changes Needed

**File: `src/commands/scrape/code/languages/python.rs`**

Add type hints and inheritance:
```rust
"class_definition" => {
    // ADD: Extract superclasses
    if let Some(args) = node.child_by_field_name("superclasses") {
        // Extract parent classes
    }
}

"function_definition" => {
    // ADD: Extract type annotations
    if let Some(return_type) = node.child_by_field_name("return_type") {
        // Extract return type hint
    }
    
    // For parameters, extract type hints
}

// ADD: Module-level assignments (constants)
"assignment" => {
    // Check if at module level
    // Extract as potential constant (UPPER_CASE convention)
}
```

## TypeScript/JavaScript Language (src/commands/scrape/code/languages/typescript.rs)

### Currently Extracts ✅
- **Functions**: Arrow functions, async functions
- **Classes**: With methods tracked separately
- **Types**: Interfaces, type aliases, enums
- **Imports**: ES6 imports, requires
- **JSX**: Components detected
- **Methods**: Constructor, getters, setters marked

### Missing Facts ❌
1. **Class Member Visibility**
   - `private field: string` - Visibility not captured
   - `protected method()` - Access modifiers ignored
   - Impact: Can't respect encapsulation

2. **Constants**
   - `const MAX_SIZE = 100` - Module constants not tracked
   - Impact: Missing configuration values

3. **Generic Types**
   - `interface Box<T>` - Type parameters not extracted
   - Impact: Can't use generics

4. **Decorators**
   - `@Component` - Metadata not captured
   - Impact: Missing framework patterns

### Implementation Changes Needed

**File: `src/commands/scrape/code/languages/typescript.rs`**

Add visibility and constants:
```rust
"class_body" => {
    // ADD: Track access modifiers
    // Look for "public", "private", "protected", "readonly"
    
    // ADD: Static members
    // Check for "static" keyword
}

"lexical_declaration" => {
    // ADD: Module-level const
    if node.kind() == "const" && at_module_level {
        // Extract as constant
    }
}
```

## Solidity Language (src/commands/scrape/code/languages/solidity.rs)

### Currently Extracts ✅
- **Contracts**: Contract definitions
- **Libraries**: Library contracts
- **State Variables**: With visibility ✅ (WORKING!)
- **Functions**: With modifiers
- **Modifiers**: Function modifiers
- **Structs**: Custom types
- **Events**: Some event detection

### Missing Facts ❌
1. **Inheritance**
   - `contract Token is ERC20, Ownable` - Parents not captured
   - Impact: Can't understand contract hierarchies

2. **Function Modifiers Details**
   - `payable`, `view`, `pure` - Not distinguished
   - Impact: Can't call functions correctly

3. **Event Parameters**
   - Events detected but parameters not extracted
   - Impact: Can't emit events properly

4. **Mappings**
   - `mapping(address => uint256)` - Type not parsed
   - Impact: Can't use storage correctly

### Implementation Changes Needed

**File: `src/commands/scrape/code/languages/solidity.rs`**

Already relatively complete, but enhance:
```rust
"contract_declaration" => {
    // ADD: Extract inheritance
    if let Some(heritage) = node.child_by_field_name("heritage") {
        // Extract parent contracts
    }
}

"function_definition" => {
    // ADD: Check for payable/view/pure
    // Store in function metadata
}
```

## Cairo Language (src/commands/scrape/code/languages/cairo.rs)

### Currently Extracts ✅
- **Functions**: Using native cairo-lang-parser (not tree-sitter)
- **Structs**: Type definitions
- **Traits**: Trait definitions
- **Imports**: Use statements

### Missing Facts ❌
Cairo uses a completely different parser, limiting extraction:

1. **Storage Variables**
   - Contract storage not extracted
   - Impact: Can't understand state

2. **Implementations**
   - `impl ContractImpl of IContract` - Not captured
   - Impact: Missing trait implementations

3. **Attributes**
   - `#[external]`, `#[view]` - Not extracted
   - Impact: Don't know function visibility

### Implementation Changes Needed

**File: `src/commands/scrape/code/languages/cairo.rs`**

This requires enhancing the patina_metal::cairo parser:
```rust
// Current limitation: patina_metal::cairo::parse_cairo
// returns limited CairoSymbols struct
// Need to extend the parser in patina_metal crate
```

## Implementation Priority

Based on impact and usage:

### High Priority (C/C++ - Most broken)
1. **C/C++ constants/macros** - Critical for API usage
2. **C++ class members** - Essential for OOP code
3. **C/C++ enum values** - Required for correct usage

### Medium Priority (Go/Python - Important gaps)
1. **Go constants** - Common pattern missing
2. **Python type hints** - Modern Python needs these
3. **Go package declarations** - Module organization

### Low Priority (Rust/TypeScript/Solidity - Mostly complete)
1. **Generic parameters** - Advanced usage
2. **Inheritance relationships** - Nice to have
3. **Lifetime annotations** - Rust-specific

## Next Steps

1. **Extend ExtractedData struct** in `extracted_data.rs`:
   - Add `constants: Vec<ConstantFact>`
   - Add `members: Vec<MemberFact>`
   - Add `enum_values: Vec<EnumValueFact>`

2. **Update database schema** in `database.rs`:
   - Add new tables for missing facts
   - Ensure cross-language compatibility

3. **Implement extraction** in each language processor:
   - Start with C/C++ (highest impact)
   - Test on SDL/DuckDB repositories
   - Verify with `ask` command patterns

## Success Metrics

After implementation, we should see:
- SDL: Capture all `SDL_*` macros and enum values
- DuckDB: Distinguish class methods from free functions
- Dagger: Extract Go constants and package structure
- All: Constants, globals, and proper method/function distinction

---

*"The goal is to extract sufficient facts so an LLM can write code that looks native to each language and codebase."*