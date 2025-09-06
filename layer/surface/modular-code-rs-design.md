---
id: modular-code-rs-design
status: proposal
created: 2025-09-05
tags: [architecture, refactoring, languages, modular]
---

# Modular code.rs Design - Language Sections Within One File

## Overview

Instead of plugins or the current mixed approach, organize `code.rs` into clear language-specific sections within a single file. Each language gets its own module with complete ownership of its extraction logic.

## Proposed Structure

```rust
// src/commands/scrape/code.rs

// ============================================================================
// CHAPTER 1: SHARED TYPES AND INTERFACES
// ============================================================================

/// Common fact types that all languages populate
pub struct FunctionFact {
    pub name: String,
    pub file: String,
    pub is_public: bool,
    // ... common fields
    
    /// Language-specific data stored as JSON
    pub language_data: serde_json::Value,
}

pub struct ExtractedFacts {
    pub functions: Vec<FunctionFact>,
    pub types: Vec<TypeFact>,
    pub imports: Vec<ImportFact>,
    pub documentation: Vec<DocFact>,
    pub call_graph: Vec<CallEdge>,
}

/// Language extractor trait - internal to this file
trait LanguageExtractor {
    fn can_handle(&self, extension: &str) -> bool;
    fn extract(&self, source: &str, path: &Path) -> Result<ExtractedFacts>;
}

// ============================================================================
// CHAPTER 2: RUST LANGUAGE MODULE
// ============================================================================
mod rust {
    use super::*;
    use tree_sitter_rust;
    
    pub struct RustExtractor {
        parser: Parser,
    }
    
    impl RustExtractor {
        pub fn new() -> Result<Self> {
            let mut parser = Parser::new();
            parser.set_language(tree_sitter_rust::language())?;
            Ok(Self { parser })
        }
        
        fn extract_function(&self, node: Node, source: &str) -> FunctionFact {
            // ALL Rust-specific logic here
            let mut fact = FunctionFact {
                name: self.get_name(node, source),
                is_public: self.has_visibility_modifier(node),
                // ... common fields
                language_data: json!({
                    "lifetimes": self.extract_lifetimes(node, source),
                    "unsafe": self.has_unsafe(node),
                    "async": self.has_async(node),
                    "const": self.has_const(node),
                    "takes_mut_self": self.takes_mut_self(node, source),
                    "returns_result": self.returns_result(node, source),
                    "returns_option": self.returns_option(node, source),
                    "impl_trait": self.get_impl_trait(node),
                    "where_clause": self.extract_where_clause(node, source),
                    "attributes": self.extract_attributes(node, source),
                }),
            };
            fact
        }
        
        fn extract_lifetimes(&self, node: Node, source: &str) -> Vec<String> {
            // Rust-only lifetime extraction
            let mut lifetimes = Vec::new();
            if let Some(params) = node.child_by_field_name("type_parameters") {
                for child in params.children(&mut params.walk()) {
                    if child.kind() == "lifetime" {
                        lifetimes.push(child.utf8_text(source).unwrap_or("").to_string());
                    }
                }
            }
            lifetimes
        }
        
        fn extract_impl_block(&self, node: Node, source: &str) -> ImplBlock {
            // Rust-only impl block analysis
            ImplBlock {
                impl_type: self.get_impl_type(node, source),
                trait_name: self.get_impl_trait(node, source),
                methods: self.extract_impl_methods(node, source),
            }
        }
        
        fn extract_macro(&self, node: Node, source: &str) -> MacroDef {
            // Rust-only macro extraction
            MacroDef {
                name: self.get_macro_name(node, source),
                is_proc_macro: self.has_attribute(node, "proc_macro"),
                exported: self.has_attribute(node, "macro_export"),
            }
        }
    }
    
    impl LanguageExtractor for RustExtractor {
        fn can_handle(&self, extension: &str) -> bool {
            extension == "rs"
        }
        
        fn extract(&self, source: &str, path: &Path) -> Result<ExtractedFacts> {
            let tree = self.parser.parse(source, None).unwrap();
            let mut facts = ExtractedFacts::default();
            
            self.walk_tree(tree.root_node(), source, &mut facts);
            
            Ok(facts)
        }
    }
    
    // Rust-specific types that don't leak out
    struct ImplBlock {
        impl_type: String,
        trait_name: Option<String>,
        methods: Vec<String>,
    }
    
    struct MacroDef {
        name: String,
        is_proc_macro: bool,
        exported: bool,
    }
}

// ============================================================================
// CHAPTER 3: GO LANGUAGE MODULE
// ============================================================================
mod go {
    use super::*;
    use tree_sitter_go;
    
    pub struct GoExtractor {
        parser: Parser,
    }
    
    impl GoExtractor {
        pub fn new() -> Result<Self> {
            let mut parser = Parser::new();
            parser.set_language(tree_sitter_go::language())?;
            Ok(Self { parser })
        }
        
        fn extract_function(&self, node: Node, source: &str) -> FunctionFact {
            // ALL Go-specific logic here
            FunctionFact {
                name: self.get_name(node, source),
                is_public: self.is_exported(node, source), // Go uses capital letters
                language_data: json!({
                    "receiver": self.extract_receiver(node, source),
                    "returns_error": self.returns_error(node, source),
                    "multiple_returns": self.extract_return_values(node, source),
                    "goroutine_safe": self.analyze_goroutine_safety(node, source),
                    "defer_statements": self.extract_defers(node, source),
                }),
            }
        }
        
        fn is_exported(&self, node: Node, source: &str) -> bool {
            // Go-specific: exported if starts with capital letter
            if let Some(name) = self.get_name(node, source) {
                name.chars().next().map_or(false, |c| c.is_uppercase())
            } else {
                false
            }
        }
        
        fn extract_interface(&self, node: Node, source: &str) -> Interface {
            // Go-specific interface extraction
            Interface {
                name: self.get_name(node, source),
                methods: self.extract_interface_methods(node, source),
                embeds: self.extract_embedded_interfaces(node, source),
            }
        }
        
        fn analyze_goroutine_safety(&self, node: Node, source: &str) -> SafetyInfo {
            // Go-specific concurrency analysis
            SafetyInfo {
                uses_channels: self.uses_channels(node, source),
                uses_mutex: self.uses_mutex(node, source),
                has_race_condition: self.detect_race_conditions(node, source),
            }
        }
    }
    
    impl LanguageExtractor for GoExtractor {
        fn can_handle(&self, extension: &str) -> bool {
            extension == "go"
        }
        
        fn extract(&self, source: &str, path: &Path) -> Result<ExtractedFacts> {
            // Go-specific extraction
            let tree = self.parser.parse(source, None).unwrap();
            let mut facts = ExtractedFacts::default();
            
            self.walk_tree(tree.root_node(), source, &mut facts);
            
            Ok(facts)
        }
    }
    
    // Go-specific types
    struct Interface {
        name: String,
        methods: Vec<String>,
        embeds: Vec<String>,
    }
    
    struct SafetyInfo {
        uses_channels: bool,
        uses_mutex: bool,
        has_race_condition: bool,
    }
}

// ============================================================================
// CHAPTER 4: PYTHON LANGUAGE MODULE
// ============================================================================
mod python {
    use super::*;
    use tree_sitter_python;
    
    pub struct PythonExtractor {
        parser: Parser,
    }
    
    impl PythonExtractor {
        pub fn new() -> Result<Self> {
            let mut parser = Parser::new();
            parser.set_language(tree_sitter_python::language())?;
            Ok(Self { parser })
        }
        
        fn extract_function(&self, node: Node, source: &str) -> FunctionFact {
            // ALL Python-specific logic here
            FunctionFact {
                name: self.get_name(node, source),
                is_public: !self.get_name(node, source).starts_with("_"),
                language_data: json!({
                    "decorators": self.extract_decorators(node, source),
                    "is_async": self.is_async_def(node),
                    "is_generator": self.is_generator(node, source),
                    "type_hints": self.extract_type_hints(node, source),
                    "docstring": self.extract_docstring(node, source),
                    "is_property": self.has_decorator(node, "property"),
                    "is_staticmethod": self.has_decorator(node, "staticmethod"),
                    "is_classmethod": self.has_decorator(node, "classmethod"),
                }),
            }
        }
        
        fn extract_decorators(&self, node: Node, source: &str) -> Vec<Decorator> {
            // Python-specific decorator extraction
            let mut decorators = Vec::new();
            if node.kind() == "decorated_definition" {
                for child in node.children(&mut node.walk()) {
                    if child.kind() == "decorator" {
                        decorators.push(Decorator {
                            name: self.get_decorator_name(child, source),
                            args: self.get_decorator_args(child, source),
                        });
                    }
                }
            }
            decorators
        }
        
        fn extract_class(&self, node: Node, source: &str) -> ClassInfo {
            // Python-specific class extraction
            ClassInfo {
                name: self.get_name(node, source),
                bases: self.extract_base_classes(node, source),
                metaclass: self.extract_metaclass(node, source),
                is_dataclass: self.has_decorator(node, "dataclass"),
                slots: self.extract_slots(node, source),
            }
        }
        
        fn extract_type_hints(&self, node: Node, source: &str) -> TypeHints {
            // Python-specific type hint extraction
            TypeHints {
                params: self.extract_param_types(node, source),
                return_type: self.extract_return_type(node, source),
                is_fully_typed: self.check_full_typing(node, source),
            }
        }
    }
    
    impl LanguageExtractor for PythonExtractor {
        fn can_handle(&self, extension: &str) -> bool {
            matches!(extension, "py" | "pyi")
        }
        
        fn extract(&self, source: &str, path: &Path) -> Result<ExtractedFacts> {
            // Python-specific extraction
            Ok(facts)
        }
    }
}

// ============================================================================
// CHAPTER 5: SOLIDITY LANGUAGE MODULE
// ============================================================================
mod solidity {
    use super::*;
    use tree_sitter_solidity;
    
    pub struct SolidityExtractor {
        parser: Parser,
    }
    
    impl SolidityExtractor {
        fn extract_function(&self, node: Node, source: &str) -> FunctionFact {
            // ALL Solidity-specific logic here
            FunctionFact {
                name: self.get_name(node, source),
                is_public: self.get_visibility(node) == "public",
                language_data: json!({
                    "visibility": self.get_visibility(node),
                    "state_mutability": self.get_state_mutability(node),
                    "modifiers": self.extract_modifiers(node, source),
                    "is_payable": self.is_payable(node),
                    "is_view": self.is_view(node),
                    "is_pure": self.is_pure(node),
                    "is_virtual": self.is_virtual(node),
                    "is_override": self.is_override(node),
                    "gas_estimate": self.estimate_gas(node, source),
                }),
            }
        }
        
        fn extract_contract(&self, node: Node, source: &str) -> Contract {
            // Solidity-specific contract extraction
            Contract {
                name: self.get_name(node, source),
                is_abstract: self.is_abstract(node),
                inheritance: self.extract_inheritance(node, source),
                state_variables: self.extract_state_vars(node, source),
                events: self.extract_events(node, source),
                modifiers: self.extract_contract_modifiers(node, source),
            }
        }
        
        fn analyze_security(&self, node: Node, source: &str) -> SecurityAnalysis {
            // Solidity-specific security analysis
            SecurityAnalysis {
                has_reentrancy_risk: self.check_reentrancy(node, source),
                has_overflow_risk: self.check_overflow(node, source),
                has_unchecked_call: self.check_unchecked_calls(node, source),
                uses_delegate_call: self.uses_delegate_call(node, source),
            }
        }
    }
}

// ============================================================================
// CHAPTER 6: C/C++ LANGUAGE MODULE
// ============================================================================
mod c_cpp {
    use super::*;
    
    pub struct CExtractor {
        parser: Parser,
    }
    
    impl CExtractor {
        fn extract_function(&self, node: Node, source: &str) -> FunctionFact {
            // C-specific function extraction
            FunctionFact {
                name: self.extract_c_function_name(node, source), // Handle pointer syntax
                is_public: !self.is_static(node),
                language_data: json!({
                    "is_static": self.is_static(node),
                    "is_inline": self.is_inline(node),
                    "linkage": self.get_linkage(node),
                    "calling_convention": self.get_calling_convention(node, source),
                    "is_variadic": self.is_variadic(node, source),
                    "pointer_depth": self.get_pointer_depth(node, source),
                }),
            }
        }
        
        fn extract_c_function_name(&self, node: Node, source: &str) -> String {
            // C-specific: handle complex declarators iteratively
            let mut current = node;
            while current.kind() == "pointer_declarator" {
                if let Some(child) = current.child(1) {
                    current = child;
                } else {
                    break;
                }
            }
            
            if current.kind() == "function_declarator" {
                if let Some(declarator) = current.child_by_field_name("declarator") {
                    return self.extract_c_function_name(declarator, source);
                }
            }
            
            current.utf8_text(source).unwrap_or("").to_string()
        }
    }
}

// ============================================================================
// CHAPTER 7: ORCHESTRATION
// ============================================================================

pub struct LanguageRegistry {
    extractors: Vec<Box<dyn LanguageExtractor>>,
}

impl LanguageRegistry {
    pub fn new() -> Result<Self> {
        Ok(Self {
            extractors: vec![
                Box::new(rust::RustExtractor::new()?),
                Box::new(go::GoExtractor::new()?),
                Box::new(python::PythonExtractor::new()?),
                Box::new(solidity::SolidityExtractor::new()?),
                Box::new(c_cpp::CExtractor::new()?),
                Box::new(typescript::TypeScriptExtractor::new()?),
            ],
        })
    }
    
    pub fn extract(&self, path: &Path, source: &str) -> Result<ExtractedFacts> {
        let ext = path.extension()
            .and_then(|e| e.to_str())
            .ok_or_else(|| anyhow!("No file extension"))?;
        
        for extractor in &self.extractors {
            if extractor.can_handle(ext) {
                return extractor.extract(source, path);
            }
        }
        
        Err(anyhow!("No extractor for extension: {}", ext))
    }
}

// ============================================================================
// CHAPTER 8: MAIN EXTRACTION FUNCTION
// ============================================================================

pub fn extract_code_metadata(db_path: &str, work_dir: &Path) -> Result<()> {
    let registry = LanguageRegistry::new()?;
    
    for entry in WalkBuilder::new(work_dir).build() {
        let path = entry?.path();
        
        if let Ok(source) = std::fs::read_to_string(path) {
            match registry.extract(path, &source) {
                Ok(facts) => write_facts_to_db(db_path, &facts)?,
                Err(_) => continue, // Skip unsupported files
            }
        }
    }
    
    Ok(())
}
```

## Benefits of This Approach

### 1. **Clear Separation Within One File**
- Each language gets its own module section
- No cross-contamination between languages
- Easy to find all code for a specific language

### 2. **Shared Infrastructure**
- Common types at the top
- Shared database writing logic
- Single registry to manage all extractors

### 3. **Language-Specific Freedom**
- Each module can have its own helper structs
- Language-specific methods don't pollute others
- Can optimize for each language's patterns

### 4. **Progressive Enhancement**
- Start with common fields all languages share
- Add language-specific data as JSON
- Clients can access extra data if they need it

## Example: Adding a New Language

```rust
// Just add a new module section
mod zig {
    use super::*;
    use tree_sitter_zig;
    
    pub struct ZigExtractor {
        parser: Parser,
    }
    
    impl ZigExtractor {
        fn extract_function(&self, node: Node, source: &str) -> FunctionFact {
            FunctionFact {
                name: self.get_name(node, source),
                is_public: self.is_pub(node),
                language_data: json!({
                    "is_comptime": self.is_comptime(node),
                    "error_set": self.extract_error_set(node, source),
                    "is_inline": self.is_inline(node),
                    // Zig-specific features
                }),
            }
        }
    }
    
    impl LanguageExtractor for ZigExtractor {
        fn can_handle(&self, extension: &str) -> bool {
            extension == "zig"
        }
        
        fn extract(&self, source: &str, path: &Path) -> Result<ExtractedFacts> {
            // Zig extraction logic
        }
    }
}

// Then register it
impl LanguageRegistry {
    pub fn new() -> Result<Self> {
        Ok(Self {
            extractors: vec![
                // ... existing extractors
                Box::new(zig::ZigExtractor::new()?), // Just add this line
            ],
        })
    }
}
```

## File Organization

The file would be organized with clear chapter markers:

```
CHAPTER 1: SHARED TYPES (200 lines)
CHAPTER 2: RUST MODULE (400 lines)  
CHAPTER 3: GO MODULE (300 lines)
CHAPTER 4: PYTHON MODULE (350 lines)
CHAPTER 5: SOLIDITY MODULE (400 lines)
CHAPTER 6: C/C++ MODULE (500 lines)
CHAPTER 7: TYPESCRIPT MODULE (400 lines)
CHAPTER 8: ORCHESTRATION (100 lines)
CHAPTER 9: DATABASE OPERATIONS (200 lines)
```

Total: ~2850 lines, but each section is independent and focused.

## Advantages Over Current Approach

1. **Find things easily**: Want to change Rust extraction? Go to the Rust module.
2. **No conditional pollution**: No `if language == Rust` scattered everywhere
3. **Language-specific types**: Each module can define its own helper types
4. **Clear ownership**: Each language module owns its entire extraction pipeline
5. **Single file simplicity**: Still just one file to manage and understand

## DuckDB Integration - Keep Exact Same Tables

The beauty of this modular design is we can keep the **exact same DuckDB tables** we have now. Each language module just writes to the same schema differently:

### Current Tables (Unchanged)
```sql
-- Keep all existing tables exactly as they are
CREATE TABLE IF NOT EXISTS function_facts (
    file VARCHAR NOT NULL,
    name VARCHAR NOT NULL,
    takes_mut_self BOOLEAN,     -- Rust uses this
    takes_mut_params BOOLEAN,   -- Rust uses this  
    returns_result BOOLEAN,     -- Rust/Go use this
    returns_option BOOLEAN,     -- Rust uses this
    is_async BOOLEAN,          -- Multiple languages
    is_unsafe BOOLEAN,         -- Rust/C use this
    is_public BOOLEAN,         -- All languages
    parameter_count INTEGER,    -- All languages
    generic_count INTEGER,      -- Rust/TypeScript/Go
    parameters TEXT,            -- All languages
    return_type TEXT,           -- All languages
    PRIMARY KEY (file, name)
);

-- All other tables remain exactly the same
CREATE TABLE IF NOT EXISTS type_vocabulary (...);
CREATE TABLE IF NOT EXISTS import_facts (...);
CREATE TABLE IF NOT EXISTS documentation (...);
CREATE TABLE IF NOT EXISTS call_graph (...);
```

### How Each Language Module Populates the Same Tables

```rust
// ============================================================================
// RUST MODULE - Writes Rust-specific data to common tables
// ============================================================================
mod rust {
    impl RustExtractor {
        fn write_function_fact(&self, func: &RustFunction, sql: &mut String) {
            // Rust populates ALL relevant fields
            sql.push_str(&format!(
                "INSERT OR REPLACE INTO function_facts VALUES ('{}', '{}', {}, {}, {}, {}, {}, {}, {}, {}, {}, '{}', '{}');\n",
                func.file,
                func.name,
                func.takes_mut_self,      // ✓ Rust tracks this
                func.takes_mut_params,    // ✓ Rust tracks this
                func.returns_result,      // ✓ Rust tracks this
                func.returns_option,      // ✓ Rust tracks this
                func.is_async,           // ✓ Rust tracks this
                func.is_unsafe,          // ✓ Rust tracks this
                func.is_public,          // ✓ Rust tracks this
                func.parameters.len(),
                func.generics.len(),     // ✓ Rust tracks this
                func.parameters.join(", "),
                func.return_type
            ));
        }
    }
}

// ============================================================================
// GO MODULE - Writes Go-specific interpretation to same tables
// ============================================================================
mod go {
    impl GoExtractor {
        fn write_function_fact(&self, func: &GoFunction, sql: &mut String) {
            // Go only populates fields that make sense for Go
            sql.push_str(&format!(
                "INSERT OR REPLACE INTO function_facts VALUES ('{}', '{}', {}, {}, {}, {}, {}, {}, {}, {}, {}, '{}', '{}');\n",
                func.file,
                func.name,
                false,                    // takes_mut_self - N/A for Go
                false,                    // takes_mut_params - N/A for Go
                func.returns_error,       // ✓ Go uses this as "returns_result"
                false,                    // returns_option - N/A for Go
                false,                    // is_async - Go doesn't have async keyword
                false,                    // is_unsafe - N/A for Go
                func.is_exported,         // ✓ Go's version of "is_public"
                func.parameters.len(),
                func.type_params.len(),   // ✓ Go generics (if using Go 1.18+)
                func.parameters.join(", "),
                func.return_types.join(", ")  // Go can have multiple returns
            ));
        }
    }
}

// ============================================================================
// PYTHON MODULE - Writes Python-specific interpretation to same tables
// ============================================================================
mod python {
    impl PythonExtractor {
        fn write_function_fact(&self, func: &PythonFunction, sql: &mut String) {
            // Python maps its concepts to the common schema
            sql.push_str(&format!(
                "INSERT OR REPLACE INTO function_facts VALUES ('{}', '{}', {}, {}, {}, {}, {}, {}, {}, {}, {}, '{}', '{}');\n",
                func.file,
                func.name,
                false,                    // takes_mut_self - N/A for Python
                false,                    // takes_mut_params - N/A for Python  
                false,                    // returns_result - N/A for Python
                func.returns_optional,    // ✓ Python Optional[T] maps here
                func.is_async,           // ✓ Python async def
                false,                    // is_unsafe - N/A for Python
                !func.name.starts_with("_"), // ✓ Python convention for public
                func.parameters.len(),
                0,                        // generic_count - use 0 for Python
                func.parameters.join(", "),
                func.return_type_hint.unwrap_or("Any")
            ));
        }
    }
}

// ============================================================================
// C/C++ MODULE - Writes C-specific interpretation to same tables  
// ============================================================================
mod c_cpp {
    impl CExtractor {
        fn write_function_fact(&self, func: &CFunction, sql: &mut String) {
            // C maps what it can to the common schema
            sql.push_str(&format!(
                "INSERT OR REPLACE INTO function_facts VALUES ('{}', '{}', {}, {}, {}, {}, {}, {}, {}, {}, {}, '{}', '{}');\n",
                func.file,
                func.name,
                false,                    // takes_mut_self - N/A for C
                func.has_pointer_params,  // ✓ C uses this for pointer params
                false,                    // returns_result - N/A for C
                func.returns_null,        // ✓ C can use this for nullable returns
                false,                    // is_async - N/A for C
                false,                    // is_unsafe - all C is "unsafe" 
                !func.is_static,         // ✓ C static = private
                func.parameters.len(),
                0,                        // generic_count - N/A for C
                func.parameters.join(", "),
                func.return_type
            ));
        }
    }
}

// ============================================================================
// SOLIDITY MODULE - Writes Solidity-specific interpretation to same tables
// ============================================================================  
mod solidity {
    impl SolidityExtractor {
        fn write_function_fact(&self, func: &SolidityFunction, sql: &mut String) {
            // Solidity maps its unique concepts to common fields
            sql.push_str(&format!(
                "INSERT OR REPLACE INTO function_facts VALUES ('{}', '{}', {}, {}, {}, {}, {}, {}, {}, {}, {}, '{}', '{}');\n",
                func.file,
                func.name,
                false,                    // takes_mut_self - N/A
                func.modifies_state,      // ✓ Solidity uses for state changes
                false,                    // returns_result - N/A
                false,                    // returns_option - N/A
                false,                    // is_async - N/A for Solidity
                func.has_assembly,        // ✓ Solidity inline assembly = unsafe
                func.visibility == "public" || func.visibility == "external",
                func.parameters.len(),
                0,                        // generic_count - N/A
                func.parameters.join(", "),
                func.return_types.join(", ")  // Solidity can return tuples
            ));
        }
    }
}
```

### The Key Insight

Each language module:
1. **Extracts** data in a language-specific way
2. **Maps** its concepts to the common schema
3. **Writes** to the exact same tables

For example, `returns_result` field:
- **Rust**: Set to `true` if return type contains "Result"
- **Go**: Set to `true` if function returns an error
- **Python**: Always `false` (doesn't have Result type)
- **C**: Always `false`
- **Solidity**: Always `false`

This way:
- No schema changes needed
- Existing queries still work
- Each language uses fields that make sense for it
- Unused fields just get default values

### Benefits

1. **Zero Database Changes** - Keep exact same schema
2. **Backward Compatible** - All existing queries work
3. **Language Flexibility** - Each language interprets fields its own way
4. **Clean Modules** - Each language module knows how to map its concepts
5. **No Conditionals** - No `if language == "rust"` in shared code

## Migration Path

1. Start with current code.rs
2. Create language modules one at a time
3. Move language-specific extraction logic into modules
4. Each module writes to same tables with its interpretation
5. Remove old mixed extraction code

This gives us clean separation while keeping the exact same database structure!