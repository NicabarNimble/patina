# Spec: Scry

## Overview

Scry is the unified query interface between LLMs and Patina's knowledge stores. It searches both project knowledge and personal beliefs, returning tagged results.

**Pipeline position:**
```
SQLite + Vectors → scry → Tagged Results → LLM Context
(project + persona)  (this)   ([PROJECT], [PERSONA])
```

## Query Flow

```
User: patina scry "error handling patterns"

1. Query project (.patina/data/)
   ├── Vector search: finds similar observations
   ├── SQLite: gets context, code refs
   └── Results: [PROJECT] "TypeScript prefers Result types"
                [PROJECT] "Effect library handles this"

2. Query persona (~/.patina/personas/default/)
   └── Results: [PERSONA] "I prefer Rust Result<T,E>"
                [PERSONA] "Always use explicit error types"

3. Combine & return:
   ┌─────────────────────────────────────────────┐
   │ PROJECT KNOWLEDGE (from livestore):         │
   │ • TypeScript prefers Result types here      │
   │ • Effect library handles this elegantly     │
   │                                             │
   │ YOUR BELIEFS (from persona):                │
   │ • You prefer Rust Result<T,E>               │
   │ • You like explicit error types             │
   └─────────────────────────────────────────────┘
```

## Components

### 1. Scry Command
**Location:** `src/commands/scry/mod.rs`

```rust
pub struct ScryResult {
    pub source: ResultSource,
    pub content: String,
    pub score: f32,
    pub metadata: ScryMetadata,
}

pub enum ResultSource {
    Project { name: String },
    Persona,
}

pub struct ScryMetadata {
    pub code_refs: Vec<String>,      // file:line references
    pub session_id: Option<String>,  // source session
    pub timestamp: Option<String>,
    pub domains: Vec<String>,        // ["rust", "embeddings"]
}

pub fn scry(query: &str, options: ScryOptions) -> Result<Vec<ScryResult>> {
    let mut results = Vec::new();

    // 1. Query current project
    if let Some(project_results) = query_project(query, &options)? {
        results.extend(project_results);
    }

    // 2. Query persona (if enabled)
    if options.include_persona {
        if let Some(persona_results) = query_persona(query, &options)? {
            results.extend(persona_results);
        }
    }

    // 3. Merge and rank
    results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
    results.truncate(options.limit);

    Ok(results)
}
```

### 2. Vector Search
**Location:** `src/commands/scry/search.rs`

```rust
pub fn vector_search(
    query: &str,
    index_path: &Path,
    embedding_model: &dyn EmbeddingModel,
    projection: Option<&Projection>,
    limit: usize,
) -> Result<Vec<(usize, f32)>> {
    // Embed query
    let query_vec = embedding_model.embed(query)?;

    // Apply projection if specified
    let search_vec = match projection {
        Some(proj) => proj.forward(&query_vec)?,
        None => query_vec,
    };

    // Search USearch index
    let index = usearch::Index::open(index_path)?;
    let results = index.search(&search_vec, limit)?;

    Ok(results)
}
```

### 2b. File-Based Search (Phase 3)
**Location:** `src/commands/scry/file_search.rs`

File-based queries skip embedding - the file's vector already exists in the index.

```rust
pub fn file_search(
    file_path: &str,
    index_path: &Path,
    db: &Connection,
    limit: usize,
) -> Result<Vec<(usize, f32)>> {
    // Look up file's index position
    let file_index = find_file_index(db, file_path)?;

    // Load index and get the file's existing vector
    let index = usearch::Index::open(index_path)?;
    let file_vector = index.get(file_index)?;

    // Search for neighbors (excluding self)
    let results = index.search(&file_vector, limit + 1)?;
    let filtered: Vec<_> = results.into_iter()
        .filter(|(idx, _)| *idx != file_index)
        .take(limit)
        .collect();

    Ok(filtered)
}

fn find_file_index(db: &Connection, file_path: &str) -> Result<u64> {
    // For temporal: files are indexed sequentially from co_changes
    let mut stmt = db.prepare(
        "WITH files AS (
            SELECT DISTINCT file_a as f FROM co_changes
            UNION SELECT DISTINCT file_b FROM co_changes
            ORDER BY 1
        )
        SELECT rowid - 1 FROM files WHERE f = ?"
    )?;
    stmt.query_row([file_path], |row| row.get(0))
}
```

**Why no embedding for file queries:**
- Temporal index: files indexed by path, vectors already computed
- Dependency index: functions indexed by name, vectors already computed
- Query = neighbor lookup, not new embedding

### 3. SQLite Metadata Lookup
**Location:** `src/commands/scry/metadata.rs`

```rust
pub fn enrich_with_metadata(
    db: &Connection,
    vector_results: &[(usize, f32)],
) -> Result<Vec<ScryResult>> {
    let mut enriched = Vec::new();

    for (id, score) in vector_results {
        // Look up in observations table
        let row = db.query_row(
            "SELECT content, session_id, observation_type, domains, code_refs, timestamp
             FROM observations WHERE id = ?",
            [id],
            |row| Ok(ObservationRow::from(row)),
        )?;

        enriched.push(ScryResult {
            source: ResultSource::Project { name: current_project_name()? },
            content: row.content,
            score: *score,
            metadata: ScryMetadata {
                code_refs: serde_json::from_str(&row.code_refs)?,
                session_id: Some(row.session_id),
                timestamp: Some(row.timestamp),
                domains: serde_json::from_str(&row.domains)?,
            },
        });
    }

    Ok(enriched)
}
```

### 4. Result Formatting
**Location:** `src/commands/scry/format.rs`

```rust
pub fn format_for_llm(results: &[ScryResult]) -> String {
    let mut output = String::new();

    // Group by source
    let (project_results, persona_results): (Vec<_>, Vec<_>) =
        results.iter().partition(|r| matches!(r.source, ResultSource::Project { .. }));

    if !project_results.is_empty() {
        output.push_str("## Project Knowledge\n\n");
        for result in project_results {
            output.push_str(&format!("- {}", result.content));
            if !result.metadata.code_refs.is_empty() {
                output.push_str(&format!(" ({})", result.metadata.code_refs.join(", ")));
            }
            output.push('\n');
        }
        output.push('\n');
    }

    if !persona_results.is_empty() {
        output.push_str("## Your Beliefs\n\n");
        for result in persona_results {
            output.push_str(&format!("- {}\n", result.content));
        }
    }

    output
}
```

## CLI

```bash
# Text queries (semantic dimension)
patina scry "error handling"              # Search project + persona
patina scry "error handling" --limit 20   # More results (default 10)
patina scry "error handling" --json       # JSON output for tooling

# File queries (temporal/dependency dimensions) - Phase 3
patina scry --file src/auth/login.rs      # What files co-change with this?
patina scry --file Game.cairo --dimension temporal
patina scry --file spawn.rs --dimension dependency  # What calls/is called by?

# Exact match (FTS5 lexical) - Phase 3
patina scry "find COMPONENT_ID"           # Auto-detects exact match pattern
patina scry "where is spawn_entity"       # Routes to FTS5

# Cross-project (via Mothership) - Phase 3
patina scry "spawn patterns" --projects dojo,bevy

# Filters
patina scry "error handling" --project    # Project only
patina scry "error handling" --persona    # Persona only
patina scry "error handling" --dimension semantic
```

## Options

```rust
pub struct ScryOptions {
    pub include_persona: bool,       // default: true
    pub include_project: bool,       // default: true
    pub limit: usize,                // default: 10
    pub dimension: Option<String>,   // e.g., "semantic", "temporal", "dependency"
    pub output_format: OutputFormat, // Text, Json, LlmContext
    pub min_score: f32,              // default: 0.5
    pub file: Option<String>,        // Phase 3: file-based queries
    pub projects: Option<Vec<String>>, // Phase 3: cross-project via mothership
}
```

## Integration with Mothership

When mothership is running, scry can query multiple projects:

```bash
# Direct CLI (current project only)
patina scry "error handling"

# Via mothership (cross-project)
curl -X POST http://localhost:50051/scry \
  -H "Content-Type: application/json" \
  -d '{"query": "error handling", "projects": ["livestore", "patina"]}'
```

**Mothership endpoint:**
```rust
// POST /scry
pub async fn scry_handler(Json(req): Json<ScryRequest>) -> Json<ScryResponse> {
    let mut all_results = Vec::new();

    for project in &req.projects {
        let project_path = registry.get(project)?;
        let results = scry(&req.query, project_path, &req.options)?;
        all_results.extend(results);
    }

    // Always include persona
    let persona_results = query_persona(&req.query)?;
    all_results.extend(persona_results);

    // Merge, rank, return
    all_results.sort_by_score();
    Json(ScryResponse { results: all_results })
}
```

## Prolog Integration (Optional)

For complex reasoning queries, scry can invoke Prolog:

```bash
patina scry "what calls foo and is called by bar" --reason
```

```rust
// If query contains relational patterns, use Prolog
pub fn maybe_use_prolog(query: &str, db: &Connection) -> Option<Vec<ScryResult>> {
    if looks_like_relational_query(query) {
        // Translate to Prolog query
        // calls(X, foo), calls(bar, X)
        let prolog_results = prolog_engine.query(&translated)?;
        return Some(prolog_results);
    }
    None
}
```

This is a future enhancement, not required for v1.

## File Structure

```
src/commands/scry/
├── mod.rs           # Main scry command
├── search.rs        # Vector search logic
├── metadata.rs      # SQLite enrichment
├── format.rs        # Output formatting
└── prolog.rs        # Optional reasoning (future)
```

## Acceptance Criteria

### MVP (Phase 2.5b) ✅
- [x] `patina scry "query"` returns relevant results
- [x] Vector search uses correct projection (semantic/temporal)
- [x] SQLite metadata enriches results (event_type, source_id, content)
- [x] Results sorted by relevance score
- [x] `--limit` and `--min-score` options work
- [x] `--dimension` selects semantic or temporal index

### Phase 3: Hackathon MVP
- [ ] `--file` flag for file-based queries (temporal, dependency)
- [ ] File lookup without re-embedding (direct neighbor search)
- [ ] FTS5 integration for exact match queries
- [ ] Auto-detect query type (text vs file vs exact match)
- [ ] `--projects` flag for cross-project queries via mothership

### Phase 4+
- [ ] Results tagged as [PROJECT] or [PERSONA]
- [ ] `--json` outputs structured data
- [ ] `--project` / `--persona` filters work
- [ ] Integrates with mothership `/scry` endpoint
