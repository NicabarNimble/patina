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

## Solution: Random UID in Identity File

```
.patina/
├── uid                    # Identity (permanent)
└── data/
    └── patina.db          # Data (ephemeral, can rebuild)
```

**Simple rules:**
1. UID is random 32-bit hex (8 chars)
2. Generated once at `patina init`
3. Stored in `.patina/uid` file (not in database)
4. Never changes, survives rebuild

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

## Why File (Not In Database)

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
    uid TEXT PRIMARY KEY,     -- "550e8400"
    name TEXT NOT NULL,       -- "claude-code" (human display)
    path TEXT NOT NULL,       -- location (can change)
);

CREATE TABLE edges (
    from_uid TEXT NOT NULL,
    to_uid TEXT NOT NULL,
    edge_type TEXT NOT NULL,
);
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

## What We Dropped

Earlier versions of this spec considered:
- Hash of canonical identity (GitHub URL, git SHA, path)
- Fallback chains for different scenarios
- UID stored in database `_meta` table

All dropped in favor of simpler random UID in file.

---

## References

- [spec-mothership-graph.md](spec-mothership-graph.md) - Graph routing using UIDs
- Session 20260112-061237 - Design discussion and simplification
