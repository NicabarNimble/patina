---
id: spec-version-simplification
status: design
created: 2026-01-16
tags: [spec, version, release-plz, tech-debt]
references: [spec-adapter-polish]
---

# Spec: Version Simplification

**Problem:** Multiple manual version systems that are out of sync and never updated.

**Solution:** Single source of truth from `Cargo.toml` + git, managed by release-plz.

---

## Current State (Tech Debt)

### Three Version Systems

| System | Location | Purpose | Status |
|--------|----------|---------|--------|
| Cargo.toml | `version = "0.1.0"` | CLI version | release-plz manages |
| CLAUDE_ADAPTER_VERSION | `manifest.rs` | Template version | Manual, stale |
| versions.json | `.patina/versions.json` | Project tracking | Manual, stale |

### Problems

1. **CLAUDE_ADAPTER_VERSION** is `0.7.0` but meaningless
   - Manually bumped, changelog manually maintained
   - Not connected to actual template changes
   - VERSION_CHANGES array is historical fiction

2. **versions.json** tracks component versions nobody updates
   ```json
   {
     "patina": "0.1.0",
     "components": {
       "claude-adapter": { "version": "0.7.0" },  // stale
       "gemini-adapter": { "version": "0.1.0" },  // stub
       "docker": { "version": "0.1.0" }           // stub
     }
   }
   ```

3. **release-plz is configured but never triggered**
   - Workflow exists: `.github/workflows/release-plz.yml`
   - Config exists: `release-plz.toml`
   - v0.1.0 was created manually, not by release-plz
   - No releases since Dec 16, 2025

---

## Target State

### Single Source of Truth

```
Cargo.toml version  ──►  release-plz  ──►  GitHub Release + Tag
                              │
                              ▼
                    Binary embeds version
                              │
                              ▼
              Projects compare installed vs current
```

### Simplified versions.json

```json
{
  "patina_version": "0.1.0",
  "installed_at": "2026-01-16T12:00:00Z",
  "installed_commit": "abc123def"
}
```

That's it. No component versions. No changelogs. Just:
- What version was installed
- When
- From what commit (for debugging)

### Detection Logic

```rust
fn check_for_updates(project_path: &Path) -> Option<UpdateInfo> {
    let versions = load_versions_json(project_path)?;
    let current = env!("CARGO_PKG_VERSION");

    if versions.patina_version != current {
        Some(UpdateInfo {
            from: versions.patina_version,
            to: current,
        })
    } else {
        None
    }
}
```

---

## What to Delete

### Files to Remove
- `src/adapters/claude/internal/manifest.rs` (or gut it)

### Constants to Remove
```rust
// DELETE these:
pub const CLAUDE_ADAPTER_VERSION: &str = "0.7.0";
pub const GEMINI_ADAPTER_VERSION: &str = "0.1.0";
pub const OPENCODE_ADAPTER_VERSION: &str = "0.1.0";
pub const DOCKER_VERSION: &str = "0.1.0";

const VERSION_CHANGES: &[(&str, &[&str])] = &[...];
```

### Struct Changes
```rust
// BEFORE (version.rs)
pub struct VersionManifest {
    pub patina: String,
    pub components: HashMap<String, ComponentInfo>,
}

// AFTER
pub struct VersionManifest {
    pub patina_version: String,
    pub installed_at: String,
    pub installed_commit: Option<String>,
}
```

---

## Fix release-plz

### Current Issue

release-plz workflow exists but has never created a release:
- v0.1.0 was tagged manually on Dec 16, 2025
- No commits to main since then trigger release-plz
- Workflow may have permission issues

### Root Cause (Found 2026-01-16)

**All 9 runs have failed with same error:**

```
the working directory of this project has uncommitted changes.
If these files are both committed and in .gitignore, either delete
them or remove them from .gitignore.
```

**Problematic files:**
- `.patina/config.toml` - gitignored, project-specific
- `.patina/oxidize.yaml` - gitignored, project-specific
- `.trunk/*` - gitignored, trunk.io artifacts
- `layer/dust/*` - should be committed but aren't on main?

**The problem:** release-plz clones the repo and these gitignored files don't exist in CI, but git sees them as "uncommitted changes" because they were tracked at some point.

### Fix Applied (2026-01-16)

Commit `036e9c64` untracked the conflicting files:
- `layer/dust/` - archived patterns (gitignored)
- `.trunk/` - removed entirely (no longer used)
- `.patina/config.toml`, `.patina/oxidize.yaml` - project-specific

**Next step:** PR to main, then verify release-plz succeeds

---

## Implementation Phases

### Phase 1: Fix release-plz (verify automation works)

- [ ] Diagnose why release-plz hasn't triggered
- [ ] Fix permissions if needed
- [ ] Test with a patch bump
- [ ] Verify GitHub release created

### Phase 2: Simplify version tracking

- [ ] Simplify `VersionManifest` struct
- [ ] Remove component version constants
- [ ] Remove `VERSION_CHANGES` changelog
- [ ] Update `init` to save simplified manifest
- [ ] Update `adapter refresh` to save manifest
- [ ] Update `doctor` to use simplified check

### Phase 3: Wire up update detection

- [ ] `adapter doctor` shows "update available" when versions differ
- [ ] `patina launch` warns if binary newer than installed
- [ ] Consider: auto-refresh prompt on launch?

---

## Success Criteria

1. `release-plz` creates GitHub releases automatically
2. No manual version constants to maintain
3. `versions.json` is simple (3 fields)
4. Projects detect when patina binary is newer
5. `adapter refresh` is the single upgrade path

---

## References

- [release-plz docs](https://release-plz.dev/)
- Session 20251216-055435: release-plz selection
- Session 20251216-085711: release-plz setup
- spec-adapter-polish: original manifest design (superseded)
