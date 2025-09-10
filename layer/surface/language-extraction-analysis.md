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
- **Preprocessor Macros**: `#define NAME VALUE` stored as ConstantFact
- **Global/Static Variables**: static/const/extern declarations as ConstantFact
- **Enum Values**: Individual enum constants with scope
- **Struct/Union Fields**: Member fields stored as MemberFact with type info

### Implementation Complete ✅

All critical C language features are now extracted:
- SDL validation: 35,431 constants, 330 struct fields
- Macros, enum values, and globals stored in constant_facts
- Struct/union fields stored in member_facts with type context
- Full coverage for C API usage patterns

## C++ Language (src/commands/scrape/code/languages/cpp.rs)

### Currently Extracts ✅
- **Functions**: Including namespace-qualified names
- **Classes**: Class names with namespace context
- **Types**: struct, class, enum, typedef, using
- **Namespaces**: Captured in function/type names (e.g., `duckdb::Connection`)
- **Templates**: Basic template detection
- **Imports**: #include statements
- **Class Members**: Fields and methods with visibility (public/private/protected)
- **Method Types**: Constructors, destructors, virtual, static, const methods
- **Constants and Statics**: Macros, global/static/const/constexpr variables
- **Inheritance**: Base classes with access specifiers stored as ConstantFact
- **Enum Values**: Scoped and unscoped enum constants

### Implementation Complete ✅

Critical C++ OOP features are now extracted:
- DuckDB validation: 1,944 inheritance relationships, 8,347 members
- Accurate visibility detection using context analysis
- Method distinction from free functions with modifiers
- Inheritance stored as special constant_facts with access levels
- Multiple inheritance fully supported

## Go Language (src/commands/scrape/code/languages/go.rs)

### Currently Extracts ✅
- **Functions**: With public/private based on capitalization
- **Methods**: Separate from functions (detected via receivers)
- **Structs**: Type definitions with all fields
- **Interfaces**: Interface types with methods and embedded types
- **Imports**: Import statements with aliases
- **Error Returns**: Detects `error` and tuple returns `(string, error)`
- **Constants**: Both single `const` and `const()` blocks with iota support
- **Package Declarations**: Package names stored as special constants
- **Global Variables**: Package-level `var` declarations
- **Struct Fields**: All struct members with visibility and tags
- **Interface Methods**: Method specifications and embedded interfaces
- **Goroutines**: Tracked via CallType::Goroutine in call graph
- **Defer Calls**: Tracked via CallType::Defer in call graph

### Implementation Complete ✅

Go extraction fully supports idiomatic code generation:
- Dagger validation: 750 packages, 1,923 constants (including 375 globals)
- 3,972 struct fields with proper public/private visibility
- 347 interface methods and 62 embedded interfaces
- Full support for iota patterns in const blocks
- Goroutine concurrency patterns tracked in call graph

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

## Completed Languages

**C and C++ extraction is complete:**
- SDL: 35,431 constants (macros, enums, globals), 330 struct fields ✅
- DuckDB: 1,944 inheritance relationships, 8,347 class members, proper visibility ✅

**Next priorities for other languages:**
- Go: Constants and package declarations (high impact)
- Python: Type hints and class inheritance (modernization)
- Rust: Already strong, minor improvements only

---

*"The goal is to extract sufficient facts so an LLM can write code that looks native to each language and codebase."*