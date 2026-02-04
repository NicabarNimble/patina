---
type: refactor
id: database-identity
status: in_progress
created: 2026-01-12
updated: 2026-01-22
sessions:
  origin: 20260112-061237
  work:
    - 20260112-093636
    - 20260121-102727
    - 20260122-154954
related:
  - layer/core/dependable-rust.md
  - layer/core/unix-philosophy.md
---

# refactor: Database Identity (UIDs for Federation)

**Progress:** Phase 1 Complete ✅, Phase 2-3 remain

---

## Problem Statement

Patina databases are **anonymous** - they don't know who they are. Identity is assigned externally by filesystem path (fragile) or registry name (can change).

This breaks federation:
- Cross-DB references have no stable target
- Mother graph edges break on path/name changes
- Can't trace where federated results came from

**Root cause:** Databases lack intrinsic identity.

---

## Solution: UID File

```
.patina/
├── uid                    # Project identity (permanent, committed to git)
└── local/
    └── data/
        └── patina.db      # Data (ephemeral, can be rebuilt)
```

**Core principle:** UID identifies the patina index. It's created once, never changes, and survives rebuilds.

---

## UID Generation Strategy (Random + Git Propagation)

**All UIDs are random 32-bit values (8 hex chars).**

```rust
// Simple: just random
let uid = format!("{:08x}", fastrand::u32(..));
```

| Scenario | Behavior |
|----------|----------|
| `patina init` (new project) | Generate random UID |
| `patina init .` (re-init) | Preserve existing UID |
| `patina repo add` (patina project) | Preserve UID from clone |
| `patina repo add` (non-patina repo) | Generate random UID |

**Why this works:**
- Patina projects commit `.patina/uid` to git
- Cloning brings the UID with it
- `create_uid_if_missing()` preserves existing UIDs
- Same project = same UID via git propagation, not hashing

**Example:**
```
# anthropics/claude-code is a patina project with uid "abc12345"
# When anyone clones it (as ref repo or fork), they get that same UID

User A: patina repo add anthropics/claude-code → uid: abc12345 (from clone)
User B: patina repo add anthropics/claude-code → uid: abc12345 (from clone)
User C: gh repo fork --clone && patina init .  → uid: abc12345 (from clone)
```

---

## Multi-User Identity

**Single machine:** UID alone (user is implicit)

**Federation:** `{user}:{uid}` provides namespace

```
nicabar:abc12345   # User nicabar's index abc12345
alice:abc12345     # User alice's index abc12345 (different index, same UID is fine)
```

**Why 32-bit is sufficient:**
- 1% collision at ~9,300 projects per user
- User namespace eliminates cross-user collision
- Matches GitHub model: unique per owner, not globally

---

## Design Principles

### From Dependable Rust

> "Black box modules with small, stable interfaces"

- UID is ONE thing: 8 hex chars
- Interface is minimal: `create_uid_if_missing()` and `get_uid()`
- Idempotent: calling create when exists just returns existing
- Plain text file - can inspect with `cat`

### From Unix Philosophy

> "One tool, one job, done well"

- UID identifies the project, nothing else
- Separate concerns: identity (uid file) vs data (patina.db)
- Git as distribution: UID propagates via normal git operations

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

## Current Implementation Status

**Phase 1: Complete ✅ (verified 2026-01-22)**

| Feature | Location | Status |
|---------|----------|--------|
| `uid_path()` | `src/project/internal.rs:313` | ✅ Working |
| `create_uid_if_missing()` | `src/project/internal.rs:319` | ✅ Working |
| `get_uid()` | `src/project/internal.rs:343` | ✅ Working |
| UID created at `patina init` | `src/commands/init/internal/mod.rs:147` | ✅ Working |
| UID committed to git | `src/commands/init/internal/mod.rs:201` | ✅ Working |
| UID for ref repos (new) | `repo/internal.rs:533` → `scaffold_patina()` | ✅ Working |
| UID for ref repos (migration) | `repo/internal.rs:238` → `update_repo()` | ✅ Working |
| UID for projects (migration) | `scrape/mod.rs:61,92` | ✅ Working |
| Doctor shows UID | `src/commands/doctor.rs:252-257` | ✅ Working |

**Verification:** All 15 registered ref repos have UIDs. Patina's own UID: `2bdc808e`.

**Phase 2: Deferred**

| Feature | Location | Status |
|---------|----------|--------|
| DB generation tracking | Database `_meta` table | ⏳ Phase 2 |
| Mother graph uses UIDs | `src/mother/graph.rs` | ⏳ Phase 2 |
| Collision detection | Mother registration | ⏳ Phase 2 |

---

## Implementation

### File Format

```bash
cat .patina/uid
550e8400
```

Plain text, one line, 8 hex characters. No JSON, no metadata.

### Existing Functions

```rust
/// Create UID if missing, return existing if present
/// This is the ONLY function that creates UIDs
pub fn create_uid_if_missing(project_path: &Path) -> Result<String> {
    let uid_file = uid_path(project_path);

    // If UID exists, read and return it (idempotent)
    if uid_file.exists() {
        return Ok(fs::read_to_string(&uid_file)?.trim().to_string());
    }

    // Generate new UID (8 hex chars from random u32)
    let uid = format!("{:08x}", fastrand::u32(..));

    // Ensure .patina directory exists
    fs::create_dir_all(uid_file.parent().unwrap())?;

    // Write UID
    fs::write(&uid_file, &uid)?;
    Ok(uid)
}

/// Get UID (returns None if not initialized)
pub fn get_uid(project_path: &Path) -> Option<String> {
    let uid_file = uid_path(project_path);
    if uid_file.exists() {
        fs::read_to_string(&uid_file).ok().map(|s| s.trim().to_string())
    } else {
        None
    }
}
```

### Ref Repo UID (NEW)

Add to `scaffold_patina()` in `src/commands/repo/internal.rs`:

```rust
fn scaffold_patina(repo_path: &Path) -> Result<()> {
    let patina_dir = repo_path.join(".patina");
    fs::create_dir_all(&patina_dir)?;

    // Create UID if not already present (preserves existing from clone)
    patina::project::create_uid_if_missing(repo_path)?;

    // ... rest unchanged ...
}
```

This single line gives ref repos UIDs:
- If cloned repo has `.patina/uid` → preserved
- If cloned repo has no UID → random generated

---

## Doctor Display

Add UID to health check output in `src/commands/doctor.rs`:

```rust
// In display_health_check()
println!("\nProject Identity:");
if let Some(uid) = patina::project::get_uid(&project_root) {
    println!("  ✓ UID: {}", uid);
} else {
    println!("  ⚠ UID: missing (run 'patina init .' to add)");
}
```

---

## Auto-Create Migration Strategy

Instead of fail guards, auto-create UIDs wherever we touch a project. This provides seamless migration with no breaking changes.

**Philosophy:** `create_uid_if_missing()` is already idempotent. Just call it in more places.

### Touch Points

| Operation | File | Behavior |
|-----------|------|----------|
| `patina repo add` | `repo/internal.rs` → `scaffold_patina()` | Create UID for new ref repos |
| `patina repo update` | `repo/internal.rs` → `update_repo()` | Create UID if missing (migration) |
| `patina scrape` | `scrape/mod.rs` | Create UID if missing (migration) |

### Implementation

```rust
// In scaffold_patina() - new ref repos
patina::project::create_uid_if_missing(repo_path)?;

// In update_repo() - existing ref repos (migration)
let repo_path = Path::new(&entry.path);
patina::project::create_uid_if_missing(repo_path)?;

// In scrape entry point - all projects (migration)
patina::project::create_uid_if_missing(&project_path)?;
```

### Migration Behavior

- Next `patina repo update` → ref repo gets UID
- Next `patina repo update --all` → all ref repos get UIDs
- Next `patina scrape` on any project → UID ensured
- No fail guards, no error messages, no friction
- Existing UIDs preserved (idempotent)

---

## DB Generation Tracking (Phase 2)

Track rebuild count for staleness detection in federation. Deferred until Mother Graph integration.

### Schema

```sql
-- Inside patina.db
CREATE TABLE IF NOT EXISTS _meta (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

-- Set at creation
INSERT INTO _meta (key, value) VALUES ('generation', '1');
```

### On Rebuild

```rust
fn rebuild_database(project_path: &Path) -> Result<()> {
    let old_gen = get_generation(&conn)?;  // Read before delete

    fs::remove_file(db_path)?;             // Delete old DB
    let conn = create_database(db_path)?;  // Create fresh

    set_generation(&conn, old_gen + 1)?;   // Increment generation
    // ... re-scrape ...
}
```

### Full Reference

```
550e8400:3 = "project 550e8400, generation 3"
```

Mother graph tracks `last_indexed_generation` to detect stale caches.

---

## Collision Handling (Phase 2)

With 32-bit UIDs, collision is rare but possible. When registering in mother graph:

```rust
fn register_in_mother(uid: &str, path: &Path) -> Result<()> {
    if let Some(existing) = mother.get_node(uid)? {
        if existing.path != path {
            warn!(
                "UID collision detected: {} already registered at {}",
                uid, existing.path
            );
            mother.log_event("uid_collision", json!({
                "uid": uid,
                "existing_path": existing.path,
                "new_path": path,
            }))?;
        }
    }
    // Proceed with registration (last-write-wins)
    mother.upsert_node(uid, path)?;
    Ok(())
}
```

**Decision:** Detect and warn, don't block.
- Log collision events for analysis
- User can regenerate: `rm .patina/uid && patina init .`

---

## Federation Integration (Future)

### Mother Graph Schema

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

### Federated Results

```
scry "error handling"

Results:
  [550e8400] src/errors.ts:45 - ErrorBoundary pattern
  [a3f2b1c4] src/forge/sync.rs:120 - Result<T> pattern
```

---

## Phases

### Phase 1: Complete UID Coverage ✅

All items complete (verified 2026-01-22):

1. ✅ **Ref repo UIDs (new)** - `repo/internal.rs:533` in `scaffold_patina()`
2. ✅ **Ref repo UIDs (migration)** - `repo/internal.rs:238` in `update_repo()`
3. ✅ **Project UIDs (migration)** - `scrape/mod.rs:61,92` at entry points
4. ✅ **Doctor display** - `doctor.rs:252-257` shows UID in health check

### Phase 2: DB Generation + Mother Graph (Deferred)

1. **DB generation** - `_meta` table with generation counter
2. Mother graph uses UID as primary key
3. Collision detection on registration
4. Edges reference UIDs, not names/paths
5. Staleness detection via generation

### Phase 3: Cross-DB References

1. Eventlog stores `source_uid`
2. Scry results include source UID
3. Full provenance tracking

---

## Success Criteria

| Phase | Criteria |
|-------|----------|
| 1 | `patina repo add` creates UID |
| 1 | `patina repo update` creates UID if missing |
| 1 | `patina scrape` creates UID if missing |
| 1 | `patina doctor` shows UID |
| 2 | DB tracks generation count |
| 2 | Mother graph uses UIDs |
| 2 | Collisions detected and logged |
| 3 | Scry results show source UID |

---

## Design Evolution

**Session 20260112:** Initial design with hybrid model (random for local, hash for ref repos)

**Session 20260121:** Simplified to pure random + git propagation:
- Removed hybrid hash model entirely
- Random UIDs for everything
- Git propagation handles "same repo, same UID" naturally
- Patina projects commit UID → clones inherit it
- Multi-user: `user:uid` namespace makes 32-bit sufficient
- Auto-create migration instead of fail guards (no friction)

**Key insight:** We don't need deterministic hashing because git already propagates the UID. If a repo is a patina project, its UID travels with the clone.

---

## References

- Session 20260112-061237 - Initial design discussion
- Session 20260112-093636 - Explicit create/read pattern, migration strategy
- Session 20260121-102727 - Simplification: random + git propagation model
