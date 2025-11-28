# Spec: Lexical Search (FTS5)

**Status:** Phase 3 - Not Started

## Overview

FTS5 provides exact-match search for when Claude needs to find specific symbols, definitions, or patterns. Complements vector search - vectors find concepts, FTS5 finds exact text.

**Why needed:**
- "Where is `COMPONENT_ID` defined?" - vector search might miss exact match
- "Find all uses of `@storage_var`" - needs literal string matching
- "Show me the `spawn_entity` function" - exact symbol lookup

## Integration with Scry

Scry auto-detects query type and routes appropriately:

```
Query → Detect Type → Route
         ↓
    "error handling"     → Semantic (vectors)
    "find COMPONENT_ID"  → Lexical (FTS5)
    --file src/foo.rs    → Temporal (file lookup)
```

**Detection heuristics:**
- Starts with "find ", "where is", "show me the" → Lexical
- Contains `::`, `()`, `@`, exact symbol patterns → Lexical
- Everything else → Semantic

## Schema Addition

Add to `patina.db` during scrape:

```sql
-- FTS5 virtual table for code search
CREATE VIRTUAL TABLE IF NOT EXISTS code_fts USING fts5(
    symbol_name,      -- Function, struct, const names
    file_path,        -- Source file location
    content,          -- Code content/docstrings
    event_type,       -- code.function, code.struct, etc.
    tokenize='porter unicode61'
);

-- Populate from eventlog
INSERT INTO code_fts (symbol_name, file_path, content, event_type)
SELECT
    json_extract(data, '$.name') as symbol_name,
    source_id as file_path,
    json_extract(data, '$.content') as content,
    event_type
FROM eventlog
WHERE event_type LIKE 'code.%'
  AND json_extract(data, '$.name') IS NOT NULL;
```

## Query Interface

```rust
// src/commands/scry/lexical.rs

pub fn lexical_search(query: &str, limit: usize) -> Result<Vec<ScryResult>> {
    let conn = Connection::open(".patina/data/patina.db")?;

    // FTS5 match syntax
    let fts_query = prepare_fts_query(query);

    let mut stmt = conn.prepare(
        "SELECT
            symbol_name,
            file_path,
            snippet(code_fts, 2, '>>>', '<<<', '...', 32) as snippet,
            event_type,
            bm25(code_fts) as score
         FROM code_fts
         WHERE code_fts MATCH ?
         ORDER BY score
         LIMIT ?"
    )?;

    let results = stmt.query_map([&fts_query, &limit.to_string()], |row| {
        Ok(ScryResult {
            id: 0,
            content: row.get::<_, String>(2)?,  // snippet
            score: -row.get::<_, f32>(4)?,      // BM25 is negative, lower is better
            event_type: row.get(3)?,
            source_id: format!("{}:{}", row.get::<_, String>(1)?, row.get::<_, String>(0)?),
            timestamp: String::new(),
        })
    })?;

    results.collect()
}

fn prepare_fts_query(query: &str) -> String {
    // Strip "find ", "where is ", etc.
    let cleaned = query
        .trim_start_matches("find ")
        .trim_start_matches("where is ")
        .trim_start_matches("show me the ")
        .trim_start_matches("show me ");

    // Quote for exact match if contains special chars
    if cleaned.contains("::") || cleaned.contains("()") {
        format!("\"{}\"", cleaned)
    } else {
        cleaned.to_string()
    }
}

pub fn is_lexical_query(query: &str) -> bool {
    let lower = query.to_lowercase();

    // Explicit lexical patterns
    lower.starts_with("find ") ||
    lower.starts_with("where is ") ||
    lower.starts_with("show me the ") ||
    lower.contains("defined") ||

    // Code symbol patterns
    query.contains("::") ||
    query.contains("()") ||
    query.contains("@") ||
    query.contains("fn ") ||
    query.contains("struct ") ||
    query.contains("const ")
}
```

## CLI Examples

```bash
# Exact symbol search
patina scry "find spawn_entity"
# → contracts/Game.cairo:spawn_entity (function)
# → >>>fn spawn_entity(world: @IWorldDispatcher)<<< ...

# With context
patina scry "where is COMPONENT_ID defined"
# → lib/components.cairo:COMPONENT_ID (const)
# → >>>const COMPONENT_ID: felt252 = 0x123<<<

# Pattern search
patina scry "find @storage_var"
# → contracts/Storage.cairo (multiple matches)
# → >>>@storage_var<<< func balances...
# → >>>@storage_var<<< func allowances...
```

## Scrape Integration

Add FTS5 population to `patina scrape code`:

```rust
// src/commands/scrape/code/mod.rs

fn populate_fts5(conn: &Connection) -> Result<()> {
    // Clear existing FTS5 data
    conn.execute("DELETE FROM code_fts", [])?;

    // Repopulate from eventlog
    conn.execute(
        "INSERT INTO code_fts (symbol_name, file_path, content, event_type)
         SELECT
             json_extract(data, '$.name'),
             source_id,
             json_extract(data, '$.content'),
             event_type
         FROM eventlog
         WHERE event_type LIKE 'code.%'
           AND json_extract(data, '$.name') IS NOT NULL",
        [],
    )?;

    Ok(())
}
```

## Performance

- FTS5 is built into SQLite, no external dependencies
- BM25 ranking is fast and well-understood
- Index size: ~10-20% of content size
- Query time: <10ms for most queries

## Acceptance Criteria

- [ ] FTS5 virtual table created during scrape
- [ ] `is_lexical_query()` correctly detects exact match patterns
- [ ] `patina scry "find X"` returns exact matches
- [ ] Results include highlighted snippets
- [ ] BM25 scoring orders results by relevance
