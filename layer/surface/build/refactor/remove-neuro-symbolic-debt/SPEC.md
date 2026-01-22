---
type: refactor
id: remove-neuro-symbolic-debt
status: done
created: 2026-01-20
completed: 2026-01-21
session-origin: 20260120-165543
session-completed: 20260121-091351
---

# refactor: Remove Neuro-Symbolic Tech Debt

**Problem:** ~1700 lines of unused Rust code from Nov 2025 "neuro-symbolic-knowledge-system" exploration. Prolog reasoning engine, BeliefStorage, ObservationStorage, and SemanticSearch wrapper were built but never wired up. Beliefs are now markdown files, not code-managed.

**Solution:** Delete unused modules, tests, and dependencies. Keep scrape/scry which work.

**Result:** ~2660 lines deleted across 4 phased commits. Build, tests, clippy all pass.

---

## Quick Reference

| Module | Lines | Status | Action |
|--------|-------|--------|--------|
| `src/reasoning/` | ~660 | ✅ Deleted | Phase 1 |
| `src/storage/` | ~600 | ✅ Deleted | Phase 2 |
| `src/query/` | ~450 | ✅ Deleted | Phase 3 |
| `tests/neuro_symbolic_integration.rs` | ~100 | ✅ Deleted | Phase 1 |
| `tests/semantic_search_integration.rs` | ~80 | ✅ Deleted | Phase 3 |
| `examples/semantic_search_demo.rs` | ~80 | ✅ Deleted | Phase 4 |
| `scryer-prolog` dep | - | ✅ Removed | Phase 1 |

**Total removed**: ~2660 lines + heavy Prolog dependency

---

## Status

- **Phase:** ✅ Complete
- **Archive:** `archive/prolog-exploration-2025-11` (tag preserves code)
- **Architecture Doc:** Moved to `layer/dust/architecture/`

---

## Checklist

- [x] Verify no runtime callers (grep complete)
- [x] Delete `src/reasoning/` module
- [x] Delete `src/storage/` module
- [x] Delete `src/query/` (entire module, not just semantic_search.rs)
- [x] Update `src/lib.rs` (remove exports)
- [x] Delete test files
- [x] Delete example file
- [x] Remove `scryer-prolog` from Cargo.toml
- [x] Run `cargo build --release` ✅
- [x] Run `cargo test --workspace` ✅
- [x] Run `cargo clippy --workspace` ✅

---

See [[design.md]] for origin story, dependency analysis, and detailed file list.
