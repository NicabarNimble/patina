---
id: language-plugin-architecture
status: proposal
created: 2025-09-05
tags: [architecture, refactoring, plugins, languages]
---

# Language Plugin Architecture for Patina

## Problem Statement

The current `code.rs` file (3000+ lines) tries to handle all languages generically with:
- Language-specific edge cases scattered throughout
- Complex conditional logic (`match language` everywhere)
- Difficult to add new languages
- Hard to maintain language-specific optimizations

## Proposed Solution: Language Plugins

### Core Architecture

```rust
// src/languages/mod.rs
pub trait LanguagePlugin: Send + Sync {
    /// Language identifier
    fn language(&self) -> &'static str;
    
    /// File extensions this plugin handles
    fn extensions(&self) -> &[&'static str];
    
    /// Create a tree-sitter parser for this language
    fn create_parser(&self) -> Result<Parser>;
    
    /// Extract all facts from a file
    fn extract_facts(&self, source: &str, path: &Path) -> Result<ExtractedFacts>;
    
    /// Language-specific query optimizations (optional)
    fn optimize_query(&self, query: &str) -> String {
        query.to_string()
    }
}

/// All facts extracted from a file
pub struct ExtractedFacts {
    pub functions: Vec<FunctionFact>,
    pub types: Vec<TypeFact>,
    pub imports: Vec<ImportFact>,
    pub documentation: Vec<DocFact>,
    pub call_graph: Vec<CallEdge>,
    pub search_entries: Vec<SearchEntry>,
}
```

### Language Plugin Example: Rust

```rust
// src/languages/rust.rs
pub struct RustPlugin;

impl LanguagePlugin for RustPlugin {
    fn language(&self) -> &'static str {
        "rust"
    }
    
    fn extensions(&self) -> &[&'static str] {
        &["rs"]
    }
    
    fn create_parser(&self) -> Result<Parser> {
        let mut parser = Parser::new();
        parser.set_language(tree_sitter_rust::language())?;
        Ok(parser)
    }
    
    fn extract_facts(&self, source: &str, path: &Path) -> Result<ExtractedFacts> {
        let mut facts = ExtractedFacts::default();
        let tree = self.parse(source)?;
        
        // Rust-specific extraction with full control
        self.walk_tree(&tree, source, &mut facts);
        
        Ok(facts)
    }
}

impl RustPlugin {
    /// Rust-specific: Extract lifetime parameters
    fn extract_lifetimes(&self, node: Node, source: &str) -> Vec<String> {
        // Rust-only feature
    }
    
    /// Rust-specific: Track unsafe blocks within safe functions
    fn track_unsafe_blocks(&self, node: Node, source: &str) -> Vec<UnsafeBlock> {
        // Rust-only analysis
    }
    
    /// Rust-specific: Analyze trait implementations
    fn analyze_trait_impls(&self, node: Node, source: &str) -> Vec<TraitImpl> {
        // Rust-only feature
    }
    
    /// Rust-specific: Extract macro definitions and invocations
    fn extract_macros(&self, node: Node, source: &str) -> Vec<MacroInfo> {
        // Rust-only feature
    }
}
```

### Language Plugin Example: Solidity

```rust
// src/languages/solidity.rs
pub struct SolidityPlugin;

impl SolidityPlugin {
    /// Solidity-specific: Extract contract inheritance
    fn extract_inheritance(&self, node: Node, source: &str) -> Vec<String> {
        // Solidity-only: contract Foo is Bar, Baz
    }
    
    /// Solidity-specific: Track state variables and their visibility
    fn extract_state_variables(&self, node: Node, source: &str) -> Vec<StateVar> {
        // Solidity-only feature
    }
    
    /// Solidity-specific: Analyze modifiers (payable, view, pure)
    fn analyze_modifiers(&self, node: Node, source: &str) -> Vec<Modifier> {
        // Solidity-only feature
    }
    
    /// Solidity-specific: Extract events and their parameters
    fn extract_events(&self, node: Node, source: &str) -> Vec<Event> {
        // Solidity-only feature
    }
}
```

### Language Plugin Example: Python

```rust
// src/languages/python.rs
pub struct PythonPlugin;

impl PythonPlugin {
    /// Python-specific: Extract decorator metadata
    fn extract_decorators(&self, node: Node, source: &str) -> Vec<Decorator> {
        // Python-only: @property, @staticmethod, etc.
    }
    
    /// Python-specific: Analyze type hints
    fn analyze_type_hints(&self, node: Node, source: &str) -> TypeHints {
        // Python-only: def foo(x: int) -> str
    }
    
    /// Python-specific: Extract class metaclasses
    fn extract_metaclasses(&self, node: Node, source: &str) -> Vec<Metaclass> {
        // Python-only feature
    }
    
    /// Python-specific: Track async context managers
    fn track_async_context(&self, node: Node, source: &str) -> Vec<AsyncContext> {
        // Python-only: async with statements
    }
}
```

### Plugin Registry

```rust
// src/languages/registry.rs
pub struct LanguageRegistry {
    plugins: HashMap<String, Box<dyn LanguagePlugin>>,
    extension_map: HashMap<String, String>, // ext -> language
}

impl LanguageRegistry {
    pub fn new() -> Self {
        let mut registry = Self::default();
        
        // Register built-in plugins
        registry.register(Box::new(RustPlugin));
        registry.register(Box::new(GoPlugin));
        registry.register(Box::new(SolidityPlugin));
        registry.register(Box::new(PythonPlugin));
        registry.register(Box::new(TypeScriptPlugin));
        registry.register(Box::new(CPlugin));
        
        registry
    }
    
    pub fn register(&mut self, plugin: Box<dyn LanguagePlugin>) {
        let lang = plugin.language();
        for ext in plugin.extensions() {
            self.extension_map.insert(ext.to_string(), lang.to_string());
        }
        self.plugins.insert(lang.to_string(), plugin);
    }
    
    pub fn get_plugin_for_file(&self, path: &Path) -> Option<&dyn LanguagePlugin> {
        let ext = path.extension()?.to_str()?;
        let lang = self.extension_map.get(ext)?;
        self.plugins.get(lang).map(|p| p.as_ref())
    }
}
```

### Simplified Main Extraction Loop

```rust
// src/commands/scrape/code.rs (dramatically simplified)
pub fn extract_code_metadata(db_path: &str, work_dir: &Path) -> Result<()> {
    let registry = LanguageRegistry::new();
    let mut all_facts = Vec::new();
    
    for entry in WalkBuilder::new(work_dir).build() {
        let entry = entry?;
        let path = entry.path();
        
        // Get the right plugin for this file
        if let Some(plugin) = registry.get_plugin_for_file(path) {
            let source = std::fs::read_to_string(path)?;
            let facts = plugin.extract_facts(&source, path)?;
            all_facts.push((path.to_path_buf(), facts));
        }
    }
    
    // Write all facts to database
    write_facts_to_db(db_path, all_facts)?;
    Ok(())
}
```

## Benefits

### 1. **Separation of Concerns**
- Each language owns its extraction logic
- No cross-language pollution
- Clear boundaries

### 2. **Language-Specific Optimizations**
- Rust plugin can track lifetimes and unsafe blocks
- Solidity plugin can analyze gas costs
- Python plugin can extract type hints
- Go plugin can track goroutines

### 3. **Easier to Add Languages**
- Implement the trait
- Register the plugin
- Done!

### 4. **Easier to Test**
- Test each language plugin independently
- Mock plugins for testing the framework
- Language-specific test fixtures

### 5. **Parallel Development**
- Different people can work on different language plugins
- No merge conflicts in a giant file
- Clear ownership

### 6. **Dynamic Loading (Future)**
```rust
// Could even load plugins dynamically
let plugin = load_plugin_from_file("./plugins/zig.so")?;
registry.register(plugin);
```

## Migration Strategy

### Phase 1: Extract Language Specs
1. Move current `LanguageSpec` structs to separate files
2. Keep existing extraction logic working

### Phase 2: Create Plugin Trait
1. Define the `LanguagePlugin` trait
2. Create adapter that wraps existing specs

### Phase 3: Migrate Languages One by One
1. Start with Rust (most complete)
2. Then Go, Python, TypeScript
3. Finally C/C++ (most complex)

### Phase 4: Remove Old System
1. Delete the generic extraction code
2. Remove `LanguageSpec` system
3. Clean up `code.rs` to just orchestrate plugins

## Example: How Rust Plugin Would Handle Edge Cases

```rust
impl RustPlugin {
    fn extract_function_facts(&self, func_node: Node, source: &str) -> FunctionFact {
        // No need to check language or handle other languages' edge cases
        let mut fact = FunctionFact::default();
        
        // Direct Rust-specific logic
        fact.is_unsafe = self.has_child_kind(func_node, "unsafe");
        fact.is_async = self.has_child_kind(func_node, "async");
        fact.is_const = self.has_child_kind(func_node, "const");
        
        // Rust-specific: Check for #[must_use]
        fact.must_use = self.has_attribute(func_node, "must_use");
        
        // Rust-specific: impl block context
        if let Some(impl_block) = self.find_parent_impl(func_node) {
            fact.impl_trait = self.extract_impl_trait(impl_block);
        }
        
        // Rust-specific: lifetime parameters
        fact.lifetimes = self.extract_lifetimes(func_node);
        
        fact
    }
}
```

## Database Schema Extensions

Each plugin could even extend the schema:

```rust
impl RustPlugin {
    fn additional_tables(&self) -> Vec<TableDefinition> {
        vec![
            // Rust-specific tables
            TableDefinition {
                name: "rust_lifetimes",
                sql: "CREATE TABLE rust_lifetimes (
                    function VARCHAR,
                    lifetime VARCHAR,
                    bounds VARCHAR
                )"
            },
            TableDefinition {
                name: "rust_macros",
                sql: "CREATE TABLE rust_macros (
                    name VARCHAR,
                    definition TEXT,
                    is_procedural BOOLEAN
                )"
            }
        ]
    }
}
```

## Configuration

Each plugin could have its own configuration:

```toml
# .patina/config.toml

[languages.rust]
extract_lifetimes = true
track_unsafe_blocks = true
analyze_macro_expansion = false  # Expensive

[languages.python]
extract_type_hints = true
follow_imports = true
analyze_metaclasses = false

[languages.solidity]
estimate_gas_costs = true
analyze_reentrancy = true
track_state_changes = true
```

## Concrete Example: Full Rust Plugin Implementation

```rust
// src/languages/rust.rs - Complete plugin showing how it would work

use anyhow::Result;
use tree_sitter::{Parser, Node};
use std::path::Path;

pub struct RustPlugin {
    parser: Parser,
}

impl RustPlugin {
    pub fn new() -> Result<Self> {
        let mut parser = Parser::new();
        parser.set_language(tree_sitter_rust::language())?;
        Ok(Self { parser })
    }
    
    pub fn extract_facts(&self, source: &str, path: &Path) -> Result<ExtractedFacts> {
        let tree = self.parser.parse(source, None)
            .ok_or_else(|| anyhow!("Failed to parse Rust file"))?;
        
        let mut facts = ExtractedFacts::default();
        let mut context = RustContext::new();
        
        self.walk_node(tree.root_node(), source, &mut facts, &mut context);
        
        Ok(facts)
    }
    
    fn walk_node(&self, node: Node, source: &str, facts: &mut ExtractedFacts, ctx: &mut RustContext) {
        match node.kind() {
            "function_item" => {
                self.extract_function(node, source, facts, ctx);
            }
            "impl_item" => {
                // Track impl context for methods
                ctx.enter_impl(self.extract_impl_info(node, source));
                self.walk_children(node, source, facts, ctx);
                ctx.exit_impl();
            }
            "struct_item" => {
                self.extract_struct(node, source, facts);
            }
            "enum_item" => {
                self.extract_enum(node, source, facts);
            }
            "trait_item" => {
                self.extract_trait(node, source, facts);
            }
            "use_declaration" => {
                self.extract_import(node, source, facts);
            }
            "macro_definition" => {
                self.extract_macro(node, source, facts);
            }
            "mod_item" => {
                ctx.enter_module(self.get_name(node, source));
                self.walk_children(node, source, facts, ctx);
                ctx.exit_module();
            }
            _ => {
                self.walk_children(node, source, facts, ctx);
            }
        }
    }
    
    fn extract_function(&self, node: Node, source: &str, facts: &mut ExtractedFacts, ctx: &RustContext) {
        let name = self.get_name(node, source);
        
        // Create function fact with Rust-specific details
        let mut func = FunctionFact {
            name: name.clone(),
            file: ctx.current_file.clone(),
            module_path: ctx.module_path(),
            
            // Basic Rust function attributes
            is_async: self.has_async(node),
            is_unsafe: self.has_unsafe(node),
            is_const: self.has_const(node),
            is_pub: self.has_pub(node),
            
            // Rust-specific visibility
            visibility: self.extract_visibility(node, source),
            
            // Parameters with Rust-specific details
            takes_self: self.takes_self(node, source),
            takes_mut_self: self.takes_mut_self(node, source),
            parameters: self.extract_parameters(node, source),
            
            // Return type analysis
            return_type: self.extract_return_type(node, source),
            returns_result: self.returns_result(node, source),
            returns_option: self.returns_option(node, source),
            
            // Generics and lifetimes (Rust-specific)
            generics: self.extract_generics(node, source),
            lifetimes: self.extract_lifetimes(node, source),
            where_clause: self.extract_where_clause(node, source),
            
            // Rust-specific attributes
            attributes: self.extract_attributes(node, source),
            is_test: self.has_attribute(node, "test"),
            is_bench: self.has_attribute(node, "bench"),
            must_use: self.has_attribute(node, "must_use"),
            
            // If inside impl block
            impl_trait: ctx.current_impl.as_ref().map(|i| i.trait_name.clone()),
            impl_type: ctx.current_impl.as_ref().map(|i| i.type_name.clone()),
        };
        
        // Extract documentation with Rust-specific parsing
        if let Some(doc) = self.extract_doc_comment(node, source) {
            func.documentation = Some(self.parse_rust_doc(doc));
        }
        
        facts.functions.push(func);
    }
    
    // Rust-specific helper methods
    fn extract_lifetimes(&self, node: Node, source: &str) -> Vec<Lifetime> {
        let mut lifetimes = Vec::new();
        if let Some(params) = node.child_by_field_name("type_parameters") {
            for child in params.children(&mut params.walk()) {
                if child.kind() == "lifetime" {
                    let name = child.utf8_text(source)?;
                    let bounds = self.extract_lifetime_bounds(child, source);
                    lifetimes.push(Lifetime { name, bounds });
                }
            }
        }
        lifetimes
    }
    
    fn parse_rust_doc(&self, raw_doc: String) -> Documentation {
        let mut doc = Documentation::default();
        
        // Parse sections like # Examples, # Panics, # Safety
        for line in raw_doc.lines() {
            if line.starts_with("# Examples") {
                doc.has_examples = true;
            } else if line.starts_with("# Panics") {
                doc.panic_conditions = Some(String::new());
            } else if line.starts_with("# Safety") {
                doc.safety_requirements = Some(String::new());
            }
        }
        
        doc
    }
}
```

## Comparison: Current vs Plugin Architecture

### Current Approach (Scattered Logic)
```rust
// Current code.rs - mixed concerns everywhere
fn extract_function_facts(node: Node, source: &str, language: Language) {
    match language {
        Language::Rust => {
            // Rust-specific logic
            let takes_mut_self = /* Rust check */;
            let returns_result = /* Rust check */;
        }
        Language::Go => {
            // Go-specific logic
            let returns_error = /* Go check */;
        }
        Language::Python => {
            // Python-specific logic
            let has_decorators = /* Python check */;
        }
        // ... more languages
    }
    
    // Generic logic trying to handle all cases
    if language == Language::Solidity {
        // Special Solidity handling
    } else if language == Language::C {
        // Special C handling
    }
    // ... edge cases everywhere
}
```

### Plugin Approach (Clean Separation)
```rust
// Each language owns its logic completely
impl RustPlugin {
    fn extract_function(&self, node: Node) -> FunctionFact {
        // ONLY Rust logic, no conditionals for other languages
        FunctionFact {
            takes_mut_self: self.check_mut_self(node),
            returns_result: self.check_result_return(node),
            lifetimes: self.extract_lifetimes(node), // Rust-only feature
            // ... Rust-specific fields
        }
    }
}

impl GoPlugin {
    fn extract_function(&self, node: Node) -> FunctionFact {
        // ONLY Go logic
        FunctionFact {
            returns_error: self.check_error_return(node),
            goroutine_safe: self.check_goroutine_safety(node), // Go-only
            // ... Go-specific fields
        }
    }
}
```

## How Tree-Sitter Grammars Work with Plugins

Each plugin owns its parser configuration:

```rust
impl LanguagePlugin for RustPlugin {
    fn create_parser(&self) -> Parser {
        let mut parser = Parser::new();
        // Each plugin knows exactly which grammar it needs
        parser.set_language(tree_sitter_rust::language()).unwrap();
        parser
    }
    
    fn node_queries(&self) -> &'static str {
        // Language-specific tree-sitter queries
        r#"
        (function_item
          name: (identifier) @function.name
          parameters: (parameters) @function.params
          return_type: (_)? @function.return
          body: (block) @function.body)
        
        (impl_item
          trait: (type_identifier)? @impl.trait
          type: (type_identifier) @impl.type)
        "#
    }
}
```

## Summary

Moving to a plugin architecture would:
1. Reduce code.rs from 3000+ lines to ~200 lines
2. Make each language's extraction logic clear and maintainable
3. Allow language-specific optimizations and features
4. Make adding new languages trivial
5. Enable community contributions (just write a plugin!)
6. Allow parallel development and testing

The key insight: **Each language is different enough that trying to handle them generically creates more complexity than it saves.**

### Real Benefits Example
With plugins, adding unsafe block tracking for Rust doesn't affect ANY other language:
```rust
// Just add to RustPlugin, no impact on other languages
impl RustPlugin {
    fn track_unsafe_blocks(&self, node: Node) -> Vec<UnsafeBlock> {
        // Rust-only feature, no conditionals needed
    }
}
```

Versus current approach where you'd need to:
1. Add a field to shared structs
2. Add conditionals to check if language is Rust
3. Handle the case where other languages don't have unsafe
4. Worry about breaking other language extraction