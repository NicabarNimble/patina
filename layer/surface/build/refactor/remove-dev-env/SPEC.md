---
type: refactor
id: remove-dev-env
status: ready
created: 2026-01-21
session-origin: 20260121-190217
---

# refactor: Remove dev_env Subsystem

**Problem:** ~400 lines of vestigial code from abandoned "awaken layer" vision. The `dev_env` subsystem (build/test commands, DevEnvironment trait, --dev flag, devcontainer generation) was built for docker/dagger/native but only docker exists, and it duplicates `yolo`.

**Solution:** Delete the entire subsystem surgically - commands, module, config section, and all dependent code paths.

---

## Quick Reference

| Component | Lines | Status | Action |
|-----------|-------|--------|--------|
| `src/commands/build.rs` | 32 | Unused wrapper | DELETE |
| `src/commands/test.rs` | 31 | Unused wrapper | DELETE |
| `src/dev_env/mod.rs` | 43 | Trait never fully used | DELETE |
| `src/dev_env/docker.rs` | 179 | Only implementation | DELETE |
| `src/version.rs` | ~50 | DOCKER_VERSION refs | EDIT |
| `src/commands/init/internal/*` | ~50 | Dev param, devcontainer | EDIT |
| `src/commands/doctor.rs` | ~5 | dev_type conditional | EDIT |
| `src/project/internal.rs` | ~5 | DevSection compat | EDIT |

**Total removed**: ~400 lines

---

## Status

- **Phase:** Ready to implement
- **Phases:** 6 (delete → init cleanup → version → doctor → config compat → verify)
- **Risk:** None - dead code removal

---

## Checklist

### Phase 1: Delete Commands and Module
- [ ] Delete `src/commands/build.rs`
- [ ] Delete `src/commands/test.rs`
- [ ] Delete `src/dev_env/` directory
- [ ] Remove exports from `src/commands/mod.rs`
- [ ] Remove `pub mod dev_env` from `src/lib.rs`
- [ ] Remove Build, Test from `src/main.rs` Commands enum
- [ ] Remove `--dev` flag from Init struct

### Phase 2: Clean Up Init
- [ ] Remove dev parameter from `execute_init()`
- [ ] Remove `determine_dev_environment` call
- [ ] Remove `dev_env.init_project()` call
- [ ] Remove DevEnvironment import from config.rs
- [ ] Delete `determine_dev_environment()` from validation.rs

### Phase 3: Clean Up Version Tracking
- [ ] Remove DOCKER_VERSION import
- [ ] Remove docker from VersionManifest components
- [ ] Remove docker from UpdateChecker
- [ ] Fix test assertions (3 → 2 components)

### Phase 4: Simplify Doctor
- [ ] Remove dev_type parameter from `analyze_environment()`
- [ ] Simplify `is_tool_required()` to not use dev_type

### Phase 5: Config Backwards Compatibility
- [ ] Add `is_default()` method to DevSection
- [ ] Add `skip_serializing_if` attribute
- [ ] Verify old configs still load

### Phase 6: Verify
- [ ] `cargo build --release` succeeds
- [ ] `cargo test --workspace` passes
- [ ] `cargo clippy --workspace` clean
- [ ] `patina build` → "unrecognized subcommand"
- [ ] `patina test` → "unrecognized subcommand"
- [ ] `patina init` does NOT create `.devcontainer/`

---

See [[design.md]] for dependency graph, origin story, and detailed implementation.
