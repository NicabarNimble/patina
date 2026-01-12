# Spec: Database Identity (UIDs for Federation)

**Status:** Design
**Created:** 2026-01-12
**Session:** 20260112-061237
**Core References:** [dependable-rust](../../core/dependable-rust.md), [unix-philosophy](../../core/unix-philosophy.md), [adapter-pattern](../../core/adapter-pattern.md)

---

## Problem Statement

Patina databases are currently **anonymous** - they don't know who they are. Identity is assigned externally by:
- Filesystem path (fragile, breaks on move)
- Registry name (can change, not intrinsic)
- Mother graph node ID (external assignment)

This creates problems for federation:

| Problem | Current State | Impact |
|---------|---------------|--------|
| Cross-DB references | Can't reference data in other DBs | No provenance tracking |
| Graph edges | Based on names/paths | Break on rename/move |
| Federated results | No source identification | Can't trace where data came from |
| Database portability | Identity lost when moved | Can't share/backup databases |
| Deduplication | No way to identify source | Duplicate results in federation |

**Root cause:** Databases lack intrinsic identity. They're files, not entities.

---

## Design Principles

### From Dependable Rust

> "Black box modules with small, stable interfaces"

**Applied here:**
- UID is a single, stable identifier (8 hex chars)
- Interface is minimal: `get_uid()`, `set_uid()`
- Implementation details (hash algorithm, storage) are hidden
- Once assigned, UID never changes

### From Unix Philosophy

> "One tool, one job, done well"

**Applied here:**
- UID does ONE thing: identifies the database
- Not overloaded with version, location, or metadata
- Separate concerns: identity (UID) vs location (path) vs metadata (registry)

### From Adapter Pattern

> "Trait-based abstraction for external systems"

**Applied here:**
- All databases share the same identity interface
- Mother graph doesn't care HOW identity works, just that it exists
- Future: could support different identity schemes via trait

---

## What is Identity?

A database's identity answers: "What IS this database?"

Not to be confused with:
- **Location:** Where is the database? (path)
- **Name:** What do humans call it? (registry name)
- **Content:** What's in the database? (eventlog)

Identity must be:
- **Intrinsic:** Stored IN the database, not external
- **Immutable:** Never changes after creation
- **Deterministic:** Same inputs → same UID (reproducible)
- **Collision-resistant:** Practically unique

---

## UID Options Analysis

### Option 1: Random UUID

```
patina-550e8400-e29b-41d4-a716-446655440000.db
```

| Aspect | Assessment |
|--------|------------|
| Uniqueness | Guaranteed (128 bits) |
| Reproducibility | None - must store mapping |
| Human readability | Poor (36 chars) |
| Determinism | None |

**Verdict:** Rejected. Violates "deterministic" requirement.

### Option 2: Hash of Filesystem Path

```
sha256("/Users/nicabar/Projects/patina")[:8] → "a3f2b1c4"
patina-a3f2b1c4.db
```

| Aspect | Assessment |
|--------|------------|
| Uniqueness | Good (path uniqueness) |
| Reproducibility | Only if path unchanged |
| Human readability | Good (8 chars) |
| Determinism | Yes, but fragile |

**Verdict:** Rejected. Path changes break identity.

### Option 3: Hash of Git Initial Commit

```
sha256(git_initial_commit_sha)[:8] → "7e2d1f3a"
patina-7e2d1f3a.db
```

| Aspect | Assessment |
|--------|------------|
| Uniqueness | Excellent (git SHA uniqueness) |
| Reproducibility | Perfect (immutable) |
| Human readability | Good (8 chars) |
| Determinism | Yes |

**Verdict:** Good, but requires git history. Empty repos have no commits.

### Option 4: Hash of Canonical Identity (Recommended)

```
# For repos with remote:
sha256("github:anthropics/claude-code")[:8] → "a3f2b1c4"

# For local-only projects:
sha256("git:e7d3a2f1...")[:8] → "7e2d1f3a"  # initial commit
sha256("path:/Users/.../project")[:8] → "b4c8e2a1"  # fallback
```

| Aspect | Assessment |
|--------|------------|
| Uniqueness | Excellent |
| Reproducibility | Perfect for repos, good for local |
| Human readability | Good (8 chars) |
| Determinism | Yes |
| Fallback chain | Yes |

**Verdict:** Recommended. Hierarchical identity with sensible fallbacks.

---

## Canonical Identity Hierarchy

```
1. github:{owner}/{repo}     # Best: globally unique, stable
2. git:{initial_commit_sha}  # Good: immutable, requires history
3. path:{absolute_path}      # Fallback: fragile but always available
```

**Resolution algorithm:**

```rust
fn canonical_identity(project_path: &Path) -> String {
    // Try git remote first
    if let Some(remote) = get_git_remote(project_path) {
        if let Some((owner, repo)) = parse_github_url(&remote) {
            return format!("github:{}/{}", owner, repo);
        }
    }

    // Try initial commit
    if let Some(sha) = get_initial_commit(project_path) {
        return format!("git:{}", sha);
    }

    // Fallback to path
    format!("path:{}", project_path.canonicalize().display())
}

fn compute_uid(canonical: &str) -> String {
    let hash = sha256(canonical.as_bytes());
    format!("{:08x}", u32::from_be_bytes(hash[..4].try_into().unwrap()))
}
```

---

## Schema Design

### New `_meta` Table

Every `patina.db` gets a metadata table:

```sql
CREATE TABLE IF NOT EXISTS _meta (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

-- Required entries (set at init, never change)
INSERT INTO _meta (key, value) VALUES ('uid', 'a3f2b1c4');
INSERT INTO _meta (key, value) VALUES ('canonical', 'github:anthropics/claude-code');
INSERT INTO _meta (key, value) VALUES ('created', '2026-01-12T12:00:00Z');
INSERT INTO _meta (key, value) VALUES ('schema_version', '2');
```

### Database Filename

Option A: **Embed UID in filename**
```
.patina/data/patina-a3f2b1c4.db
```
- Pro: Identity visible in filesystem
- Con: Migration required for existing DBs

Option B: **Keep filename, store UID internally**
```
.patina/data/patina.db  (contains _meta.uid = 'a3f2b1c4')
```
- Pro: No migration, backward compatible
- Con: Must open DB to learn identity

**Recommendation:** Option B for Phase 1 (internal), Option A optional for Phase 2.

---

## Federation Integration

### Mother Graph Updates

```sql
-- Current schema
CREATE TABLE nodes (
    id TEXT PRIMARY KEY,      -- name: "claude-code"
    path TEXT NOT NULL,       -- location
    ...
);

-- Updated schema
CREATE TABLE nodes (
    uid TEXT PRIMARY KEY,     -- intrinsic: "a3f2b1c4"
    name TEXT NOT NULL,       -- human: "claude-code"
    path TEXT NOT NULL,       -- location (can change)
    canonical TEXT,           -- "github:anthropics/claude-code"
    ...
);

CREATE TABLE edges (
    from_uid TEXT NOT NULL,   -- "7e2d1f3a" (was: from_node)
    to_uid TEXT NOT NULL,     -- "a3f2b1c4" (was: to_node)
    ...
);
```

### Cross-DB References in Eventlog

```sql
-- In patina's eventlog: reference to claude-code data
INSERT INTO eventlog (event_type, data) VALUES (
  'reference.pattern',
  '{
    "pattern": "error boundary",
    "source_uid": "a3f2b1c4",
    "source_event_seq": 12345,
    "learned_at": "2026-01-12T12:00:00Z"
  }'
);
```

### Federated Query Results

```rust
pub struct FederatedResult {
    pub source_uid: String,      // "a3f2b1c4"
    pub source_name: String,     // "claude-code" (for display)
    pub doc_id: String,
    pub score: f32,
    pub content: String,
}
```

---

## Migration Strategy

### Phase 1: Add Identity (Non-Breaking)

1. Add `_meta` table to new databases at init
2. Add `_meta` table to existing databases on first open (lazy migration)
3. Compute UID from canonical identity
4. Store but don't require for existing flows

```rust
fn ensure_identity(conn: &Connection, project_path: &Path) -> Result<String> {
    // Check if already has identity
    if let Some(uid) = get_meta(conn, "uid")? {
        return Ok(uid);
    }

    // Compute and store
    let canonical = canonical_identity(project_path);
    let uid = compute_uid(&canonical);

    set_meta(conn, "uid", &uid)?;
    set_meta(conn, "canonical", &canonical)?;
    set_meta(conn, "created", &Utc::now().to_rfc3339())?;

    Ok(uid)
}
```

### Phase 2: Update Mother Graph

1. Migrate `nodes.id` → `nodes.uid`
2. Migrate `edges.from_node/to_node` → `edges.from_uid/to_uid`
3. Keep `nodes.name` for human display

### Phase 3: Enable Cross-DB References

1. Add `source_uid` field to relevant event types
2. Update scry to include source in results
3. Add provenance tracking in feedback loop

### Phase 4: Optional Filename Update

1. Rename `patina.db` → `patina-{uid}.db`
2. Update all path references
3. Add symlink for backward compatibility

---

## API Design

### Library Interface

```rust
// src/db/identity.rs

/// Get database UID (creates if missing)
pub fn get_or_create_uid(conn: &Connection, project_path: &Path) -> Result<String>;

/// Get database UID (returns None if missing)
pub fn get_uid(conn: &Connection) -> Result<Option<String>>;

/// Get canonical identity string
pub fn get_canonical(conn: &Connection) -> Result<Option<String>>;

/// Check if database has identity
pub fn has_identity(conn: &Connection) -> Result<bool>;
```

### CLI Integration

```bash
# Show database identity
patina doctor --identity
# Database: patina-a3f2b1c4
# Canonical: github:NicabarNimble/patina
# Created: 2026-01-12T12:00:00Z

# Show all known databases
patina repo list --uids
# UID       Name          Canonical
# a3f2b1c4  claude-code   github:anthropics/claude-code
# 7e2d1f3a  patina        github:NicabarNimble/patina
# b4c8e2a1  local-proj    path:/Users/.../local-proj
```

---

## Testing Strategy

### Unit Tests

```rust
#[test]
fn test_canonical_identity_github() {
    let id = canonical_identity_from_remote("git@github.com:anthropics/claude-code.git");
    assert_eq!(id, "github:anthropics/claude-code");
}

#[test]
fn test_uid_deterministic() {
    let uid1 = compute_uid("github:anthropics/claude-code");
    let uid2 = compute_uid("github:anthropics/claude-code");
    assert_eq!(uid1, uid2);
}

#[test]
fn test_uid_collision_resistance() {
    let uid1 = compute_uid("github:anthropics/claude-code");
    let uid2 = compute_uid("github:anthropics/claude-cod");  // typo
    assert_ne!(uid1, uid2);
}
```

### Integration Tests

```rust
#[test]
fn test_migration_preserves_data() {
    // Create old-style DB
    // Run migration
    // Verify all data intact + has identity
}

#[test]
fn test_cross_db_reference() {
    // Create two DBs with UIDs
    // Insert cross-reference in eventlog
    // Query and verify provenance
}
```

---

## Success Criteria

| Phase | Criteria |
|-------|----------|
| 1 | All new DBs have `_meta` table with UID |
| 1 | Existing DBs get UID on first open |
| 1 | `patina doctor --identity` shows UID |
| 2 | Mother graph uses UIDs for edges |
| 2 | `patina repo list --uids` works |
| 3 | Scry results include source UID |
| 3 | Eventlog can store cross-DB references |
| 4 | (Optional) Filenames include UID |

---

## Design Decision: 32-bit UID (8 hex chars)

**Decision:** Use 32-bit UIDs (8 hex characters).

**Rationale:** UIDs are namespaced per-user. Each user's databases only need to be unique within their own namespace. At 32 bits, collision risk reaches 1% at ~9,300 databases - far beyond any realistic personal usage. Future multi-user federation adds a user identity layer on top:

```
{user_uid}:{db_uid}  →  64-bit effective global namespace
```

This matches GitHub's model: repo names aren't globally unique, just unique per owner.

---

## Open Questions

1. **What if canonical changes?** If a local project gets pushed to GitHub, its canonical identity changes. Do we:
   - Keep old UID (stable but now "wrong")
   - Recompute UID (correct but breaks references)
   - Store both (complex)

2. **Should UID be in filename?** Pros: visible identity. Cons: migration pain, longer paths.

---

## References

- [spec-mothership-graph.md](spec-mothership-graph.md) - Graph routing that would use UIDs
- [spec-pipeline.md](spec-pipeline.md) - Scrape pipeline that creates databases
- Session 20260112-061237 - Origin of this design discussion
