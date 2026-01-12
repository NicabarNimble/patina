# Spec: Database Identity (UIDs for Federation)

**Status:** Design
**Created:** 2026-01-12
**Session:** 20260112-061237
**Core References:** [dependable-rust](../../core/dependable-rust.md), [unix-philosophy](../../core/unix-philosophy.md)

---

## Problem Statement

Patina databases are **anonymous** - they don't know who they are. Identity is assigned externally by filesystem path (fragile) or registry name (can change).

This breaks federation:
- Cross-DB references have no stable target
- Mother graph edges break on path/name changes
- Can't trace where federated results came from

**Root cause:** Databases lack intrinsic identity.

---

## Solution: UID File + DB Generation

```
.patina/
├── uid                    # Project identity (permanent)
└── data/
    └── patina.db          # Data (ephemeral, tracks generation)
```

**Two-layer identity:**

| Layer | Location | Purpose |
|-------|----------|---------|
| Project UID | `.patina/uid` file | Identifies the project (permanent) |
| DB Generation | `patina.db._meta` | Tracks rebuild count (increments) |

Full reference: `550e8400:3` = "project 550e8400, generation 3"

---

## UID Generation Strategy (Hybrid)

Different sources get different UID strategies:

| Source | UID Strategy | Rationale |
|--------|--------------|-----------|
| Local project | Random | No canonical external identity |
| GitHub ref repo | Hash of `github:owner/repo` | Deterministic, shared across users |
| GitLab ref repo | Hash of `gitlab:owner/repo` | Same logic |
| Other forges | Hash of source URL | Extensible |

**Why hybrid:**
- Local projects have no shared identity → random is fine
- Ref repos have canonical source → hash enables "find all indexes of this repo"
- Two users indexing `anthropics/claude-code` get **same UID**

```rust
fn compute_uid(source: &RepoSource) -> String {
    match source {
        RepoSource::Local => format!("{:08x}", fastrand::u32(..)),
        RepoSource::GitHub { owner, repo } =>
            hash_uid(&format!("github:{}/{}", owner, repo)),
        RepoSource::GitLab { owner, repo } =>
            hash_uid(&format!("gitlab:{}/{}", owner, repo)),
    }
}

fn hash_uid(canonical: &str) -> String {
    let hash = sha256(canonical.as_bytes());
    format!("{:08x}", u32::from_be_bytes(hash[..4].try_into().unwrap()))
}
```

---

## Design Principles

### From Dependable Rust

> "Black box modules with small, stable interfaces"

- UID is ONE thing: 8 hex chars
- Interface is minimal: `get_uid()`
- No external dependencies, no edge cases

### From Unix Philosophy

> "One tool, one job, done well"

- UID identifies the project, nothing else
- Separate concerns: identity (uid file) vs data (patina.db)
- Plain text file - can inspect with `cat`

---

## Why Random (Not Hash)

We considered hashing canonical identity (GitHub URL, path, etc.):

| Approach | Problem |
|----------|---------|
| Hash of GitHub URL | What if no GitHub? What if URL changes? |
| Hash of git SHA | What if no git? Empty repos have no commits |
| Hash of path | Path changes break identity |

**All add complexity and edge cases.**

Random UID means:
- No external dependencies
- No fallback chains
- No "what if X changes" questions
- Just works

---

## Why UID in File (Not In Database)

Database can be rebuilt (`patina rebuild`). If UID is in database:
- Rebuild deletes database
- UID is lost
- Mother graph has stale reference

With UID in separate file:
- Rebuild deletes `patina.db`
- `.patina/uid` survives
- Identity is preserved

**The UID identifies the project, not the database file.**

---

## DB Generation (Rebuild Tracking)

The database tracks its own generation count:

```sql
-- Inside patina.db
CREATE TABLE IF NOT EXISTS _meta (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

-- Set at creation, increment on rebuild
INSERT INTO _meta (key, value) VALUES ('generation', '1');
```

**On rebuild:**
```rust
fn rebuild_database(project_path: &Path) -> Result<()> {
    let old_gen = get_generation(&conn)?;  // Read before delete

    fs::remove_file(db_path)?;             // Delete old DB
    let conn = create_database(db_path)?;  // Create fresh

    set_generation(&conn, old_gen + 1)?;   // Increment generation
    // ... re-scrape ...
}
```

**Use case:** Mother graph tracks `last_indexed_generation` to detect stale caches.

---

## 32-bit UID (8 hex chars)

```
550e8400
```

**Why 32-bit:**
- 1% collision at ~9,300 projects per user
- Multi-user adds namespace: `{user}:{db}` = 64-bit effective
- Matches GitHub model: unique per owner, not globally

---

## Implementation

### File Format

```bash
cat .patina/uid
550e8400
```

Plain text, one line, 8 hex characters. No JSON, no metadata.

### Init Logic

```rust
fn ensure_uid(project_path: &Path) -> Result<String> {
    let uid_path = project_path.join(".patina/uid");

    if uid_path.exists() {
        return Ok(fs::read_to_string(&uid_path)?.trim().to_string());
    }

    // Generate random 32-bit UID
    let uid = format!("{:08x}", fastrand::u32(..));
    fs::create_dir_all(uid_path.parent().unwrap())?;
    fs::write(&uid_path, &uid)?;

    Ok(uid)
}
```

### Rebuild Behavior

```bash
patina rebuild
# Deletes: .patina/data/patina.db
# Keeps:   .patina/uid
# Result:  Same identity, fresh data
```

---

## Federation Integration

### Mother Graph

```sql
CREATE TABLE nodes (
    uid TEXT PRIMARY KEY,           -- "550e8400"
    name TEXT NOT NULL,             -- "claude-code" (human display)
    path TEXT NOT NULL,             -- location (can change)
    last_indexed_generation INT,    -- track staleness
    last_indexed_at TEXT,           -- when we last indexed
);

CREATE TABLE edges (
    from_uid TEXT NOT NULL,
    to_uid TEXT NOT NULL,
    edge_type TEXT NOT NULL,
);
```

### Staleness Detection

```rust
fn is_stale(node: &Node, project_path: &Path) -> Result<bool> {
    let current_gen = get_generation_from_db(project_path)?;
    Ok(current_gen > node.last_indexed_generation)
}

// On federated query:
// 1. Check if node is stale
// 2. If stale, re-index before querying (or mark results as potentially stale)
```

### Cross-DB References

```sql
-- In patina's eventlog
INSERT INTO eventlog (event_type, data) VALUES (
  'reference.pattern',
  '{"source_uid": "550e8400", "source_seq": 12345}'
);
```

### Federated Results

```
scry "error handling"

Results:
  [550e8400] src/errors.ts:45 - ErrorBoundary pattern
  [a3f2b1c4] src/forge/sync.rs:120 - Result<T> pattern
```

---

## Phases

### Phase 1: Add Identity

1. Create `.patina/uid` at `patina init`
2. Read UID in commands that need it
3. `patina doctor` shows UID

### Phase 2: Mother Graph Integration

1. Mother graph uses UID as primary key
2. Edges reference UIDs, not names/paths
3. Path changes don't break graph

### Phase 3: Cross-DB References

1. Eventlog can store `source_uid`
2. Scry results include source UID
3. Provenance tracking enabled

---

## Success Criteria

| Phase | Criteria |
|-------|----------|
| 1 | `.patina/uid` created at init |
| 1 | UID survives rebuild |
| 1 | `patina doctor` shows UID |
| 2 | Mother graph uses UIDs |
| 3 | Scry results show source UID |

---

## Design Evolution

Earlier versions considered:
- Pure random UID for everything → lost shared identity for ref repos
- Pure hash of canonical → edge cases for local projects
- UID in database only → lost on rebuild

**Final hybrid approach:**
- Local projects: random UID (no canonical source)
- Ref repos: hash of source URL (shared identity)
- UID in file (survives rebuild)
- Generation in DB (tracks rebuilds)

---

## References

- [spec-mothership-graph.md](spec-mothership-graph.md) - Graph routing using UIDs
- Session 20260112-061237 - Design discussion and simplification
