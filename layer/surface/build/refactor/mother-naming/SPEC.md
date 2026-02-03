---
type: refactor
id: mother-naming
status: complete
created: 2026-01-22
sessions:
  origin: 20260122-154954
related:
  - spec-mother.md
  - src/mother/
---

# refactor: Mother Naming Cleanup

> One name, everywhere: "mother" not "mothership"

**Problem:** The codebase uses "mother" for the module/command but "mothership" in function names, struct fields, env vars, and comments. This inconsistency causes confusion and makes the code harder to understand.

**Solution:** Rename all "mothership" references to "mother" for consistency.

**Precedent:** Similar cleanup was done for "adapters" vs "frontend" naming.

---

## Exit Criteria

- [ ] No "mothership" in function names
- [ ] No "mothership" in struct fields
- [ ] Environment variable renamed to `PATINA_MOTHER`
- [ ] Comments updated to use "mother"
- [ ] User-facing output uses "Mother" (capitalized) consistently
- [ ] Benchmark files reference correct paths (`src/mother/` not `src/mothership/`)
- [ ] `patina --help` shows consistent terminology

---

## Critical Bug (Fix First)

`resources/bench/patina-dogfood-v1.json:78` references **non-existent paths**:
```json
"relevant_docs": ["src/mothership/mod.rs", "src/mothership/internal.rs"]
```

Should be:
```json
"relevant_docs": ["src/mother/mod.rs", "src/mother/internal.rs"]
```

This was missed when `src/mothership/` was consolidated to `src/mother/` in session 20260106-062944.

---

## Complete Inventory

### 1. Environment Variable (Breaking Change)

| File | Line | Current | New |
|------|------|---------|-----|
| `src/mother/mod.rs` | 46 | `ENV_MOTHERSHIP = "PATINA_MOTHERSHIP"` | `ENV_MOTHER = "PATINA_MOTHER"` |

**Migration:** Check for old var, warn if set, accept both during transition.

### 2. Function Renames (4 functions)

| File | Line | Current | New |
|------|------|---------|-----|
| `src/commands/launch/internal.rs` | 190 | `check_mothership_health()` | `check_mother_health()` |
| `src/commands/launch/internal.rs` | 208 | `ensure_mothership_running()` | `ensure_mother_running()` |
| `src/commands/launch/internal.rs` | 230 | `start_mothership_daemon()` | `start_mother_daemon()` |
| `src/commands/scry/internal/routing.rs` | 43 | `execute_via_mothership()` | `execute_via_mother()` |

### 3. Struct Field Renames (2 fields)

| File | Line | Current | New |
|------|------|---------|-----|
| `src/commands/launch/mod.rs` | 28 | `auto_start_mothership: bool` | `auto_start_mother: bool` |
| `src/workspace/internal.rs` | 98 | `mothership_path: PathBuf` | `mother_path: PathBuf` |
| `src/workspace/mod.rs` | 75 | `mothership_path: PathBuf` | `mother_path: PathBuf` |

### 4. Local Variable Renames (in workspace/internal.rs)

Lines 119, 126-128, 212, 227, 229, 297, 311:
```rust
let mothership = paths::patina_home();  // → let mother = ...
```

### 5. User-Facing Output (Capitalized "Mother")

| File | Line | Current | New |
|------|------|---------|-----|
| `src/commands/launch/internal.rs` | 210 | `"✓ Mothership running"` | `"✓ Mother running"` |
| `src/commands/launch/internal.rs` | 221 | `"✓ Mothership started"` | `"✓ Mother started"` |
| `src/commands/serve/internal.rs` | 101 | `"🚀 Mothership daemon starting..."` | `"🚀 Mother daemon starting..."` |
| `src/commands/scry/internal/routing.rs` | 45 | `"Querying mothership at"` | `"Querying mother at"` |
| `src/commands/model.rs` | 158 | `"Mothership cache:"` | `"Mother cache:"` |
| `src/main.rs` | 299 | `"Start the Mothership daemon"` | `"Start the Mother daemon"` |

### 6. Doc Comments in Source (lowercase "mother")

| File | Lines | Count |
|------|-------|-------|
| `src/commands/launch/internal.rs` | 3, 51, 189, 207, 214, 226, 229 | 7 |
| `src/commands/scry/mod.rs` | 7, 8, 82 | 3 |
| `src/commands/scry/internal/routing.rs` | 3, 42, 47, 49 | 4 |
| `src/commands/serve/mod.rs` | 1, 4, 31 | 3 |
| `src/commands/serve/internal.rs` | 1, 96 | 2 |
| `src/commands/model.rs` | 1 | 1 |
| `src/embeddings/mod.rs` | 77 | 1 |
| `src/main.rs` | 252 | 1 |
| `src/models/mod.rs` | 3, 14, 49, 97, 102, 148 | 6 |
| `src/mother/mod.rs` | 3, 7, 9, 42, 45, 48, 53, 59, 60, 65, 74, 75 | 12 |
| `src/mother/internal.rs` | 1, 8, 31, 38, 41, 49, 62, 79, 112 | 9 |
| `src/paths.rs` | 106 | 1 |
| `src/workspace/internal.rs` | 65, 67 | 2 |

### 7. Benchmark/Test Files (Critical - Wrong Paths)

| File | Line | Issue |
|------|------|-------|
| `resources/bench/patina-dogfood-v1.json` | 78 | References `src/mothership/*.rs` (DOESN'T EXIST) |
| `resources/bench/patina-dogfood-v1.json` | 76-77 | Query ID `df15-mothership` |
| `resources/bench/patina-commits-v1.json` | 12, 55, 78, 81 | References `spec-mother.md` (keep? rename spec?) |
| `resources/bench/temporal-queryset-v2.json` | 235 | Query mentions "mothership" |
| `resources/bench/temporal-queryset.json` | 29, 34, 96 | Query ID and references |

### 8. Layer Documentation (Many Files)

**Core:**
- `layer/core/build.md` - 3 references

**Surface/build specs:**
- `spec-mother.md` - THE MAIN SPEC (rename to `spec-mother.md`?)
- `spec-pipeline.md` - 2 references
- `spec-code-audit.md` - 3 references
- `spec-architectural-alignment.md` - 1 reference
- `spec-epistemic-layer.md` - 1 reference
- `spec-ref-repo-semantic.md` - 1 reference
- `spec-review-q4-2025.md` - 6 references

**Surface/build deferred:**
- `spec-build-system.md`, `spec-hosts-deploy.md`, `spec-lab-automation.md`, `spec-persona-fusion.md`, `spec-report.md`, `spec-skill-derive.md`, `spec-work-deferred.md`

**Surface concepts/analysis:**
- `analysis-command-architecture.md` - 3 references
- `analysis-commit-training-signal.md` - 2 references
- `architecture-patina-embedding.md` - 6 references
- `concept-orchestration-agent.md` - 2 references
- `concept-rag-network.md` - 4 references
- `concept-repo-patina.md` - 4 references

**Epistemic:**
- `beliefs/error-analysis-over-architecture.md` - 1 reference
- `beliefs/measure-first.md` - 2 references
- `beliefs/phased-development-with-measurement.md` - 2 references
- `rules/implement-after-measurement.md` - 2 references

**Reports:**
- `reports/eval/temporal/error-analysis.md` - 2 references

**Resources:**
- `resources/gemini/GEMINI.md` - 1 reference

### 9. Files to NOT Change

**Historical session files** (`layer/sessions/`) - These are historical records:
- `20251208-105433.md` - Documents when `src/mothership/` was created
- `20260106-062944.md` - Documents consolidation to `src/mother/`
- Others with historical references

**Git tags** - Can't be changed:
- `spec/mothership-graph`

---

## Implementation Plan

### Phase 1: Fix Critical Bug
1. Fix `resources/bench/patina-dogfood-v1.json` path references

### Phase 2: Source Code (Rust)
1. Add env var migration (accept both, warn on old)
2. Rename functions (4)
3. Rename struct fields (3)
4. Rename local variables (8)
5. Update user-facing strings (6)
6. Update doc comments (~50)
7. Run `cargo build` and `cargo test`

### Phase 3: Benchmark Files
1. Update `patina-dogfood-v1.json`
2. Update `patina-commits-v1.json`
3. Update `temporal-queryset*.json`

### Phase 4: Documentation
1. Rename `spec-mother.md` → `spec-mother.md`
2. Update all references in layer/ docs
3. Update `resources/gemini/GEMINI.md`

### Phase 5: Verification
1. `cargo clippy --workspace`
2. `cargo test --workspace`
3. `grep -ri mothership src/` returns only migration code
4. `patina doctor` works
5. `patina serve` starts with "Mother daemon" message

---

## Migration Code for Environment Variable

```rust
// In src/mother/mod.rs

pub const ENV_MOTHER: &str = "PATINA_MOTHER";
const ENV_MOTHER_LEGACY: &str = "PATINA_MOTHERSHIP";

/// Check if mother is configured via environment
pub fn is_configured() -> bool {
    if std::env::var(ENV_MOTHER_LEGACY).is_ok() && std::env::var(ENV_MOTHER).is_err() {
        eprintln!("⚠️  PATINA_MOTHERSHIP is deprecated, use PATINA_MOTHER instead");
    }
    std::env::var(ENV_MOTHER).is_ok() || std::env::var(ENV_MOTHER_LEGACY).is_ok()
}

/// Get the mother address from environment
pub fn get_address() -> Option<String> {
    std::env::var(ENV_MOTHER)
        .or_else(|_| std::env::var(ENV_MOTHER_LEGACY))
        .ok()
}
```

---

## Effort Estimate

| Phase | Files | Changes | Risk |
|-------|-------|---------|------|
| 1. Bug fix | 1 | 2 lines | None |
| 2. Source | 15 | ~80 lines | Low (mechanical) |
| 3. Benchmarks | 4 | ~10 lines | None |
| 4. Docs | ~25 | ~60 lines | None |
| 5. Verify | - | - | None |

**Total:** ~150 line changes across ~45 files

---

## Open Questions

1. **Rename spec-mother.md?** → Yes, rename to `spec-mother.md` for consistency
2. **Update git tag references?** → No, git tags are immutable. Keep references to `spec/mothership-graph` as historical.
3. **Capitalization in output?** → Use "Mother" (capitalized) for user-facing, "mother" (lowercase) in code/comments

---

## Status Log

| Date | Status | Note |
|------|--------|------|
| 2026-01-22 | ready | Spec created with complete inventory |
| 2026-02-03 | complete | Verified against codebase: all renames applied, env var migrated with backward compat, benchmark paths fixed |
