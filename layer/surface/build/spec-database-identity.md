# Spec: Database Identity (UIDs for Federation)

**Status:** Design (blocked)
**Created:** 2026-01-12
**Updated:** 2026-01-12
**Sessions:** 20260112-061237 (initial), 20260112-093636 (explicit patterns)
**Blocked By:** [spec-init-hardening.md](spec-init-hardening.md) - init must preserve config before adding UID
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
- Interface is minimal: `create_uid()` and `read_uid()`
- Explicit over implicit: creation and reading are separate operations
- No hidden side effects: reads don't write, creates don't silently succeed

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

### UID Operations (Explicit Create/Read)

**Principle:** Explicit over implicit. No `ensure_uid()` magic - creation and reading are separate.

```rust
/// Create UID at init time. Fails if already exists (idempotent re-init is separate).
fn create_uid(project_path: &Path, source: &RepoSource) -> Result<String> {
    let uid_path = project_path.join(".patina/uid");

    if uid_path.exists() {
        // Re-init case: read existing, don't overwrite
        return read_uid(project_path);
    }

    let uid = compute_uid(source);
    fs::create_dir_all(uid_path.parent().unwrap())?;
    fs::write(&uid_path, &uid)?;
    Ok(uid)
}

/// Read UID. Fails if missing - caller must handle upgrade path.
fn read_uid(project_path: &Path) -> Result<String> {
    let uid_path = project_path.join(".patina/uid");

    if !uid_path.exists() {
        return Err(anyhow!(
            "No UID found. Run `patina init .` to upgrade this project."
        ));
    }

    Ok(fs::read_to_string(&uid_path)?.trim().to_string())
}
```

**Why separate functions (Jon Gjengset / Eskil Steenberg principle):**
- `read_uid()` never writes - no surprising side effects
- `create_uid()` is called at ONE place (init) - single point of control
- Failures are explicit - "no UID" is a clear error, not silent creation
- Testing is straightforward - no hidden state changes

### Rebuild Behavior

```bash
patina rebuild
# Deletes: .patina/data/patina.db
# Keeps:   .patina/uid
# Result:  Same identity, fresh data
```

---

## Edge Cases

### Manual Database Deletion

If user runs `rm .patina/data/patina.db` directly (not `patina rebuild`):
- Generation counter resets to 1
- Mother graph may have stale `last_indexed_generation`

**Decision:** Accept this edge case.
- Manual deletion is explicit user intent
- Cleverness can come from mother graph later (detect generation regression)
- Keep this spec simple

### UID Collision Detection

With 32-bit UIDs, collision probability is ~1% at 9,300 projects. When registering a project in mother graph:

```rust
fn register_in_mother(uid: &str, path: &Path) -> Result<()> {
    if let Some(existing) = mother.get_node(uid)? {
        if existing.path != path {
            warn!(
                "UID collision detected: {} already registered at {}",
                uid, existing.path
            );
            // Log to mother system logs for future analysis
            mother.log_event("uid_collision", json!({
                "uid": uid,
                "existing_path": existing.path,
                "new_path": path,
            }))?;
        }
    }
    // Proceed with registration (last-write-wins for now)
    mother.upsert_node(uid, path)?;
    Ok(())
}
```

**Decision:** Detect and warn, don't block.
- Log collision events for future analysis
- Mother graph can implement smarter resolution later
- User can manually regenerate UID if needed (`rm .patina/uid && patina init .`)

---

## Migration (Upgrading Old Projects)

Old `.patina` directories without `uid` file need explicit upgrade.

### Behavior by Command

| Command | Old Project (no UID) | New Project (has UID) |
|---------|---------------------|----------------------|
| `patina init .` | Creates UID | Reads existing UID |
| `patina scrape` | **Fails with upgrade message** | Works normally |
| `patina rebuild` | **Fails with upgrade message** | Works normally |
| `patina doctor` | Shows "UID: missing" | Shows UID |

### Error Message

```
Error: No UID found for this project.

This project was created before UID support. Run:

    patina init .

This will add a UID without affecting your existing data.
```

### Why Explicit Migration

- **No magic:** Commands don't silently create files
- **User awareness:** User knows their project was upgraded
- **Single entry point:** `patina init` is the ONE place UIDs are created
- **Predictable:** `read_uid()` either succeeds or fails, never writes

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

1. Add `create_uid()` and `read_uid()` functions (explicit, separate)
2. Create `.patina/uid` at `patina init` (local=random, ref=hash)
3. Commands that need UID call `read_uid()` - fail with upgrade message if missing
4. `patina doctor` shows UID (or "missing - run patina init .")
5. `patina init .` on old project creates UID (migration path)

### Phase 2: Mother Graph Integration

1. Mother graph uses UID as primary key
2. Collision detection on registration (warn + log, don't block)
3. Edges reference UIDs, not names/paths
4. Path changes don't break graph

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
| 1 | Old projects fail with clear upgrade message |
| 1 | `patina init .` upgrades old projects |
| 1 | `patina doctor` shows UID |
| 2 | Mother graph uses UIDs |
| 2 | Collision detected and logged |
| 3 | Scry results show source UID |

---

## Design Evolution

Earlier versions considered:
- Pure random UID for everything → lost shared identity for ref repos
- Pure hash of canonical → edge cases for local projects
- UID in database only → lost on rebuild
- `ensure_uid()` lazy creation → hidden side effects, magic

**Final approach:**
- Local projects: random UID (no canonical source)
- Ref repos: hash of source URL (shared identity)
- UID in file (survives rebuild)
- Generation in DB (tracks rebuilds)
- **Explicit create/read** (no `ensure_uid()` magic)
- **Explicit migration** (require `patina init .` for old projects)
- **Collision detection** (warn, log, don't block)

---

## References

- [spec-mothership-graph.md](spec-mothership-graph.md) - Graph routing using UIDs
- Session 20260112-061237 - Initial design discussion and simplification
- Session 20260112-093636 - Explicit create/read pattern, migration strategy, collision handling
