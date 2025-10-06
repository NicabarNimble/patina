---
id: language-extraction-analysis
status: active
created: 2025-01-09
tags: [scrape, extraction, languages, analysis, implementation]
---

# Language Extraction Analysis: Current State & Missing Facts

A comprehensive analysis of what each language processor extracts and what critical facts are missing for LLM code generation.

## Overview

The `patina scrape code` command uses language-specific processors to extract semantic information from codebases. Each processor returns `ExtractedData` structs that populate SQLite tables. This analysis documents the current extraction capabilities and missing facts for each language, based on real database analysis of production repositories.

**Extraction Complete:** C, C++, Go, Rust, Python, TypeScript, JavaScript, Solidity, Cairo ✅ (9/9)

All supported languages now extract comprehensive semantic information for LLM code generation.

**Repositories Analyzed:**
- **SDL** - C codebase with 11,997 functions
- **DuckDB** - C++ codebase with 52,424 functions
- **Dagger** - Go (+ TypeScript/Python/Rust) with 9,402 functions
- **Cortex** - Python codebase with 205 functions, 85 constants, 165 members
- **Gemini-CLI** - TypeScript codebase with 2,165 functions, 12,047 constants, 5,505 members
- **game-engine** - JavaScript codebase with 307 functions, 97 constants, 2 members
- **Dust** - Solidity/TypeScript with 2,678 functions, 272 constants (135 inheritance)
- **Dojo** - Rust/Cairo with 2,202 functions, 973 constants (296 trait impls), 1,562 members

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
- **Functions**: With async/unsafe/pub markers, parameter text, return types
- **Methods**: Within impl blocks, distinguished from free functions
- **Types**: struct, enum, trait, type aliases with visibility
- **Imports**: use statements with path and imported names
- **Constants**: `const NAME: type = value` stored as ConstantFact
- **Statics**: Static variables with values stored as ConstantFact
- **Macro Definitions**: `macro_rules!` definitions stored as ConstantFact
- **Struct Fields**: All fields with pub/private visibility stored as MemberFact
- **Enum Variants**: Variants with discriminant values stored as both ConstantFact and MemberFact
- **Trait Implementations**: `impl Trait for Type` relationships stored as ConstantFact
- **Mutability**: Detects `&mut self`, `&mut` params in functions
- **Generic Count**: Number of type parameters on functions tracked
- **Call Graph**: Function calls and macro invocations tracked

### Implementation Complete ✅

Rust extraction validated on Loro repository (309 files):
- 819 trait implementations for understanding type capabilities
- 538 enum variants with discriminant values where specified  
- 1,227 struct fields with proper visibility detection
- 89 const definitions and 13 statics with values
- 19 macro definitions captured
- Full support for Rust's ownership patterns through mutability tracking

## Python Language (src/commands/scrape/code/languages/python.rs)

### Currently Extracts ✅
- **Functions**: Including async functions with full type hints
- **Classes**: Class definitions with methods and inheritance
- **Methods**: Distinguished from functions with visibility (public/private/special)
- **Imports**: from/import statements
- **Decorators**: Tracked in call graph
- **Docstrings**: Captured for functions/classes
- **Type Hints**: Full preservation of parameter and return type annotations
- **Module Constants**: UPPER_CASE convention-based detection stored as ConstantFact
- **Module Variables**: Package-level assignments stored as ConstantFact
- **Class Inheritance**: Parent classes stored as ConstantFact with type="inheritance"
- **Class Members**: Methods and constructors stored as MemberFact with visibility
- **Class Variables**: Extracted from class body assignments

### Implementation Complete ✅

All critical Python language features are now extracted:
- Cortex validation: 85 constants, 165 class members, 205 functions
- Module-level constants detected by UPPER_CASE naming convention
- Full type hint preservation including complex annotations like `Optional[str]`, `Dict[str, Union[str, int]]`
- Inheritance relationships stored as special constant_facts (e.g., `OpenAIController::inherits_from::BaseLLMController`)
- Method visibility detection based on underscore conventions (public, private with `_`, special with `__magic__`)
- Constructor/destructor classification for `__init__` and `__del__` methods

## TypeScript/JavaScript Language (src/commands/scrape/code/languages/typescript.rs)

### Currently Extracts ✅
- **Functions**: Arrow functions, async functions, generators
- **Classes**: With full member extraction and inheritance
- **Types**: Interfaces, type aliases, enums with members
- **Imports**: ES6 imports, type imports, requires
- **JSX**: Components detected (.tsx parser support)
- **Methods**: Constructor, getters, setters with visibility
- **Constants**: Module-level const declarations stored as ConstantFact
- **Class Members**: Fields and methods with visibility (public/private/protected)
- **Inheritance**: Class extends and implements relationships
- **Interface Members**: Properties, methods, index signatures
- **Enum Values**: Individual enum members with values
- **Decorators**: Framework decorators stored as ConstantFact
- **Generic Parameters**: Type parameters on interfaces/classes
- **Export Patterns**: Default and named exports tracked

### Implementation Complete ✅

All critical TypeScript language features are now extracted:
- Gemini-CLI validation: 12,047 constants, 5,505 members from 547 classes/interfaces
- 4,117 const variables with 360 UPPER_CASE constants properly identified
- Full visibility detection: 272 private fields, 222 public fields, 165 private methods
- Interface properties (1,581) and methods properly extracted
- Class inheritance and interface implementation stored as special constant_facts
- Decorators captured for framework pattern recognition (@Injectable, @Component)
- Generic type parameters stored for proper type usage
- Readonly and static modifiers preserved on members

## JavaScript Language (src/commands/scrape/code/languages/javascript.rs)

### Currently Extracts ✅
- **Functions**: Function declarations, arrow functions, async functions, generators
- **Classes**: With full member extraction and inheritance
- **Methods**: Constructor, getters, setters with modifiers
- **Imports**: ES6 imports and CommonJS require statements
- **JSX**: Components detected (same parser handles .js and .jsx)
- **Constants**: Module-level const declarations stored as ConstantFact
- **Class Members**: Fields and methods with static/async modifiers
- **Inheritance**: Class extends relationships tracked
- **Private Fields**: Modern # private fields detected
- **Export Patterns**: Module exports tracked
- **Call Graph**: Async/await, new expressions, direct calls

### Implementation Complete ✅

All critical JavaScript language features are now extracted:
- game-engine validation: 307 functions, 97 constants (22 UPPER_CASE config constants, 73 module variables), 2 members
- Full support for ES6+ module patterns (import/export)
- Class inheritance tracked via extends clause
- Method modifiers (static, async) properly preserved
- Private fields using # syntax detected
- Module-level const detection with UPPER_CASE convention for config constants
- Both arrow functions and traditional function declarations handled
- Constructor, getter, and setter classification

**JavaScript vs TypeScript:**
- JavaScript extraction is simpler (no type system, decorators, or interfaces)
- Same tree-sitter parser handles both .js and .jsx files
- Focuses on runtime patterns rather than compile-time types
- Inheritance tracking works identically but only extends (no implements)

## Solidity Language (src/commands/scrape/code/languages/solidity.rs)

### Currently Extracts ✅
- **Contracts**: Contract definitions
- **Libraries**: Library contracts
- **Interfaces**: Interface declarations
- **State Variables**: With visibility (public, private, internal, external)
- **Functions**: Function definitions with full metadata
- **Function Modifiers**: `payable`, `view`, `pure` extracted and stored in FunctionFact
- **Modifiers**: Custom function modifiers
- **Structs**: Custom types
- **Enums**: Enum declarations
- **Events**: Event definitions with parameters
- **Inheritance**: Contract inheritance relationships stored as ConstantFacts ✅
- **Imports**: Import directives with path detection
- **Call Graph**: Function calls, method calls, and contract creation

### Implementation Complete ✅

All critical Solidity language features are now extracted:
- Dust validation: 272 constants (including 135 inheritance relationships), 2,678 functions
- Inheritance stored as ConstantFacts: `ContractName::inherits::BaseContract`
- Multiple inheritance fully supported: `contract DustTest is MudTest, GasReporter, DustAssertions`
- Function modifiers (payable, view, pure) stored in FunctionFact metadata
- Event parameters extracted and stored
- State variables with proper visibility detection
- Full coverage for Solidity smart contract patterns

**Example inheritance extraction:**
```
IWorld::inherits::IBaseWorld
CraftTest::inherits::DustTest
DustTest::inherits::MudTest
DustTest::inherits::GasReporter
DustTest::inherits::DustAssertions
```

## Cairo Language (src/commands/scrape/code/languages/cairo.rs)

### Currently Extracts ✅
- **Functions**: Using native cairo-lang-parser with full metadata
- **Structs**: Type definitions with field extraction
- **Struct Fields**: All fields stored as MemberFacts ✅
- **Traits**: Trait definitions
- **Trait Implementations**: impl Trait for Type stored as ConstantFacts ✅
- **Modules**: Module declarations
- **Imports**: Use statements

### Implementation Complete ✅

All critical Cairo language features are now extracted:
- Dojo validation: 973 constants (296 trait implementations), 1,562 members (461 struct fields)
- Trait implementations stored as ConstantFacts: `TypeName::implements::TraitName`
- Struct fields stored as MemberFacts with visibility
- Uses cairo-lang-parser (not tree-sitter) for robust Cairo 2.x support

**Example trait implementation extraction:**
```
Upgradeable::implements::super::IUpgradeable<ComponentState<TContractState>>
WorldProvider::implements::super::IWorldProvider<ComponentState<TContractState>>
```

**Example struct field extraction:**
```
Server: hooks, exporter
WorldMetadata: world_address, models
Dependency: name, read, write
```

**Remaining limitations** (not critical for LLM code generation):
- Attributes (`#[external]`, `#[view]`) not extracted - parser doesn't expose them yet
- Contract storage variables - similar to attributes, parser limitation
- These would require enhancing patina-metal/src/cairo.rs parser

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