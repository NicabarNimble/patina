# Spec: Assay Command

**Purpose:** Expose structural codebase data via CLI and MCP. Complement scry (semantic) with exact structural queries.

**Origin:** Dogfooding insight from Pass 1 code audit - gathering module inventory required 40+ shell tool calls. Patina already has the data in SQLite but no query interface.

---

## Problem

LLMs exploring codebases need both:
- **Semantic queries** (scry): "find code related to authentication"
- **Structural queries** (assay): "list all modules with stats", "what imports X"

Currently, structural queries require shell tools (`wc -l`, `find`, `grep`), causing:
- Many tool calls (40+ for a full inventory)
- High token usage (repeated file listings)
- Stale context (no single structured view)

Patina already collects structural data during scrape:
- 1,189 functions in `function_facts`
- 718 imports in `import_facts`
- 15,340 call edges in `call_graph`
- File sizes in `index_state`

**Gap:** No query interface for this data.

---

## Solution

Add `patina assay` command + MCP tool for structural queries.

### Design Principles

1. **Universal first** - No language-specific logic in Phase 0
2. **Expose existing data** - Query what scrape already collects
3. **Complement scry** - Different tool for different query type
4. **Fresh data** - Recommend scrape before assay (490ms cost)

---

## CLI Interface

```bash
# Module inventory (default)
patina assay                        # All modules, sorted by line count
patina assay src/commands           # Filter by path pattern
patina assay inventory "**/*.rs"    # Explicit subcommand

# Dependency queries
patina assay imports <module>       # What does <module> import?
patina assay importers <module>     # What imports <module>?

# Function queries
patina assay functions              # All functions with signatures
patina assay functions <pattern>    # Functions matching pattern

# Call graph queries
patina assay callers <function>     # What calls <function>?
patina assay callees <function>     # What does <function> call?

# Output format
patina assay --json                 # JSON output
patina assay --limit 100            # Limit results
```

---

## MCP Tool

```json
{
  "name": "assay",
  "description": "Query codebase structure - modules, imports, functions, call graph. Use for exact structural questions like 'list all modules', 'what imports X', 'show largest files'. For semantic similarity, use scry instead.",
  "inputSchema": {
    "type": "object",
    "properties": {
      "query_type": {
        "type": "string",
        "enum": ["inventory", "imports", "importers", "functions", "callers", "callees"],
        "default": "inventory",
        "description": "Type of structural query"
      },
      "pattern": {
        "type": "string",
        "description": "Path pattern or function name to filter results"
      },
      "limit": {
        "type": "integer",
        "default": 50,
        "description": "Maximum results to return"
      }
    }
  }
}
```

---

## Data Schema

### Inventory Output

```json
{
  "modules": [
    {
      "path": "src/retrieval/engine.rs",
      "lines": 353,
      "bytes": 12450,
      "functions": 12,
      "imports": 8,
      "public_functions": 4,
      "last_modified": "2025-12-17T14:30:00Z"
    }
  ],
  "summary": {
    "total_files": 136,
    "total_lines": 28000,
    "total_functions": 1189
  }
}
```

### Imports Output

```json
{
  "module": "src/retrieval/engine.rs",
  "imports": [
    {"path": "anyhow::Result", "kind": "external"},
    {"path": "super::oracle::Oracle", "kind": "internal"},
    {"path": "rayon::prelude::*", "kind": "external"}
  ]
}
```

### Functions Output

```json
{
  "functions": [
    {
      "name": "query_with_options",
      "file": "src/retrieval/engine.rs",
      "line": 89,
      "is_public": true,
      "is_async": false,
      "parameters": ["query: &str", "limit: usize", "options: &QueryOptions"],
      "return_type": "Result<Vec<FusedResult>>"
    }
  ]
}
```

---

## Implementation

### Phase 0 Scope (This Spec)

| Task | Effort | Notes |
|------|--------|-------|
| Add line_count to scrape | ~10 lines | Count newlines during file read |
| Add `assay` CLI command | ~150 lines | Query SQLite, format output |
| Add `assay` MCP tool | ~50 lines | Wire into server.rs |
| Update scrape to populate line_count | ~5 lines | Add column to index_state |

**Total:** ~215 lines

### Files to Modify

```
src/commands/scrape/code/database.rs  # Add line_count column
src/commands/assay/mod.rs             # New command (create)
src/mcp/server.rs                     # Add assay tool
src/main.rs                           # Wire assay command
```

### SQL Queries

```sql
-- Inventory
SELECT
  path,
  size as bytes,
  line_count as lines,
  (SELECT COUNT(*) FROM function_facts WHERE file = path) as functions,
  (SELECT COUNT(*) FROM import_facts WHERE file = path) as imports
FROM index_state
ORDER BY line_count DESC;

-- Importers (what imports X)
SELECT file, imported_names
FROM import_facts
WHERE import_path LIKE '%' || ? || '%';

-- Callers (what calls function X)
SELECT caller_file, caller_function
FROM call_graph
WHERE callee_function = ?;
```

---

## Exit Criteria

| Criteria | Status |
|----------|--------|
| `patina assay` returns module inventory | [x] |
| `patina assay imports <module>` works | [x] |
| `patina assay importers <module>` works | [x] |
| MCP `assay` tool exposed | [x] |
| Line counts in scrape output | [x] |
| Tested on Patina codebase | [x] |

---

## Phase 1: Snapshot Subcommand

**Purpose:** Complete "ore analysis" - holistic view combining structure + dependencies + origin + usage in one query. Designed for audit workflows where LLM needs full context.

### Motivation (Mining Analogy)

In metallurgy, an **assay** is a complete analysis of ore:
- What's in it (composition)
- How pure it is (quality)
- What it's worth (value)

Current assay gives composition (inventory) and relationships (imports/callers). Missing:
- **Origin** - When/why was this added? (git archaeology)
- **Usage** - Is it actually used? (importer count)
- **Holistic view** - All data in one query, not 6 separate calls

### CLI Interface

```bash
# Full snapshot of current project
patina assay snapshot

# Snapshot of reference repo
patina assay snapshot --repo dojo

# With markdown output for audit docs
patina assay snapshot --format markdown

# Filter to specific area
patina assay snapshot --pattern "src/commands/*"

# Include git origin data (slower - runs git log)
patina assay snapshot --with-origin
```

### Output Schema

```json
{
  "snapshot_time": "2025-12-19T08:30:00Z",
  "repo": "patina",
  "modules": [
    {
      "path": "src/retrieval/engine.rs",
      "lines": 353,
      "functions": 12,
      "imports": ["oracle", "fusion", "rayon"],
      "importer_count": 3,
      "importers": ["scry", "mcp/server", "eval"],
      "is_used": true,
      "top_callers": ["query_with_options ← scry::execute"],
      "origin": {
        "first_commit": "a1b2c3d",
        "first_commit_date": "2025-09-15",
        "first_commit_msg": "feat: add retrieval engine",
        "author": "user"
      },
      "purpose": null,
      "notes": null
    }
  ],
  "summary": {
    "total_files": 136,
    "total_lines": 28000,
    "total_functions": 1189,
    "unused_modules": 3,
    "largest_file": "src/commands/scrape/code/languages/typescript.rs"
  },
  "dependency_graph": {
    "src/main.rs": ["commands/*"],
    "src/commands/scry": ["retrieval", "embeddings"],
    "src/retrieval": ["storage", "embeddings"]
  }
}
```

### Markdown Format (--format markdown)

```markdown
## Module Inventory

| Module | Lines | Funcs | Importers | Used | Origin | Purpose | Notes |
|--------|-------|-------|-----------|------|--------|---------|-------|
| retrieval/engine.rs | 353 | 12 | 3 | ✓ | a1b2c3d (2025-09) | | |
| commands/scry/mod.rs | 1358 | 25 | 2 | ✓ | b2c3d4e (2025-08) | | |
| commands/belief/mod.rs | 100 | 4 | 0 | ? | c3d4e5f (2025-11) | | |

## Unused Modules (importer_count = 0)

| Module | Lines | First Commit | Message |
|--------|-------|--------------|---------|
| commands/belief/mod.rs | 100 | c3d4e5f | feat: add belief validation |

## Largest Files (>500 lines)

| Module | Lines | Functions | Notes |
|--------|-------|-----------|-------|
| commands/scrape/code/languages/typescript.rs | 1354 | 31 | |

## Dependency Summary

- **Most imported:** paths (9), environment (10), project (8)
- **Most importing:** main.rs (28), scry (12), init (15)
```

### Data Sources

| Field | Source | Automated |
|-------|--------|-----------|
| path, lines, functions | `index_state`, `function_facts` | ✅ |
| imports | `import_facts` | ✅ |
| importer_count, importers | `import_facts` GROUP BY | ✅ |
| is_used | importer_count > 0 | ✅ |
| top_callers | `call_graph` LIMIT 3 | ✅ |
| first_commit, author, msg | `git log --follow --diff-filter=A` | ✅ (with --with-origin) |
| purpose, notes | null (LLM fills) | ❌ Manual |

### Implementation

```
src/commands/assay/mod.rs:
  - Add Snapshot variant to QueryType
  - Add SnapshotOptions struct
  - Implement snapshot query combining all data sources
  - Add markdown formatter

src/commands/assay/snapshot.rs (new):
  - Git origin lookup (shell out to git log)
  - Dependency graph builder
  - Markdown table formatter
```

### MCP Tool Update

```json
{
  "query_type": {
    "enum": ["inventory", "imports", "importers", "functions",
             "callers", "callees", "snapshot"]
  },
  "format": {
    "type": "string",
    "enum": ["json", "markdown"],
    "default": "json"
  },
  "with_origin": {
    "type": "boolean",
    "default": false,
    "description": "Include git first-commit data (slower)"
  }
}
```

### Exit Criteria

| Criteria | Status |
|----------|--------|
| `patina assay snapshot` returns holistic view | [ ] |
| `--format markdown` produces audit-ready tables | [ ] |
| `--with-origin` includes git first-commit data | [ ] |
| MCP tool updated with snapshot query type | [ ] |
| Unused modules clearly flagged | [ ] |
| Tested on Patina + reference repo | [ ] |

---

## Out of Scope (Deferred)

- **Per-language module docs** - Requires tree-sitter extraction per language
- **Complexity metrics** - Cyclomatic complexity, etc.
- **Dead code detection** - Requires full reachability analysis (beyond importer count)
- **Purpose inference** - LLM annotation, not automated

---

## Usage Example

Before (40+ tool calls):
```
Bash: find src -name "*.rs" | head
Bash: wc -l src/retrieval/*.rs
Bash: grep "use crate::" src/retrieval/engine.rs
Read: src/retrieval/mod.rs
... repeat 40 times ...
```

After (1-3 tool calls):
```
MCP: assay(query_type="inventory")
MCP: assay(query_type="importers", pattern="retrieval")
```

Estimated 10-20x reduction in tool calls for structural exploration.
