# Spec: Remove Legacy Repos and Audit

**Status**: ✅ Complete
**Created**: 2025-12-29
**Purpose**: Remove deprecated layer/dust/repos system and audit command

---

## Context

Two legacy systems need removal:

1. **layer/dust/repos/** - Old manual repo management, replaced by `patina repo`
2. **audit.rs** - Hidden command (behind --audit flag), low value

Both add complexity without proportional value. The new `patina repo` command is more capable and properly integrated.

---

## Pre-Removal: Feature Parity

Before removing `doctor --repos`, ensure `patina repo` has equivalent functionality:

| Feature | doctor --repos | patina repo | Gap |
|---------|---------------|-------------|-----|
| List repos | Shows status | `list` | Need status column |
| Check behind | Yes | No | **Add --status flag** |
| Check dirty | Yes | No | **Add --status flag** |
| Update single | No | `update <name>` | - |
| Update all | `--update` | `update --all` | - |

**Action**: Add `patina repo list --status` or `patina repo status` to show which repos are behind/dirty.

---

## Removal Plan

### Phase 1: Git Tag Current State

```bash
git tag -a legacy-repo-audit-v1 -m "Preserve layer/dust/repos and audit.rs before removal"
git push origin legacy-repo-audit-v1
```

### Phase 2: Add Missing Feature to patina repo

Add status checking to `patina repo`:
- `patina repo list --status` - show behind/dirty status for each repo
- Or `patina repo status` - dedicated status subcommand

Implementation in: `src/commands/repo/internal.rs`

### Phase 3: Remove layer/dust/repos Code

**Files to modify:**

| File | Action |
|------|--------|
| `src/commands/doctor.rs` | Remove lines 298-602 (repo handling) |
| `src/main.rs` | Remove `--repos` and `--update` flags from Doctor |
| `src/commands/scrape/code/mod.rs` | Remove legacy repo path logic (lines 70, 113-125) |
| `.gitattributes` | Remove `layer/dust/repos/** linguist-vendored` |

**Estimated reduction**: ~300 lines

### Phase 4: Remove audit.rs

**Files to modify:**

| File | Action |
|------|--------|
| `src/commands/audit.rs` | Delete entire file (797 lines) |
| `src/commands/mod.rs` | Remove `pub mod audit;` |
| `src/main.rs` | Remove `--audit` flag from Doctor |
| `src/commands/doctor.rs` | Remove audit delegation (lines 51-55) |

**Estimated reduction**: ~800 lines

### Phase 5: Clean Up

1. Run `cargo build --release` - verify it compiles
2. Run `cargo test` - verify tests pass
3. Run `cargo clippy` - verify no new warnings
4. Test `patina doctor` - should work without --repos/--audit flags
5. Test `patina repo list --status` - should show status

---

## Post-Removal State

**doctor.rs**: ~290 lines
- Environment detection
- Tool comparison
- Project health status
- Recommendations

**Removed**:
- ~300 lines of legacy repo code
- ~800 lines of audit code
- 2 CLI flags (--repos, --audit)
- 1 hidden command (audit)

**Total reduction**: ~1,100 lines

---

## Migration Path for Existing layer/dust/repos Users

If users have repos in `layer/dust/repos/`, they can migrate:

```bash
# For each repo in layer/dust/repos/
patina repo add https://github.com/owner/repo

# Then optionally delete the old location
rm -rf layer/dust/repos/
```

No automatic migration - it's a clean break.

---

## Files Reference

```
src/commands/doctor.rs      # Lines 298-602 removed
src/commands/audit.rs       # Entire file deleted
src/commands/mod.rs         # Remove audit module
src/commands/scrape/code/mod.rs  # Remove legacy paths
src/main.rs                 # Remove CLI flags
.gitattributes              # Remove linguist line
```

---

## Success Criteria

- [x] `patina repo list --status` shows behind/dirty status
- [x] `patina doctor` works (no --repos, --audit flags)
- [x] `cargo build --release` succeeds
- [x] `cargo test` passes
- [x] `cargo clippy` clean
- [x] Git tag preserves old code for reference (`legacy-repo-audit-v1`)

---

## References

- [build.md](../../core/build.md) - Build recipe and roadmap
- [spec-architectural-alignment.md](./spec-architectural-alignment.md) - Code quality tracking
