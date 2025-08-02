---
id: indexer-design
version: 2
status: draft
created_date: 2025-08-02
updated_date: 2025-08-02
oxidizer: nicabar
tags: [architecture, indexer, navigation, semantic-search, rqlite]
---

# Patina Indexer Design

The indexer is Patina's navigation system - it tells LLMs where to find knowledge in our layer structure.

## Core Purpose

When an LLM asks "How do we handle authentication?", the indexer responds with:
```yaml
navigation:
  core:
    - path: authentication-pattern.md
      relevance: "Current JWT implementation"
  surface: 
    - path: auth/passkey-experiment.md
      relevance: "Exploring passwordless auth"
  dust:
    - path: archived/redis-sessions.md
      relevance: "Previous session storage approach"
```

## Architecture

### Storage Architecture: Memory + rqlite

```
┌─────────────────────┐
│  NavigationMap      │ ← In-memory cache for fast queries
│  (HashMap cache)    │
└─────────┬───────────┘
          │ Sync on changes
          ▼
┌─────────────────────┐
│     rqlite DB       │ ← Persistent storage, source of truth
│  (HTTP + SQLite)    │
└─────────────────────┘
```

### 1. Navigation Map (In-Memory Cache)
```rust
pub struct NavigationMap {
    // Concept → Location mapping (cached from DB)
    concepts: HashMap<String, Vec<Location>>,
    // Document metadata cache
    documents: HashMap<String, DocumentInfo>,
    // Relationship graph
    relationships: Graph<String, RelationType>,
    // Track cache state
    last_refresh: Instant,
}

pub struct Location {
    pub layer: Layer,
    pub path: PathBuf,
    pub relevance: String,
    pub confidence: Confidence,
}
```

### 2. Database Schema (rqlite)
```sql
-- Document registry
CREATE TABLE documents (
    id TEXT PRIMARY KEY,
    path TEXT NOT NULL,
    layer TEXT NOT NULL,
    title TEXT,
    summary TEXT,
    status TEXT,
    metadata JSON,
    last_indexed TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Concept mapping
CREATE TABLE concepts (
    id INTEGER PRIMARY KEY,
    concept TEXT NOT NULL,
    document_id TEXT NOT NULL,
    relevance TEXT,
    confidence REAL DEFAULT 1.0,
    FOREIGN KEY (document_id) REFERENCES documents(id)
);

-- Document relationships
CREATE TABLE relationships (
    from_doc TEXT NOT NULL,
    to_doc TEXT NOT NULL,
    relationship_type TEXT NOT NULL,
    reason TEXT,
    PRIMARY KEY (from_doc, to_doc, relationship_type),
    FOREIGN KEY (from_doc) REFERENCES documents(id),
    FOREIGN KEY (to_doc) REFERENCES documents(id)
);

-- Query patterns
CREATE TABLE query_patterns (
    id INTEGER PRIMARY KEY,
    query_pattern TEXT NOT NULL,
    document_id TEXT NOT NULL,
    relevance_score REAL DEFAULT 1.0,
    FOREIGN KEY (document_id) REFERENCES documents(id)
);

-- Indexes for fast lookup
CREATE INDEX idx_concepts_concept ON concepts(concept);
CREATE INDEX idx_concepts_doc ON concepts(document_id);
CREATE INDEX idx_relationships_from ON relationships(from_doc);
CREATE INDEX idx_relationships_to ON relationships(to_doc);
CREATE INDEX idx_documents_layer ON documents(layer);
```

### 3. Document Enrichment
```rust
pub trait DocumentEnricher {
    // Analyze doc and update its metadata
    fn enrich(&self, doc_path: &Path) -> Result<Enrichment>;
    
    // Update markdown with discovered relationships
    fn update_document(&self, doc_path: &Path, enrichment: &Enrichment) -> Result<()>;
}

pub struct Enrichment {
    pub discovered_concepts: Vec<String>,
    pub related_documents: Vec<(String, String)>, // (doc_id, reason)
    pub potential_queries: Vec<String>,
    pub cross_references: Vec<CrossRef>,
}
```

### 4. Change Detection
```rust
pub struct ChangeWatcher {
    // Track file state
    file_hashes: HashMap<PathBuf, u64>,
    // Track document identity (survives renames)
    id_to_path: HashMap<String, PathBuf>,
}

pub enum Change {
    NewDocument(PathBuf),
    Modified(PathBuf),
    Renamed { old: PathBuf, new: PathBuf },
    LayerMove { path: PathBuf, from: Layer, to: Layer },
}
```

### 5. Query Interface with Cache + DB
```rust
pub struct PatternIndexer {
    // In-memory cache for fast queries
    cache: NavigationMap,
    // rqlite client for persistence
    db: RqliteClient,
}

impl PatternIndexer {
    // Initialize from database
    pub async fn new(db_url: &str) -> Result<Self> {
        let db = RqliteClient::new(db_url)?;
        let cache = Self::load_cache_from_db(&db).await?;
        Ok(Self { cache, db })
    }
    
    // Main navigation query (memory-first)
    pub fn navigate(&self, query: &str) -> NavigationResponse {
        // Query in-memory cache (microseconds)
        self.cache.navigate(query)
    }
    
    // Find documents by concept (memory-first)
    pub fn find_by_concept(&self, concept: &str) -> Vec<Location> {
        self.cache.find_concept(concept)
    }
    
    // Update operations (memory + persist)
    pub async fn index_document(&mut self, path: &Path) -> Result<()> {
        // 1. Analyze document
        let info = self.analyze_document(path)?;
        
        // 2. Update memory cache
        self.cache.insert_document(&info);
        
        // 3. Persist to rqlite
        self.db.insert_document(&info).await?;
        
        Ok(())
    }
    
    // Refresh from database
    pub async fn refresh(&mut self) -> Result<Vec<Change>> {
        let changes = self.detect_changes()?;
        self.cache = Self::load_cache_from_db(&self.db).await?;
        Ok(changes)
    }
}

pub struct NavigationResponse {
    pub query: String,
    pub interpreted_concepts: Vec<String>,
    pub locations: BTreeMap<Layer, Vec<LocationInfo>>,
    pub confidence: f32,
    pub cache_hit: bool,  // For monitoring
}
```

## Implementation Plan

### Phase 1: Foundation (Memory + rqlite)
1. **Set up rqlite** - Local instance for development
2. **Create schema** - Tables and indexes
3. **Build cache structures** - NavigationMap in memory
4. **Sync mechanism** - Load from DB, persist changes

### Phase 2: Basic Navigation
1. **Scan layer structure** - Index all markdown files
2. **Extract metadata** - Parse YAML frontmatter
3. **Build concept index** - Extract and store concepts
4. **Query interface** - Memory-first navigation

### Phase 3: Enrichment
1. **Discover relationships** - Analyze content connections
2. **Update documents** - Add cross-references
3. **Generate queries** - Example questions per doc
4. **Track changes** - File monitoring

### Phase 4: Advanced Features
1. **Semantic embeddings** - Vector search preparation
2. **Query learning** - Improve based on usage
3. **Cross-project index** - Share wisdom
4. **Performance optimization** - Cache tuning

## Key Design Decisions

### 1. LLM as Co-Indexer
- When LLMs create documents, they also create index entries
- Self-describing patterns with semantic metadata
- Human-readable yet machine-processable

### 2. Layer-Aware Navigation
- Core results shown first (implemented truth)
- Surface results next (active exploration)
- Dust results last (historical context)

### 3. Identity Persistence
- Documents tracked by ID, not filename
- Survives renames and moves
- Maintains relationship integrity

### 4. Progressive Enhancement
- Start with simple keyword mapping
- Add semantic understanding over time
- Never lose basic functionality

## Example Usage

```rust
// Initialize indexer with rqlite
let mut indexer = PatternIndexer::new("http://localhost:4001").await?;

// Fast navigation query (hits memory cache)
let response = indexer.navigate("How to implement faster login?");
assert!(response.cache_hit);  // Served from memory

// Response guides LLM:
// - Check core/authentication-pattern.md first
// - Then surface/auth/performance-experiment.md
// - Consider dust/archived/redis-cache.md for historical context

// Index a new document (updates cache + DB)
indexer.index_document("surface/auth/webauthn.md").await?;
// → Analyzes content
// → Updates memory cache immediately
// → Persists to rqlite
// → Enriches related documents

// Handle rename (maintains consistency)
indexer.handle_rename("old-auth.md", "authentication-pattern.md").await?;
// → Updates memory cache
// → Updates database
// → Maintains document identity
// → Updates all references

// Refresh from database (e.g., after external changes)
let changes = indexer.refresh().await?;
println!("Detected {} changes", changes.len());
```

## Success Criteria

1. **Fast navigation** - Sub-second query response
2. **Accurate locations** - Find relevant docs reliably
3. **Relationship awareness** - Understand doc connections
4. **Change resilience** - Handle renames/moves gracefully
5. **LLM-friendly** - Clear, actionable navigation results

## Future Considerations

- **Semantic search** - Vector embeddings for concept similarity
- **Cross-project index** - Share wisdom between projects
- **Pattern templates** - Generate new docs with proper indexing
- **Query learning** - Improve based on which results get used