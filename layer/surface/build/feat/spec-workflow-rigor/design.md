# spec-workflow-rigor: Phase 1 Implementation Design

> Supporting doc for SPEC.md. Covers the **how** for Phase 1
> (Command Decomposition + State Machine). Produced by implementation
> readiness audit session 20260223-152707.

## Verdict: Ready to Implement

Phase 1 has no showstoppers. Infrastructure is 60% built. The
decomposition path is clear. The blocked_by on spec-create does not
apply to Phase 1 — it operates on existing specs, not new ones.

**Promote spec-workflow-rigor from draft to ready.**

---

## 1. Gap Analysis: What Exists vs What's Needed

### Reusable As-Is

| Function | File | Used By |
|---|---|---|
| `find_spec(id)` | internal.rs:813 | All commands — spec lookup |
| `parse_spec_file()` / `serialize_spec_file()` | spec.rs:127,150 | All commands — YAML round-trip |
| `archive_spec_inner()` | internal.rs:695 | `complete`, `abandon` |
| `ReleaseStrategy + PreparedRelease` | release/mod.rs | `complete` |
| `get_all_specs()` | internal.rs:363 | `list` (enhanced) |
| `scan_disk_specs()` | internal.rs:288 | `find_spec` fallback |

### Needs Modification

| What | Where | Change |
|---|---|---|
| `SpecFrontmatter` | spec.rs:58 | Add 6 new fields (below) |
| `VALID_STATUSES` | internal.rs:481 | Add `"paused"`, `"blocked"` |
| `create_spec_tag()` | internal.rs:609 | Generalize tag name param |
| `get_blocked_specs()` | internal.rs:163 | Also check `status = 'blocked'` |
| `get_ready_specs()` | internal.rs:33 | Exclude paused/blocked from ready |

### New: Git Helpers (`src/git/operations.rs`)

```rust
/// List tags matching a glob pattern (for tag counter D2)
/// e.g., list_matching_tags("spec/my-spec-paused-*") → ["spec/my-spec-paused-1", "spec/my-spec-paused-2"]
pub fn list_matching_tags(glob: &str) -> Result<Vec<String>>

/// Create an annotated tag on a specific git ref
/// Extends create_tag() which only tags HEAD
pub fn create_tag_at(name: &str, message: &str, git_ref: &str) -> Result<()>

/// Check if there are unresolved merge conflicts (.git/MERGE_HEAD exists)
pub fn has_merge_conflicts() -> Result<bool>
```

Each is ~10 lines wrapping a git subprocess call. Same pattern as
every other function in operations.rs.

### New: Internal Functions (`src/commands/spec/internal.rs`)

```rust
/// Derive next tag sequence number from existing tags (D2)
/// list_matching_tags("spec/{id}-paused-*") → count → N+1
fn next_tag_number(id: &str, prefix: &str) -> Result<u32>

/// Core YAML + DB status update (extracted from update_spec_status)
/// Read file, parse, apply closure, write, update DB
fn mutate_spec<F>(id: &str, mutate: F) -> Result<(String, SpecFrontmatter)>
where F: FnOnce(&mut SpecFrontmatter) -> Result<()>

/// Promote a spec one step: draft→ready, ready→active
pub fn promote_spec(id: &str, json: bool) -> Result<()>

/// Pause an active spec with reason (D1 rules)
pub fn pause_spec(id: &str, reason: &str, json: bool) -> Result<()>

/// Resume a paused or blocked spec
pub fn resume_spec(id: &str, force: bool, json: bool) -> Result<()>

/// Block an active spec on a blocker (D3 rules)
pub fn block_spec(id: &str, blocker: &str, reason: &str, json: bool) -> Result<()>

/// Complete an active spec (release + archive)
pub fn complete_spec(id: &str, major: bool, json: bool) -> Result<()>

/// Abandon a spec (archive, no release)
pub fn abandon_spec(id: &str, reason: Option<&str>, json: bool) -> Result<()>
```

### New: Clap Subcommands (`src/commands/spec/mod.rs`)

```rust
pub enum SpecCommands {
    // Existing query commands (keep):
    Ready { json: bool },
    Blocked { json: bool },
    List { status: Option<String>, target: Option<String>, json: bool },

    // Existing but reduced scope:
    Archive { id: Option<String>, dry_run: bool, stale: bool },

    // Deprecated (prints redirect message):
    Status { id: String, status: String, major: bool, no_archive: bool },

    // NEW mutation commands:
    Promote { id: String, json: bool, force: bool },
    Pause { id: String, reason: String, json: bool },
    Resume { id: String, force: bool, json: bool },
    Block { id: String, by: String, reason: String, json: bool },
    Complete { id: String, major: bool, json: bool },
    Abandon { id: String, reason: Option<String>, json: bool },
}
```

### Cleanup: Dedup with `git::operations`

internal.rs has private copies of functions that exist in
`src/git/operations.rs`:

| internal.rs (private) | git/operations.rs (public) | Action |
|---|---|---|
| `tag_exists()` (line 882) | `tag_exists()` (line 418) | Use `crate::git::tag_exists()` |
| `is_tree_clean()` (line 892) | `is_clean()` (line 78) | Use `crate::git::is_clean()` |
| `create_spec_tag()` (line 609) | `create_tag()` (line 499) | Replace with new `create_tag_at()` |

Note: `is_tree_clean()` uses `--porcelain -uno` (ignores untracked)
while `is_clean()` uses `--porcelain` (includes untracked). The spec
commands need the `-uno` variant. Either add a parameter to
`git::is_clean()` or keep the spec-specific version.

**Recommendation:** Add `is_clean_tracked() -> Result<bool>` to
`git::operations` that uses `-uno`. Then delete the private copy.

---

## 2. SpecFrontmatter Changes

Add to `src/spec.rs` `SpecFrontmatter`:

```rust
/// Why this spec was paused (required on pause)
#[serde(skip_serializing_if = "Option::is_none")]
pub paused_reason: Option<String>,

/// When paused (ISO 8601 date, UTC) — D6
#[serde(skip_serializing_if = "Option::is_none")]
pub paused_date: Option<String>,

/// Tag ref for resume diffs — D4
#[serde(skip_serializing_if = "Option::is_none")]
pub paused_at_tag: Option<String>,

/// Why this spec was blocked
#[serde(skip_serializing_if = "Option::is_none")]
pub blocked_reason: Option<String>,

/// When blocked (ISO 8601 date, UTC) — D6
#[serde(skip_serializing_if = "Option::is_none")]
pub blocked_date: Option<String>,

/// Parent spec ID (set by split) — D5
#[serde(skip_serializing_if = "Option::is_none")]
pub split_from: Option<String>,
```

All `Option<String>` with `skip_serializing_if`. Backward-compatible —
existing specs parse without them (`Default` derives `None`).

---

## 3. State Transition Matrix

### Valid Transitions

```
From        → To         Command     Side Effects
─────────────────────────────────────────────────────────────
draft       → ready      promote     —
ready       → active     promote     tag: spec/<id>-start
active      → paused     pause       WIP commit (if dirty) + tag
active      → blocked    block       tag + blocked_by + spec_deps
active      → complete   complete    release + archive + tag
active      → abandoned  abandon     archive + tag
paused      → active     resume      context diffs + tag
paused      → abandoned  abandon     archive + tag
blocked     → active     resume      context diffs + tag (when blockers done)
blocked     → abandoned  abandon     archive + tag
```

### Invalid Transitions (rejected by each command)

```
draft     → active, paused, blocked, complete    (must go through ready)
ready     → paused, blocked, complete             (must go through active)
paused    → complete, blocked                     (must resume first)
blocked   → complete, paused                      (must resume first)
complete  → *                                     (terminal)
abandoned → *                                     (terminal)
```

### Validation Strategy

Each command validates its own preconditions. No shared matrix:

```rust
// promote: assert draft or ready
fn promote_spec(id: &str, ...) {
    let (_, fm) = find_and_parse(id)?;
    match fm.status.as_deref() {
        Some("draft") => { /* → ready */ }
        Some("ready") => { /* → active, create start tag */ }
        Some(s) => bail!("Cannot promote '{}' status. Spec is {}.", id, s),
        None => bail!("Spec '{}' has no status", id),
    }
}

// pause: assert active
fn pause_spec(id: &str, reason: &str, ...) {
    let (_, fm) = find_and_parse(id)?;
    match fm.status.as_deref() {
        Some("active") => { /* proceed */ }
        Some(s) => bail!("Cannot pause '{}' — status is '{}', expected 'active'", id, s),
        _ => bail!("Spec '{}' has no status", id),
    }
}
```

This is cleaner than a matrix because each command's preconditions
are self-documenting in the function body.

### `spec status` Deprecation Redirect

```rust
fn update_spec_status(id: &str, new_status: &str, ...) -> Result<()> {
    let msg = match new_status {
        "ready" | "active" => format!("Use `patina spec promote {}`", id),
        "paused" => format!("Use `patina spec pause {} --reason \"...\"`", id),
        "blocked" => format!("Use `patina spec block {} --by <blocker>`", id),
        "complete" => format!("Use `patina spec complete {}`", id),
        "abandoned" => format!("Use `patina spec abandon {}`", id),
        _ => format!("Unknown status '{}'", new_status),
    };
    eprintln!("Warning: `spec status` is deprecated.\n  {}", msg);
    // Still execute for one release cycle, then remove
    ...
}
```

---

## 4. Command Implementation Sketches

### `spec promote`

```
1. find_spec(id)
2. parse_spec_file()
3. Validate: draft → ready, ready → active
4. If ready → active: create_tag_at("spec/{id}-start", "Spec activated", "HEAD")
5. Update status in YAML + DB
6. Git commit: "spec: promote {id} to {status}"
7. --json output
```

### `spec pause`

```
1. find_spec(id)
2. parse_spec_file()
3. Validate: status == active
4. Check: no other spec is paused (scan all specs for status=paused)
5. Check: has_merge_conflicts() == false
6. If !is_clean_tracked(): git add + commit "WIP: {id} paused — {reason}"
7. Derive N: next_tag_number(id, "paused")
8. Update YAML: status=paused, paused_reason, paused_date, paused_at_tag
9. serialize + write
10. create_tag_at("spec/{id}-paused-{N}", reason, "HEAD")
11. Update DB
12. Git add spec file + commit: "spec: pause {id} — {reason}"
13. If ANY step after YAML write fails: restore original file content
14. --json output
```

### `spec resume`

```
1. find_spec(id)
2. parse_spec_file()
3. Validate: status == paused or blocked
4. If blocked: check all blockers are complete (query spec_deps + patterns)
   - If not complete and !force: error with blocker status
5. Read paused_at_tag from YAML
6. Clear: paused_reason, paused_date, paused_at_tag, blocked_reason, blocked_date
7. Set status = active
8. Derive N: next_tag_number(id, "resumed")
9. create_tag_at("spec/{id}-resumed-{N}", "Resumed", "HEAD")
10. Update DB (status + clear spec_deps if was blocked)
11. Git add spec file + commit: "spec: resume {id}"
12. Show context diffs:
    - git diff {paused_at_tag}..HEAD (what changed while away)
    - git diff spec/{id}-start..{paused_at_tag} (what you accomplished)
13. --json output
```

### `spec block`

```
1. find_spec(id)
2. parse_spec_file()
3. Validate: status == active
4. Validate: blocker spec exists (find_spec(blocker))
5. Update YAML: status=blocked, append to blocked_by, blocked_reason, blocked_date, paused_at_tag
6. Derive N: next_tag_number(id, "blocked")
7. create_tag_at("spec/{id}-blocked-{N}", "{reason} (blocked by {blocker})", "HEAD")
8. Update DB: patterns status + INSERT spec_deps
9. Git add spec file + commit: "spec: block {id} (waiting on {blocker})"
10. --json output
```

### `spec complete`

```
1. find_spec(id)
2. parse_spec_file()
3. Validate: status == active
4. Delegate to ReleaseStrategy:
   - strategy.preflight(bump, spec_path)
   - prepared.execute(title, spec_path, archive_dir)
   (This is the existing pattern from update_spec_status lines 559-599)
5. If no release bump (explore type): archive_spec_inner()
6. create_spec_tag(id, title, "HEAD~1")
7. --json output with release info
```

### `spec abandon`

```
1. find_spec(id)
2. parse_spec_file()
3. Validate: status != complete, != abandoned
4. Update YAML: status=abandoned
5. archive_spec_inner(id, path, "abandoned", desc, spec_dir)
   (This already does: git rm + commit + tag on HEAD~1)
6. --json output
```

---

## 5. YAML Rollback Pattern (D1)

No rollback mechanism exists today. Add this pattern:

```rust
fn with_yaml_rollback<F>(file_path: &str, action: F) -> Result<()>
where
    F: FnOnce() -> Result<()>,
{
    // Save original content
    let original = std::fs::read_to_string(file_path)?;

    match action() {
        Ok(()) => Ok(()),
        Err(e) => {
            // Restore original file
            let _ = std::fs::write(file_path, &original);
            Err(e)
        }
    }
}
```

Used by `pause` and `block` where YAML is mutated before git
operations that could fail.

---

## 6. Error Types

No new error enum needed. All commands use `anyhow::Result` with
`anyhow::bail!()` — consistent with the rest of the codebase. Error
messages are the API contract for human users; `--json` output
provides structured errors for MCP/skill consumers.

Structured JSON error shape (consistent across all commands):

```json
{
  "error": "Cannot pause spec-X — spec-Y is already paused",
  "command": "pause",
  "spec_id": "spec-X",
  "hint": "Resume, split, or abandon spec-Y first"
}
```

---

## 7. Dependency Analysis: Phase 1 vs spec-create

The spec's prose says `blocked_by: [spec-create]` once spec-create
exists. But:

- **Phase 1 operates on existing specs** — it decomposes `spec status`
  into single-purpose commands. No creation involved.
- spec-create is Phase 0 — scaffolding new specs.
- The spec's YAML frontmatter has **no** `blocked_by` field today.
- Phase 1 through Phase 5 are independent of spec-create.
- Only Phase 6's `spec_create` MCP tool needs spec-create to exist.

**Conclusion:** spec-workflow-rigor is NOT blocked by spec-create for
Phase 1. The spec can be promoted to ready.

The `/spec` skill description (Phase 6) should note `create` as
"(requires spec-create spec)" until that spec ships.

---

## 8. Implementation Order Within Phase 1

Logical dependency order for the new commands:

```
1. SpecFrontmatter changes (spec.rs)           — all commands need new fields
2. VALID_STATUSES update (internal.rs)          — all commands need new statuses
3. Git helper additions (operations.rs)         — commands need tag helpers
4. mutate_spec() + next_tag_number()            — shared infrastructure
5. promote_spec()                               — simplest, validates the pattern
6. complete_spec()                              — extracts from update_spec_status
7. abandon_spec()                               — extracts from update_spec_status
8. pause_spec()                                 — new, uses WIP commit + rollback
9. block_spec()                                 — new, updates spec_deps
10. resume_spec()                               — new, checks blockers + context diffs
11. Deprecate spec status                       — redirect message
12. Fix get_blocked_specs()                     — account for status='blocked'
13. Fix get_ready_specs()                       — exclude paused/blocked
14. Dedup tag_exists/is_tree_clean              — cleanup
```

Steps 1-4 are foundational. Steps 5-7 extract from existing code.
Steps 8-10 are net-new. Steps 11-14 are cleanup.

Each step is a commit boundary.

---

## 9. Testing Approach

All commands are deterministic Rust functions. Test at two levels:

**Unit tests** (internal.rs `#[cfg(test)]`):
- Transition validation: assert promote rejects paused→active
- Tag number derivation: mock tag list, verify N+1
- YAML round-trip: verify new fields serialize/deserialize correctly
- One-paused-spec constraint: mock spec list, verify rejection

**Integration tests** (manual, per spec's Testing section):
- Build release + install: `cargo build --release && cargo install --path .`
- Run each test case from SPEC.md Testing section
- Verify git tags created with correct names and annotations
- Verify context diffs shown on resume
