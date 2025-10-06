---
id: distributed-knowledge-nodes
status: exploration
created: 2025-01-21
tags: [architecture, duckdb, semantic, local-first, decentralized]
references: [semantic-reality-system]
---

# Distributed Knowledge Nodes - A Network of Domain Experts

**Core Idea**: Every repo/project becomes a domain with its own database. Each domain is like "hey I'm birds, I know everything about birds" or "I'm patina, I know everything about patina" or "I'm godot, I know everything about godot" or even "I'm Waiting for Godot, a play by Samuel Beckett". They're all these DB nodes and patina (the program) is how you feed them, connect them, and use them.

---

## Database Structure Concept

Each database has two internal layers:
- **code** - The actual implementation, what the code does
- **layers** - The documentation (we call docs "layers" in our DB)

The DB is semantic and references files for truth or expanded info - it's not storing full text, it's smaller. The DBs live with their files.

## Layers Reorganization

Thinking about making `layers` a standalone submodule in any project - it becomes the "knowledge node":

- **core** - Refined, permanent knowledge (sessions move here from raw)
- **surface** - Active development 
- **dust** - Local archive (NOT git tracked) where we move things
- **veins** - NEW: Connections to other DBs/nodes

Git tracks all docs (but what about assets like PDFs and images?) AND DB tracking/management.

## Sync and Distribution Ideas

Originally planned sqlite + automerge (replacing rqlite) for local-first with sync capability. Now thinking these "others" we sync with are DB nodes. 

Moved to DuckDB (really liking it, on track to replace SQLite). Automerge is interesting BUT too academic/complex for these goals - too meta.

The network could be:
- All on a single computer (bunch of DBs living with their files)
- Different computers on a LAN
- Across the internet

Need to make it not matter where they are. Probably needs something like `git clone` where you can:
- Pull down a DB and store it locally
- Sans-files version that pulls files as needed (some intelligence there)
- If local, just connecting like a web of folders

## Patina Scrape Focus

`patina scrape` is for cataloging information about a codebase in a concrete and semantic way:
- Scrapes code right now (docs coming soon)
- Creates a useful semantic DB so an LLM can query efficiently (better token usage, faster learning)

Implementation approach:
- Internal: scrape current project
- External: point at a folder (like `layer/dust/repo` where I have repos)
- Creates lightweight semantic DBs for each

How we connect all these scraped DBs happens later - first just get the scraping and DB creation working.

## The Dogfood Paradox

Patina is building itself - it's self-aware. The internal node knows about building Patina.

## Decentralized & Local-First

These are the guiding north stars:
- Each node maintains its own truth
- Works offline
- Syncs when possible
- No central authority

## Database Design Considerations

### The Size Problem

Looking at what others are doing with LLM memory:
- **Context7/Upstash**: Uses vector embeddings (semantic search)
- **Mem0**: Graph-based memory with embeddings
- **LangChain**: VectorStore-backed memory

But vector embeddings are HUGE:
- Structural facts: ~80 bytes per function
- With embeddings: ~3,150 bytes per function (39x bigger!)
- For 10,000 functions: 800KB vs 31.5MB

This feels like bloatware when JS13K games fit entire games in 13KB.

### The Lean Approach

Instead of vector embeddings, considering:

1. **SQLite FTS5 + BM25** - Full-text search, no vectors
   - 90% as good as embeddings for code search
   - Basically no size overhead

2. **Structural Fingerprints** - 16 bytes captures what matters
   ```
   pattern: u32      // Hash of AST shape
   imports: u32      // Hash of dependencies  
   complexity: u16   // Cyclomatic complexity
   flags: u16        // Bitmask of features
   ```

3. **Compressed Patterns** - Like `.cursorrules` files
   - Just text descriptions
   - Few KB tells LLM everything

4. **Trigram Search** - Built into PostgreSQL/SQLite
   - Beats embeddings for most code searches
   - Fuzzy matching without the bloat

### Proposed Minimal Schema

```sql
-- Tiny structural table (10KB total)
CREATE TABLE code_facts (
    id INTEGER PRIMARY KEY,
    path TEXT,
    sig TEXT,  -- "fn execute() -> Result"
    flags INTEGER  -- Bitmask of features
);

-- Full-text search (built into DuckDB/SQLite)
CREATE VIRTUAL TABLE code_fts USING fts5(
    path, content
);

-- Pattern rules (100 bytes)
CREATE TABLE patterns (
    name TEXT,
    rule TEXT  -- "has Result + has context = good"
);
```

Total DB size: **Under 100KB** for most projects

### The Tradeoff

**Small DB (Structural)**:
- ✅ Tiny (KB not MB)
- ✅ Fast to generate
- ✅ No dependencies
- ✅ Can git track it
- ❌ Can't do "find similar code"
- ❌ Limited to exact/fuzzy text matches

**Big DB (Embeddings)**:
- ❌ 30-40x larger
- ❌ Slower to generate
- ❌ Needs ML model (500MB-2GB)
- ❌ Too big for git
- ✅ Semantic search ("code like this")
- ✅ Cross-language understanding

### Current Direction

Starting with the lean approach - no vectors, no bloat, just smart compression and structural analysis. Can always add embeddings later for specific use cases, but for now keeping it simple and small.

The goal: Make something that works like those JS13K games - every byte counts, compress aggressively, leverage what the LLM already knows instead of teaching it everything from scratch.

---

*This is all old tech really, just thinking out loud about how to wire it together in a useful way.*