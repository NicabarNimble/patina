# LLM-Optimized Code Intelligence - Implementation Plan

## Current Status ✅
- Vendored grammars working
- Basic fact extraction into DuckDB
- mtime-based incremental updates
- Import tracking exists
- Function facts and complexity metrics captured

## What We Need 🔧
- Documentation extraction and storage
- Call graph tracking
- Context retrieval engine
- LLM formatting layer

## Phase 1: Documentation Extraction 🚀 PRIORITY

### Schema Addition
```sql
CREATE TABLE documentation (
    file VARCHAR,
    symbol_name VARCHAR,
    symbol_type VARCHAR,
    line_number INTEGER,
    doc_raw TEXT,
    doc_clean TEXT,
    doc_summary VARCHAR,
    keywords VARCHAR[],
    doc_length INTEGER,
    has_examples BOOLEAN,
    has_params BOOLEAN,
    parent_symbol VARCHAR,
    PRIMARY KEY (file, symbol_name)
);
```

### Parser Changes
- [ ] Add `extract_doc_comment()` to `process_ast_node()`
- [ ] Look for comment nodes before symbol nodes
- [ ] Implement language-specific cleaning:
  - Rust: Strip `///` and `//!`
  - Python: Extract docstrings
  - TypeScript: Clean JSDoc `/** */`
  - Go: Strip `//`

### Keyword Extraction
```rust
fn extract_keywords(doc: &str) -> Vec<String> {
    const STOP_WORDS: &[&str] = &["the", "and", "for", "with", "this"];
    
    doc.split_whitespace()
        .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric()))
        .filter(|w| w.len() > 3)
        .filter(|w| !STOP_WORDS.contains(&w.to_lowercase().as_str()))
        .map(|w| w.to_lowercase())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect()
}
```

## Phase 2: Call Graph Extraction

### Schema Addition
```sql
CREATE TABLE call_graph (
    caller VARCHAR,
    callee VARCHAR,
    file VARCHAR,
    call_type VARCHAR
);

CREATE INDEX idx_caller ON call_graph(caller);
CREATE INDEX idx_callee ON call_graph(callee);
```

### Parser Changes
- [ ] Track current function context during parsing
- [ ] Identify call expressions:
  - Direct calls: `foo()`
  - Method calls: `obj.method()`
  - Async calls: `foo().await`
- [ ] Store caller → callee relationships

### Call Detection Patterns
```rust
match node.kind() {
    "call_expression" => {
        let function_name = extract_function_name(node);
        store_call(current_function, function_name, "direct");
    }
    "method_call_expression" => {
        let method_name = extract_method_name(node);
        store_call(current_function, method_name, "method");
    }
    _ => {}
}
```

## Phase 3: Context Retrieval Engine

### Create `src/context/` module
- [ ] `keyword_search.rs` - Find symbols by keywords
- [ ] `graph_traversal.rs` - Recursive CTE queries
- [ ] `context_builder.rs` - Assemble relevant facts
- [ ] `formatter.rs` - Format for different LLMs

### Core Queries

```rust
// Find entry points
pub fn find_symbols_by_keyword(keyword: &str) -> String {
    format!(
        "SELECT DISTINCT symbol_name, doc_summary, file, line_number
         FROM documentation
         WHERE list_contains(keywords, '{}')
            OR symbol_name ILIKE '%{}%'
         ORDER BY doc_length DESC
         LIMIT 20",
        keyword, keyword
    )
}

// Expand via call graph
pub fn expand_context(symbols: Vec<String>) -> String {
    format!(
        "WITH RECURSIVE context AS (
            SELECT '{}' as symbol
            UNION
            SELECT callee
            FROM call_graph cg
            JOIN context c ON cg.caller = c.symbol
         )
         SELECT * FROM context",
        symbols.join("' as symbol UNION SELECT '")
    )
}
```

## Phase 4: LLM Formatter

### Token Budget Management
```rust
pub struct ContextBudget {
    max_tokens: usize,
    used_tokens: usize,
    priority_queue: BinaryHeap<RankedSymbol>,
}

impl ContextBudget {
    pub fn add_symbol(&mut self, symbol: Symbol, relevance: f32) {
        let tokens = estimate_tokens(&symbol);
        if self.used_tokens + tokens <= self.max_tokens {
            self.priority_queue.push(RankedSymbol { symbol, relevance });
            self.used_tokens += tokens;
        }
    }
}
```

### Output Format
```rust
pub fn format_for_llm(context: Context) -> String {
    let mut output = String::new();
    
    // Entry points first
    output.push_str("## Entry Points\n");
    for func in context.entry_points {
        output.push_str(&format!(
            "- `{}({}) -> {}`\n  {}\n  File: {}:{}\n",
            func.name, func.params, func.return_type,
            func.doc_summary, func.file, func.line
        ));
    }
    
    // Core types
    output.push_str("\n## Core Types\n");
    // ...
    
    // Call chains
    output.push_str("\n## Call Chains\n");
    // ...
    
    output
}
```

## Phase 5: Integration

### CLI Commands
- [ ] Add `patina context <question>` command
- [ ] Add `--max-tokens` flag for budget control
- [ ] Add `--format` flag for different LLM formats

### Example Usage
```bash
# Get context for a question
patina context "How does authentication work?"

# With token limit
patina context "What's the build system?" --max-tokens 3000

# Format for specific LLM
patina context "Error handling patterns" --format claude
```

## Testing Strategy

### Test Queries
1. "How does authentication work?"
2. "What's the error handling strategy?"
3. "Where is database access implemented?"
4. "Can you build a caching module?"

### Success Metrics
- Context fits in 2-5K tokens (vs 50-100K for raw files)
- Includes all relevant symbols (via graph traversal)
- Docs validate against code facts
- Response time < 1 second

## Migration Path

1. **Week 1**: Doc extraction (ship immediately)
2. **Week 2**: Call graph (adds completeness)
3. **Week 3**: Context retrieval (makes it useful)
4. **Week 4**: LLM formatting (makes it production-ready)

Each phase delivers value independently - no big bang required!