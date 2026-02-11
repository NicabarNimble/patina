---
type: refactor
id: version-consolidation
status: active
created: 2026-02-11
sessions:
  origin: 20260211-100430
  work:
  - 20260211-114126
  - 20260211-121154
related:
- layer/surface/build/feat/v1-release/SPEC.md
- layer/surface/build/feat/mother-architecture/SPEC.md
beliefs:
- spec-driven-design
- simplicity-is-architecture
- dependable-rust
- unix-philosophy
- compiler-enforced-safety
- transparent-complexity
---

# refactor: Version Consolidation

> Specs are universal — every patina project has them. Versioning is a plugin.
> Spec drives the lifecycle. The release strategy is optional. Make this true
> in the code, not just in the docs.

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

### 1. Release Strategy: Decouple Spec from Version

Spec completion triggers a **release strategy** — an enum that dispatches
based on project configuration. Spec doesn't know *how* versions work. It
asks the strategy. The strategy is optional and language-aware.

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
    Cargo,     // Rust: safeguards → Cargo.toml → commit → tag
    External,  // BYO versioning: print reminder, don't touch files
    None,      // No versioning: silent no-op
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

### 2. Three User Profiles

Patina manages knowledge for any project — Rust, Node, Python, Go, whatever.
The spec system is language-agnostic (markdown files). Versioning is optional
and language-specific.

| User | Has specs? | Has versions? | Who manages versions? | Strategy |
|------|-----------|--------------|----------------------|----------|
| **Patina-native** | Yes | Yes | Patina | `Cargo` |
| **BYO-version** | Yes | Yes | Their own system | `External` |
| **Spec-only** | Yes | No | Nobody | `None` |

**`Cargo`** — Patina owns the version file. Runs safeguards, bumps
`Cargo.toml`, commits, tags. Full automation.

**`External`** — User manages versions with their own tools (`npm version`,
`poetry version`, manual edits). On spec completion, patina prints what bump
is warranted but doesn't touch files:

```
patina spec status my-feature complete

  Updated: my-feature → complete
  File: layer/surface/build/feat/my-feature/SPEC.md

  Spec type 'feat' → minor bump
  Version management: external (not managed by patina)
  Action needed: bump your version and tag manually
```

**`None`** — No versioning at all. Spec completes silently. For projects
that use specs to track work but don't release versioned artifacts.

### 3. Strategy Resolution

Auto-detect from project, overridable via config:

```
Resolution chain:
  1. Explicit config wins: .patina/config.toml [versioning] strategy
  2. Auto-detect:
     Cargo.toml exists + upstream.owned = true  → Cargo
     Cargo.toml exists + upstream.owned = false → None (fork)
     package.json exists                        → External
     pyproject.toml / setup.py exists           → External
     go.mod exists                              → External
     nothing detected                           → None
```

Config override:

```toml
# .patina/config.toml
[versioning]
strategy = "external"   # "cargo", "external", or "none"
```

### 4. Language-Specific Strategies (Future Variants)

The enum is extensible. Each language that gets native support becomes a
variant with its own version-file manipulation:

| Language | Version file | Variant | Status |
|----------|-------------|---------|--------|
| Rust | `Cargo.toml` | `Cargo` | Build now |
| Node | `package.json` | `Npm` | Future variant |
| Python | `pyproject.toml` | `PyProject` | Future variant |
| Go | tags only | `GitTagOnly` | Future variant |
| Other | varies | `External` | Build now (remind) |
| None | - | `None` | Build now (silent) |

Adding a language: one enum variant + one `preflight`/`execute` match arm.
When WIT lands: each variant becomes a plugin.

### 5. Spec-First Design: One Path for Releases

**`patina spec`** — drives lifecycle, delegates to release strategy on complete:
- `spec status` — lifecycle transitions, release strategy on complete
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

### 6. The Emergency Path: `version hotfix`

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
2. Bumps `BumpType::Patch` through the same release strategy
3. Prints a reminder: "Consider creating a spec for traceability"

Same release path, same safeguards, same typestate — just without the spec
ceremony. The commit message and tag provide the audit trail. This is the
[[spec-driven-design]] threshold in action: work below the spec threshold
still gets the same safety guarantees.

**Not called `patch`** because:
- `BumpType::Patch` is the enum variant — naming collision
- "patch" sounds routine; "hotfix" sounds intentional
- Signals this is an escape hatch, not the normal workflow

**Only available for `Cargo` strategy.** `External` and `None` projects
manage their own hotfixes.

### 7. Evolution Path

```
Phase 1 (now):   ReleaseStrategy enum: Cargo, External, None
                 Typestate: PreparedRelease
                 Config: auto-detect + [versioning] override
Phase 2 (adopt): Add language variants as users need them (Npm, PyProject)
Phase 3 (WIT):   Extract enum to trait, trait maps to WIT interface
                 See [[patina-platform]] for plugin infrastructure
```

Same pattern as Mother's children — enum today, trait when a second
non-trivial implementation arrives, WIT when plugin infra lands.

## Acceptance Criteria

1. [x] `BumpType` enum defined: `Patch`, `Minor`, `Major`
2. [x] `ReleaseStrategy` enum defined: `Cargo`, `External`, `None`
3. [x] `PreparedRelease` typestate: `preflight()` returns it, `execute()` consumes it
4. [x] `Cargo` variant runs safeguard checks:
   - Clean working tree (no uncommitted tracked files)
   - Not behind remote
   - Not diverged from remote
   - Target tag doesn't already exist
   - Index not stale
5. [x] `External` variant prints bump recommendation without touching files
6. [x] `None` variant is silent no-op — spec completes without version noise
7. [x] Strategy auto-detected from project files (Cargo.toml, package.json, etc.)
8. [x] Strategy overridable via `.patina/config.toml` `[versioning]` section
9. [x] `spec status complete` delegates to release strategy (no direct Cargo.toml manipulation)
10. [x] `spec status` supports `--major` flag for 1.0.0 moments (overrides type-based bump)
11. [x] `version hotfix` replaces `version patch` — same safeguards, patch bump, Cargo-only
12. [x] `version milestone` removed (command, functions, milestone queries in version)
13. [x] `version phase` and `version init` removed (already deprecated)
14. [x] `version show` displays next ready spec instead of milestone
15. [x] v1-release milestones converted to checklist (names + status, no version numbers)
16. [x] `patina scrape layer` still indexes milestones for specs that have them (backward compat)

## Non-Goals

- Implementing language-specific strategies beyond Cargo (future variants)
- Changing spec lifecycle transitions other than `complete`
- Adding new spec commands (quick-create, CLI blocking are separate specs)
- Milestone version planning (removing the need for it)
- CI/pre-push enforcement of spec-driven workflow (separate concern)

## Implementation Notes

### Strategy Location

`src/release/` — new module following [[dependable-rust]]:
- `mod.rs`: `BumpType` enum, `ReleaseStrategy` enum, `PreparedRelease` type,
  `bump_for_spec_type()` function, `ReleaseStrategy::from_project()` factory
- `internal.rs`: `Cargo` implementation (safeguards, Cargo.toml manipulation,
  git commit, git tag), `External` implementation (print recommendation),
  `None` implementation (no-op)

`ReleaseStrategy::from_project()` checks config first, then auto-detects from
project files. Spec commands call this factory — they never construct
strategies directly.

### Safeguard Migration

Move `run_safeguard_checks()` from `src/commands/version/internal.rs:91-159`
into `Cargo` preflight. Both `spec status complete` and `version hotfix` use
the same `ReleaseStrategy::preflight()` path — one set of safeguards.

`External` preflight: check that the version file exists (package.json, etc.)
so the reminder is actionable.

`None` preflight: no-op, returns `PreparedRelease` immediately.

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

For `External` strategy, `--major` changes the printed recommendation:
"Action needed: bump to next major version."

### version show After

```
patina 0.16.0                              # from Cargo.toml (Cargo strategy)
Ready: mother-architecture, report, ...    # from spec ready query
```

For `External`/`None` projects, `version show` reports what it can detect
from the project's version file, or just shows patina's own version.

### Spec Command Surface After

```
patina spec status <id> <status>           # lifecycle transition
patina spec status <id> complete           # + release strategy (feat→minor, fix→patch)
patina spec status <id> complete --major   # + release strategy (→ major bump)
patina spec list [--status X] [--target X] # query
patina spec ready                          # unblocked specs
patina spec blocked                        # blocked specs
patina spec archive <id>                   # git-tag + remove
```

### Version Command Surface After

```
patina version                             # show (default)
patina version show [--json] [--components]# current version + ready specs
patina version hotfix <description>        # emergency patch bump (Cargo only)
```

## Build Steps

1. Create `src/release/` with `BumpType`, `ReleaseStrategy`, `PreparedRelease`
2. Implement `Cargo` variant (migrate safeguards + `do_release` logic)
3. Implement `External` variant (print bump recommendation)
4. Implement `None` variant (no-op)
5. Add `from_project()` factory with auto-detection + config override
6. Rewire `spec status complete` to use `ReleaseStrategy`
7. Add `--major` flag to `spec status`
8. Rename `version patch` to `version hotfix`, wire through `ReleaseStrategy`
9. Update `version show` to drop milestone display, show ready specs
10. Remove `version milestone`, `version phase`, `version init`
11. Clean up v1-release milestone versions (names-only checklist)
12. Test: Cargo project complete → bump + tag
13. Test: External project complete → prints recommendation, no file changes
14. Test: None project complete → silent, no version noise
15. Test: fork project → resolves to None
16. Test: config override → respects explicit strategy
17. Test: safeguard failure → no dirty state
18. Test: hotfix → same safeguards, patch bump

## Exit Criteria

- [x] `spec status complete` is the only path for spec-driven version bumps
- [x] `version hotfix` is the only escape hatch for emergency patches (Cargo only)
- [x] Both paths use the same `ReleaseStrategy` with the same safeguards
- [x] No command can move version backward
- [x] Three strategies work: Cargo (automated), External (advisory), None (silent)
- [x] Strategy auto-detected from project, overridable via config
- [x] Safeguard checks prevent dirty-state failures
- [x] `patina version show` reports accurate, non-stale information
- [x] Release strategy follows [[dependable-rust]] pattern with [[compiler-enforced-safety]] typestate
- [x] Non-Rust projects use patina specs without version system interference
