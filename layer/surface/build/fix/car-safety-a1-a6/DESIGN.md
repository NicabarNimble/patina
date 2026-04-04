# Design: CAR Safety (A1-A6)

## Principle Alignment

- [[dependable-rust]]: keep public behavior stable while fixing internals.
- [[safety-boundaries]]: no unsafe in protocol logic, project-scoped operations.
- [[children-have-agency-toys-are-capabilities]]: capability checks are the security boundary for the child/toy system.
- [[audit-before-refactor]]: read the code before changing it, especially A3.

## Gate Details

### A1: UTF-8 Safe Truncation

**File:** `src/commands/scry/internal/enrichment.rs:417`

**Problem:** `&content[..max_len]` byte-slices a String. Panics on multi-byte UTF-8 characters (e.g., emoji, CJK, accented text in commit messages or belief content).

**Fix:** Replace with `content.floor_char_boundary(max_len)` (stable since Rust 1.82). One-line change.

**Test:** Add test with multi-byte input (`"Hello 世界 🌍"`) truncated at various lengths. Assert no panic and valid UTF-8 output.

**Scope boundary:** Only touch `enrichment.rs`. Do not refactor `truncate_content` or consolidate with `snippet.rs`.

### A2: CWD Thread Safety

**File:** `src/retrieval/engine.rs:284,308`

**Problem:** `std::env::set_current_dir()` is process-global. If the code panics between lines 284-308, CWD is never restored. Mother serves scry via the daemon — concurrent queries would corrupt each other's CWD.

**Why it exists:** Added in Phase 2.8 multi-repo federation (`a97c6deb`) as a shortcut because all path resolution was CWD-relative. Safe when scry was CLI-only and single-threaded. Unsafe now that Mother serves scry.

**Fix (immediate):** RAII guard struct that saves CWD on construction and restores on Drop. This fixes panic safety and is minimal change. Full path parameterization (passing `repo_path` through the query pipeline) is the right long-term direction for Mother-as-scry-server, but that's a larger change best done when the daemon scry path is actively being hardened.

**Scope boundary:** Add `CwdGuard` in `retrieval/engine.rs`. Wrap the existing `set_current_dir` calls. Do NOT refactor the path resolution model in this gate.

**Verification:**
Preconditions: run from a patina project root with `patina scrape && patina oxidize` completed and at least one repo registered via `patina repo add`.
```
cargo test --lib -p patina-ai                     # all pass
patina scry "retrieval architecture"              # current project works
patina scry "retrieval" --repo <registered-repo>  # cross-repo works
patina scry "retrieval" --all-repos               # federation works
```

### A3: Capability Divergence

**Files:** `src/child/internal/mod.rs:127` and `src/child/internal/knowledge_child.rs:754`

**Problem:** Two `check_capabilities` functions with divergent `auto_granted` lists. Freestanding includes `"host_layer"` (6 items); engine version does not (5 items).

**Why it exists:** The freestanding version was the original. When `KnowledgeChildEngine` was extracted (`920be97b`), it got its own copy. `host_layer` was added to the freestanding version later but not propagated to the engine copy.

**This is the security boundary.** Per [[children-have-agency-toys-are-capabilities]] and [[child-construction-canon]] hard rule 2 ("Toys are explicit grants — children never self-grant or dynamically escalate"), the capability check is the gate that enforces least privilege for every child. Two divergent gates means the boundary has a crack.

**Before fixing: investigate.** Read the instantiation path in `knowledge_child.rs`. Does the engine re-check capabilities at instantiation, or does it trust the manifest-time check? If both checks run, they must agree. If only one runs, the other is dead code. The fix depends on the answer:
- If both run: unify the auto_granted list (single constant, both functions reference it). Add a test that proves both paths produce identical results for a fixture manifest.
- If only manifest-time runs: delete the engine copy.
- If only engine-time runs: delete the freestanding copy.

**Test (regardless of which path):** Manifest requesting each auto_granted toy. Manifest requesting a toy NOT in auto_granted. Manifest requesting a toy that only appears in one of the two current lists (`host_layer`).

**Scope boundary:** Only touch `child/internal/`. Do not change the toy system or manifest schema.

### A4: Starting Commit Data Loss

**File:** `mother/src/state.rs:171`

**Problem:** `MotherSessionRecord::starting_commit()` returns `"none"`. Sessions are the evolve verb — Mother must preserve session data correctly. The starting commit is available at begin time but the Mother DB doesn't store it.

**Why it exists:** `9c97cb73` moved state.rs from CLI crate to mother crate. The stub was a gap in the extraction — Mother didn't have the data, and nobody wired it in later.

**Fix:** Add `starting_commit TEXT` column to the Mother sessions table. Store the real starting commit (from `git::head_sha()` at session begin time) in `begin_session`. Return it from `starting_commit()`.

**Migration plan:**
1. `ALTER TABLE sessions ADD COLUMN starting_commit TEXT DEFAULT NULL` on first access (same pattern as `graph.rs:424`).
2. Existing rows get NULL. Real starting commit still available in session artifact YAML and git start tag.
3. No backfill. New sessions populate the column going forward.
4. `starting_commit()` returns column value if non-NULL, falls back to artifact YAML for pre-migration sessions.
5. No breaking API change.

**Verification:**
```
patina ai session start --json     # starting_commit in response
patina ai session end --json       # starting_commit preserved in archive
```

### A5: Dimension Mismatch

**Files:** `src/commands/belief/mod.rs:1056,1065,1100`

**Problem:** Belief grounding hardcodes `semantic.usearch` + `256` dimensions. Phase 5d changed knowledge/sessions domains to raw E5 (768 dimensions). Grounding queries produce garbage when index dimensions don't match the embedding vector size.

**Why it exists:** `efe19d0f` was committed as WIP ("not yet compiled or tested"). The dimensions changed underneath it in Phase 5d.

**Fix:** After loading USearch index, call `index.dimensions()` for actual dimension count. Pass to embedder instead of hardcoded constant. If index doesn't exist, skip grounding gracefully.

**Scope boundary:** Only touch `src/commands/belief/mod.rs`. Do not change oxidize pipeline or USearch index format.

### A6: SessionFrontmatter Schema Drift

**Files:** `src/commands/session/internal.rs:37-58` vs `src/session/internal/artifact.rs:32-56`

**Problem:** Two `SessionFrontmatter` structs. CLI version has `project_uid: Option<String>` for backward compat with 538 pre-UID sessions. Library version has `project_uid: String`.

**Why it exists:** `0ba8943b` created the library type (strict). `f10f687c` added the CLI type (lenient) because the CLI must parse both old and new session formats. This was an intentional backward-compat fork.

**Fix:** Make the canonical type (in `session::internal::artifact`) use `Option<String>` for `project_uid`. Pre-UID sessions are permanent artifacts in `layer/sessions/` — the lenient form is correct. Delete the private copy in `commands/session/internal.rs`. Delete `parse_session_frontmatter` (internal.rs:810) and use `artifact::parse_document`.

**Scope boundary:** Change `project_uid: String` to `Option<String>` in `session/internal/artifact.rs`. Delete the duplicate in `commands/session/internal.rs`.

## Out of Scope

- Dead code deletion and deprecated command cleanup.
- Architecture inversion gate A7 (separate spec).
- `oxidize_for_repo()` CWD fix (same pattern as A2 but separate concern).
- Full path parameterization for daemon scry (future hardening, not this gate).
