---
type: refactor
id: version-consolidation
status: draft
created: 2026-02-11
sessions:
  origin: 20260211-100430
related:
- layer/surface/build/feat/v1-release/SPEC.md
- layer/surface/build/feat/mother-architecture/SPEC.md
beliefs:
- spec-driven-design
- simplicity-is-architecture
- dependable-rust
- unix-philosophy
- compiler-enforced-safety
---

# refactor: Version Consolidation

> Three paths bump versions today. Only one works correctly. Make it the only
> path — and make versioning optional via a release hook so specs don't assume
> every project needs version bumps.

## Problem

**Three bump paths, no coordination:**

1. **`spec status <id> complete`** — bumps based on spec type (feat=minor, fix=patch). Reads Cargo.toml, increments forward. Always correct direction.
2. **`version milestone`** — writes an absolute version from spec YAML. Can go backward if patches happened between milestones. **Broken** — v1-release milestones say 0.12.0 but Cargo.toml is at 0.15.3.
3. **`version patch`** — increments Cargo.toml patch. No spec awareness. Created the drift that broke milestones.

**Versioning is hardcoded into spec completion:**

`spec status complete` calls `read_cargo_version()` → `do_release()` directly
(`src/commands/spec/internal.rs:520-537`). It doesn't check
`is_versioning_enabled()`. On a fork repo or non-Rust project, this crashes or
does the wrong thing. Specs are universal (every patina project has them).
Versioning is not (forks, non-library projects, non-Rust projects).

**The version/spec split was intentional** — not every project should adopt
patina's version system. But the current implementation couples them anyway
through hardcoded Cargo.toml manipulation in the spec command.

## Discovery Source

Walkthrough during session 20260211-100430 traced all three code paths and found:

- `spec status complete` has **zero safeguards** (no clean tree, no tag-exists, no behind-remote check)
- `version milestone` has **five safeguards** but the wrong versioning model
- Tag failure in `do_release()` happens **after** commit — no rollback, leaves dirty state
- No path to 1.0.0 exists (feat always bumps minor, never major)
- `spec status complete` doesn't check `is_versioning_enabled()` — versioning is assumed
- Historical analysis: 14 "patch" releases were all spec-worthy work, none were trivial hotfixes — `version patch` was a workaround for a broken milestone system, not a feature

## Solution

### 1. Release Hook: Decouple Spec from Version

Spec completion triggers a **release hook** — an enum today, a trait/WIT
interface when [[patina-platform]] lands. Projects without versioning get the
no-op variant. Spec doesn't know *how* versions work. It just asks the hook.

Per [[compiler-enforced-safety]]: use enums over strings, typestate over
documentation, exhaustive match over convention.

```rust
/// Version bump types — exhaustive enum, not stringly-typed
enum BumpType {
    Patch,
    Minor,
    Major,
}

/// Release strategy — enum today, trait/WIT when plugin infra lands
enum ReleaseStrategy {
    Cargo,  // Owned Rust project: safeguards → Cargo.toml → commit → tag
    None,   // Fork/unversioned: no-op, spec completes without version change
}
```

**Typestate for preflight→release ordering** (per [[compiler-enforced-safety]]):

```rust
impl ReleaseStrategy {
    /// Preflight returns a token. Release consumes it.
    /// Can't call release without preflight — compiler enforces it.
    fn preflight(&self, bump: BumpType) -> Result<PreparedRelease> { ... }
}

impl PreparedRelease {
    /// Consumes self — can only be called once, only after preflight
    fn execute(self, title: &str, spec_path: &str) -> Result<()> { ... }
}
```

**Spec completion becomes:**

```rust
if new_status == "complete" {
    let strategy = ReleaseStrategy::from_project(project_path);
    if let Some(bump) = bump_for_spec_type(&frontmatter.r#type) {
        let prepared = strategy.preflight(bump)?;
        prepared.execute(title, &file_path)?;
    }
}
```

Spec owns lifecycle. Strategy owns versioning. Clean boundary.

### 2. Spec-First Design: One Path for Releases

**`patina spec`** — drives lifecycle, delegates to release hook on complete:
- `spec status` — lifecycle transitions, release hook on complete
- `spec list` / `spec ready` / `spec blocked` — queries
- `spec archive` — git-tag + remove completed specs

**`patina version`** — simplified to two commands:
- `version show` — display current version (+ next ready spec, components)
- `version hotfix <description>` — emergency path (see below)

**Removed:**
- `version milestone` — replaced by `spec status complete`
- `version patch` — replaced by `version hotfix` with safeguards
- `version phase` — already deprecated
- `version init` — already deprecated

### 3. The Emergency Path: `version hotfix`

`version patch` is renamed to `version hotfix` to avoid confusion with the
`BumpType::Patch` enum variant, and to signal its intent: this is an escape
hatch, not a normal workflow.

Historical analysis shows all 14 "patch" releases were spec-worthy work —
`version patch` was a workaround for a broken milestone system. But genuine
emergencies need a fast path: a security fix at 2am shouldn't require YAML
frontmatter.

**`version hotfix` behavior:**

```
patina version hotfix "fix critical auth bypass"
```

1. Runs the same `ReleaseStrategy::preflight()` safeguards (clean tree, tag
   availability, etc.)
2. Bumps `BumpType::Patch` through the same release hook
3. Prints a reminder: "Consider creating a spec for traceability"

Same release path, same safeguards, same typestate — just without the spec
ceremony. The commit message and tag provide the audit trail. This is the
[[spec-driven-design]] threshold in action: work below the spec threshold
still gets the same safety guarantees.

**Not called `patch`** because:
- `BumpType::Patch` is the enum variant — naming collision
- "patch" sounds routine; "hotfix" sounds intentional
- Signals this is an escape hatch, not the normal workflow

### 4. Evolution Path

```
Phase 1 (now):   ReleaseStrategy enum + typestate (PreparedRelease)
Phase 2 (later): Extract enum to trait when second strategy arrives
Phase 3 (WIT):   Trait maps to WIT interface, WASM plugins via wasmtime
                 See [[patina-platform]] for plugin infrastructure
```

Same pattern as Mother's children — the shape is designed for WIT from day
one. The enum-to-trait refactor is mechanical when the time comes.

## Acceptance Criteria

1. [ ] `BumpType` enum defined: `Patch`, `Minor`, `Major`
2. [ ] `ReleaseStrategy` enum defined: `Cargo`, `None`
3. [ ] `PreparedRelease` typestate: `preflight()` returns it, `execute()` consumes it
4. [ ] `CargoRelease` path runs safeguard checks:
   - Clean working tree (no uncommitted tracked files)
   - Not behind remote
   - Not diverged from remote
   - Target tag doesn't already exist
   - Index not stale
5. [ ] `None` strategy is a no-op — spec completes without version errors
6. [ ] `spec status complete` delegates to release strategy (no direct Cargo.toml manipulation)
7. [ ] `spec status` supports `--major` flag for 1.0.0 moments (overrides type-based bump)
8. [ ] `version hotfix` replaces `version patch` — same safeguards, patch bump, escape hatch
9. [ ] `version milestone` removed (command, functions, milestone queries in version)
10. [ ] `version phase` and `version init` removed (already deprecated)
11. [ ] `version show` displays next ready spec instead of milestone
12. [ ] v1-release milestones converted to checklist (names + status, no version numbers)
13. [ ] `patina scrape layer` still indexes milestones for specs that have them (backward compat)

## Non-Goals

- Changing spec lifecycle transitions other than `complete`
- Adding new spec commands
- Implementing non-Cargo release strategies (that's future WIT work)
- Milestone version planning (removing the need for it)

## Implementation Notes

### Strategy Location

`src/release/` — new module following [[dependable-rust]]:
- `mod.rs`: `BumpType` enum, `ReleaseStrategy` enum, `PreparedRelease` type,
  `bump_for_spec_type()` function
- `internal.rs`: `CargoRelease` implementation (safeguards, Cargo.toml
  manipulation, git commit, git tag)

`ReleaseStrategy::from_project()` checks `is_versioning_enabled()` and returns
the appropriate variant. Spec commands call this factory — they never construct
strategies directly.

### Safeguard Migration

Move `run_safeguard_checks()` from `src/commands/version/internal.rs:91-159`
into `CargoRelease` preflight. Both `spec status complete` and `version hotfix`
use the same `ReleaseStrategy::preflight()` path — one set of safeguards.

### Commit/Tag Ordering

Current `do_release()` order: update Cargo.toml → commit → tag. If tag
fails, commit is orphaned.

Fix: check tag availability in `preflight()` before any writes. The
`version milestone` path already does this — migrate the check.

### Major Bump

`--major` flag on `spec status` overrides `bump_for_spec_type()`:

```
patina spec status v1-release complete --major
```

Produces `BumpType::Major` instead of `BumpType::Minor`. The strategy
interprets this: `0.N.0 → 1.0.0`. This is a release concern, not a spec
concern — the flag is passed through to the strategy.

### version show After

```
patina 0.16.0                              # from Cargo.toml
Ready: mother-architecture, report, ...    # from spec ready query
```

Or minimal: just the version line if no ready specs exist.

### Spec Command Surface After

```
patina spec status <id> <status>           # lifecycle transition
patina spec status <id> complete           # + release hook (feat→minor, fix→patch)
patina spec status <id> complete --major   # + release hook (→ major bump)
patina spec list [--status X] [--target X] # query
patina spec ready                          # unblocked specs
patina spec blocked                        # blocked specs
patina spec archive <id>                   # git-tag + remove
```

### Version Command Surface After

```
patina version                             # show (default)
patina version show [--json] [--components]# current version + ready specs
patina version hotfix <description>        # emergency patch bump
```

## Build Steps

1. Create `src/release/` with `BumpType`, `ReleaseStrategy`, `PreparedRelease`
2. Implement `Cargo` variant (migrate safeguards + `do_release` logic)
3. Implement `None` variant (no-op)
4. Rewire `spec status complete` to use `ReleaseStrategy`
5. Add `--major` flag to `spec status`
6. Rename `version patch` to `version hotfix`, wire through `ReleaseStrategy`
7. Update `version show` to drop milestone display, show ready specs
8. Remove `version milestone`, `version phase`, `version init`
9. Clean up v1-release milestone versions (names-only checklist)
10. Test: owned project complete → bump + tag
11. Test: fork project complete → no bump
12. Test: safeguard failure → no dirty state
13. Test: hotfix → same safeguards, patch bump

## Exit Criteria

- [ ] `spec status complete` is the only path for spec-driven version bumps
- [ ] `version hotfix` is the only escape hatch for emergency patches
- [ ] Both paths use the same `ReleaseStrategy` with the same safeguards
- [ ] No command can move version backward
- [ ] Unversioned projects complete specs without version errors
- [ ] Safeguard checks prevent dirty-state failures
- [ ] `patina version show` reports accurate, non-stale information
- [ ] Release strategy follows [[dependable-rust]] pattern with [[compiler-enforced-safety]] typestate
