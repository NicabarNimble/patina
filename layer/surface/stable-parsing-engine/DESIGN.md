# LLM-Optimized Code Intelligence System

## Core Philosophy

**Extract facts, build relationships, retrieve context efficiently.**

We're not just storing code and docs - we're building a graph of knowledge that LLMs can query intelligently. 10-50x more token-efficient than crawling raw files.

## Architecture

```
Code Files → Tree-sitter Parse → Extract Facts → DuckDB Storage
                     ↓                                ↓
            [Documentation]                   [Call Graph]
            [Function Facts]                  [Import Graph]
                     ↓                                ↓
                     └────────────────────────────────┘
                                    ↓
                          Context Retrieval Engine
                                    ↓
                        Focused LLM Context (2-5K tokens)
```

## Storage Design

### 1. Documentation Table (New)
```sql
CREATE TABLE documentation (
    -- Identity
    file VARCHAR,
    symbol_name VARCHAR,
    symbol_type VARCHAR,  -- 'function', 'struct', 'module', 'field'
    line_number INTEGER,
    
    -- Raw content
    doc_raw TEXT,         -- Original with comment markers
    doc_clean TEXT,       -- Cleaned text for display
    
    -- Search/retrieval optimization
    doc_summary VARCHAR,  -- First sentence (fast preview)
    keywords VARCHAR[],   -- Extracted: ['auth', 'token', 'validate']
    
    -- Quality signals
    doc_length INTEGER,   -- Character count
    has_examples BOOLEAN, -- Contains code blocks
    has_params BOOLEAN,   -- Documents parameters
    
    -- Relationships
    parent_symbol VARCHAR, -- For nested items
    
    PRIMARY KEY (file, symbol_name)
);
```

### 2. Call Graph Table (New)
```sql
CREATE TABLE call_graph (
    caller VARCHAR,
    callee VARCHAR,
    file VARCHAR,
    call_type VARCHAR  -- 'direct', 'method', 'callback'
);
```

### 3. Existing Tables (Enhanced)
- `function_facts` - Already captures signatures, parameters, return types
- `import_facts` - Already tracks module dependencies
- `type_vocabulary` - Already has type definitions
- `code_fingerprints` - Already has complexity metrics

## Context Retrieval Strategy

### For Questions Like "How does auth work?"

1. **Keyword Search** - Find relevant symbols via doc keywords
```sql
SELECT symbol_name FROM documentation
WHERE list_contains(keywords, 'auth')
   OR symbol_name ILIKE '%auth%';
```

2. **Graph Expansion** - Find related code via call graph
```sql
WITH RECURSIVE auth_context AS (
    -- Entry points
    SELECT symbol_name FROM matches
    UNION
    -- What they call
    SELECT callee FROM call_graph
    JOIN auth_context ON caller = symbol_name
)
SELECT * FROM auth_context;
```

3. **Fact Assembly** - Combine docs + code facts + relationships
```sql
SELECT 
    d.symbol_name,
    d.doc_summary,
    f.parameters,
    f.return_type,
    cg.calls
FROM documentation d
JOIN function_facts f ON d.symbol_name = f.name
LEFT JOIN (
    SELECT caller, array_agg(callee) as calls
    FROM call_graph GROUP BY caller
) cg ON d.symbol_name = cg.caller;
```

4. **Format for LLM** - Structured, focused context
```
## Authentication System

### Entry Points
- `authenticate_user(credentials: Credentials) -> Result<Token>`
  "Validates user credentials and returns JWT token"
  Calls: validate_credentials, generate_token

### Core Types
- `struct Token { ... }`
  "JWT token with expiration and claims"

### Implementation Chain
authenticate_user → validate_credentials → check_password → hash_compare
```

## Token Efficiency Comparison

| Approach | Tokens | Accuracy | Completeness |
|----------|--------|----------|--------------|
| Raw file crawling | 50,000-100,000 | Low (too much noise) | High (sees everything) |
| Our system | 2,000-5,000 | High (validated facts) | High (graph traversal) |
| Grep + context | 10,000-20,000 | Medium | Low (misses relationships) |

## Implementation Phases

### Phase 1: Documentation Extraction
- Parse doc comments above symbols
- Extract keywords for search
- Store in documentation table

### Phase 2: Call Graph Building
- Track function calls during parsing
- Build caller → callee relationships
- Store in call_graph table

### Phase 3: Context Retrieval
- Implement keyword → symbol search
- Add recursive graph traversal
- Build context assembly queries

### Phase 4: LLM Integration
- Format context for different LLMs
- Implement token budget management
- Add relevance ranking

## Key Insights

1. **Docs as Search Signals** - Documentation isn't truth, it's a map to find truth in code
2. **Graph Relationships Matter** - Auth isn't one function, it's a web of connected code
3. **Progressive Detail** - Start with summaries, expand to full context as needed
4. **Token Budget Awareness** - Rank and filter by relevance to fit context windows

## Why This Works

- **10-50x fewer tokens** than feeding raw files to LLMs
- **More accurate** because we validate docs against code facts
- **More complete** because we follow relationships
- **Query-driven** rather than dump-everything approach

## DuckDB Advantages

- **Recursive CTEs** for graph traversal (no graph DB needed)
- **Array columns** for keyword search (no FTS5 needed)
- **Analytical queries** for pattern detection
- **Single file** deployment stays simple