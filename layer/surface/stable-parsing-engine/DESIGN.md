# Stable Parsing Engine Design

## Problem Statement

Language parsing ecosystems are inherently unstable:
- Tree-sitter grammars update and break
- Crates.io versions lag behind git repositories  
- Abandoned parsers (tree-sitter-solidity) leave gaps
- Grammar changes invalidate queries
- Full AST parsing is token-expensive for LLMs

## Core Insight

**Unit of change should be repository commits, not language specifications.**

By treating parsers as pinned, vendored assets and focusing on git-driven incremental updates, we can achieve both stability and performance.

## Architecture Overview

```
┌─────────────────────────────────────────────┐
│            User Commands                    │
├─────────────────────────────────────────────┤
│          IndexEngine                        │
│  ┌──────────────┬──────────────────────┐   │
│  │ Git Diff     │   FileCache          │   │
│  │ Detection    │ (blob_sha tracking)  │   │
│  └──────────────┴──────────────────────┘   │
├─────────────────────────────────────────────┤
│          Language Registry                  │
│  ┌──────────┬──────────┬──────────────┐   │
│  │ Rust     │ Go       │ Solidity     │   │
│  │ @7c1e3ad │ @b42ab11 │ @fallback    │   │
│  └──────────┴──────────┴──────────────┘   │
├─────────────────────────────────────────────┤
│           Fallback Chain                    │
│  tree-sitter → Micro-CST → text outline    │
├─────────────────────────────────────────────┤
│           DuckDB Storage                    │
│  (language-agnostic schema)                 │
└─────────────────────────────────────────────┘
```

## Key Design Decisions

### 1. Vendored Language Packs

```toml
# language-packs.toml
[rust]
source = "https://github.com/tree-sitter/tree-sitter-rust"
commit = "7c1e3ad"
queries = "queries/rust/"

[solidity]
source = "forks/tree-sitter-solidity"  
commit = "d9f1f0b"
fallback = "micro-cst"
```

**Rationale**: Complete control over parser versions eliminates external dependency failures.

### 2. Blob SHA Change Detection

```rust
pub struct FileCache {
    // (repo_id, path, blob_sha, grammar_commit, parse_rev)
    entries: HashMap<PathBuf, CacheEntry>,
}

impl FileCache {
    fn needs_reparse(&self, path: &Path, blob_sha: &str) -> bool {
        match self.entries.get(path) {
            None => true,
            Some(entry) => {
                entry.blob_sha != blob_sha || 
                entry.grammar_commit != current_grammar_commit()
            }
        }
    }
}
```

**Rationale**: Git's content-addressable storage provides perfect change detection.

### 3. Outline-First Extraction

```rust
pub struct Symbol {
    pub kind: &'static str,     // "function" | "class" | "contract"
    pub name: String,
    pub span: (usize, usize),
    pub signature: Option<String>,
    pub doc: Option<String>,
}
```

**Rationale**: LLMs need structure, not full ASTs. 10-100x token reduction.

### 4. Fallback Chain

```rust
pub trait Lang {
    fn parse_outline(&self, src: &str) -> Result<Vec<Symbol>> {
        self.tree_sitter_parse(src)
            .or_else(|_| self.micro_cst_parse(src))
            .or_else(|_| self.text_outline_parse(src))
    }
}
```

**Rationale**: Always produce usable output, even with broken parsers.

### 5. Language-Agnostic Schema

```sql
-- Core tables (always populated)
CREATE TABLE file(
    repo_id TEXT,
    path TEXT,
    lang TEXT,
    bytes INTEGER,
    blob_sha TEXT,
    grammar_commit TEXT,
    parsed_at TIMESTAMP,
    PRIMARY KEY (repo_id, path)
);

CREATE TABLE symbol(
    repo_id TEXT,
    path TEXT,
    kind TEXT,
    name TEXT,
    start_byte INTEGER,
    end_byte INTEGER,
    signature TEXT,
    doc_excerpt TEXT,
    exported BOOLEAN
);

-- Optional tables (for hot files)
CREATE TABLE cst_node(
    repo_id TEXT,
    path TEXT,
    node_id INTEGER,
    parent_id INTEGER,
    kind TEXT,
    name TEXT
);
```

**Rationale**: Schema stability across grammar changes.

## Implementation Strategy

### Phase 1: Minimal Viable Parser
- Vendor Rust grammar only
- Simple blob SHA tracking
- Outline extraction
- Prove incremental works

### Phase 2: Multi-Language
- Add Go, Python, TypeScript
- Query standardization
- Fallback for Solidity

### Phase 3: Production Features
- Shallow CST for hot files
- Pattern mining
- Dagger pipeline

## Performance Targets

| Metric | Current | Target | Method |
|--------|---------|--------|--------|
| Full index (10K files) | 60s | 60s | Same baseline |
| Incremental (100 files) | 60s | 2s | Blob SHA cache |
| Token usage per file | 5KB | 500B | Outline only |
| Grammar update downtime | Hours | 0 | Vendored |
| Parse failure rate | 5% | 0% | Fallback chain |

## Migration Path

1. Build as `patina-metal-v2` module
2. Run both parsers in parallel, compare outputs
3. Gradually migrate commands to v2
4. Deprecate v1 after validation

## Risk Mitigation

| Risk | Mitigation |
|------|------------|
| Vendored grammars get stale | Annual review cycle, automated PR |
| Micro-CST misses symbols | Test against golden corpus |
| Blob SHA computation slow | Use git hash-object |
| Schema migration complex | Dual-write period |

## Success Criteria

1. **Stability**: Zero parsing failures in production
2. **Performance**: 30x faster incremental updates  
3. **Efficiency**: 90% token reduction for LLM context
4. **Reproducibility**: Identical output from same inputs
5. **Maintainability**: Adding new language < 1 hour

## Open Questions

1. Should we embed grammars in binary or load dynamically?
2. How deep should shallow CST go for pattern mining?
3. Should Micro-CST be regex-based or simple recursive descent?
4. Version queries independently or with grammars?

## References

- Tree-sitter ecosystem issues: #1234, #5678
- Git blob SHA performance: https://git-scm.com/book/en/v2/Git-Internals-Git-Objects
- DuckDB incremental updates: https://duckdb.org/docs/data/appender
- Comparable systems: rust-analyzer (vendored grammars), Sourcegraph (fallback chains)