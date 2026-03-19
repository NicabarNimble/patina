# Design: Remove Dead Native Child Infrastructure

## Why This Design

The doctrine says: children are WASM components with agency, toys are granted capabilities, Mother owns authority ([[belief:children-have-agency-toys-are-capabilities]]). `patina-sdk` is the single extension surface. But `patina-pipe` exposes a parallel `Child` trait, `child.toml` manifest, and `ChildType` enum that nothing in the production path uses. This dead infrastructure creates the illusion of two SDKs. Removing it makes code match doctrine.

## Build Target

Delete ~800 lines of dead native child infrastructure across 4 files and 1 package. Refactor ~50 lines in the connection model. Add CI guard. DuckLake production path unchanged — it already uses WASM knowledge child + direct HTTP, not native spawn.

## Resolved Decisions

| Decision | Rationale |
|----------|-----------|
| Replace binary check with provider check | `resolve_child_binary` validated a binary on disk; provider-capability check is stronger and still fail-closed |
| Keep `auth.child` as `#[serde(default)]` | Existing connection TOML files must still parse; field becomes ignored on read, empty on write |
| Move `WriteResult` before deleting `routing.rs` | Only live type from the file; move to `mod.rs` to unblock deletion |
| Delete `routing.rs` entirely | `validate_fact()` and `write_facts()` have zero runtime callsites; fact validation was native-pipe-specific |
| Replace `child.toml` schema check with `plugin.toml` | Drift guard stays, target changes to match SDK doctrine |
| Scope CI guards to code dirs only | `layer/` contains session/spec history that references `ChildType` — false positives if not excluded |

## Commits

1. `refactor(connect): replace binary-existence check with provider validation` — remove `resolve_child_binary` call from `resolve.rs`, deprecate `auth.child` field, remove `default_child()` from Provider trait
2. `refactor(broker): move WriteResult to mod.rs` — decouple from `routing.rs`
3. `refactor(broker): delete dead native spawn path` — remove `spawn.rs`, `lifecycle.rs`, `routing.rs`
4. `refactor: remove native connector package` — delete `children/github-connector/`, replace schema check
5. `refactor(pipe-types): remove native manifest surface` — delete `manifest.rs`
6. `ci: add native-child anti-regression guard` — extend `check-single-sdk-surface.sh`

## Direct Code Targets

### Step 1: Connection model refactor

- `src/connect/internal/resolve.rs:22-28` — delete `resolve_child_binary` check, replace with provider validation
- `src/connect/internal/resolve.rs:53-54` — `AuthPlan { child: ... }` — remove `child` field from plan construction
- `src/connect/internal/model.rs:93-94` — `pub child: String` in `AuthConfig` — make `#[serde(default)]`, stop populating
- `src/connect/internal/model.rs:128-129` — `pub child: String` in `AuthPlan` — remove field
- `src/connect/internal/model.rs:192-193` — `ConnectError::ChildNotFound` variant — remove (replaced by existing `UnknownProvider`)
- `src/connect/internal/model.rs:232-239` — `ChildNotFound` Display impl — remove
- `src/connect/providers/mod.rs:58` — `fn default_child(&self) -> &str` — remove from `Provider` trait
- `src/connect/providers/github.rs:105-107` — `default_child()` impl — remove
- `src/connect/providers/github.rs:351-353,376` — `default_child()` tests — remove
- `src/commands/connect.rs:287` — `child: provider.default_child().to_string()` — change to `child: String::new()`
- `src/mother/broker/mod.rs:14` — `pub use self::spawn::resolve_child_binary` — delete re-export

### Step 2: WriteResult decoupling

- `src/mother/broker/routing.rs:16-20` — move `WriteResult` struct definition to `src/mother/broker/mod.rs`
- `src/mother/broker/mod.rs:22` — add `use self::routing::WriteResult` → replace with inline definition

### Step 3: Delete dead files

- `src/mother/broker/spawn.rs` — delete entire file (356 lines)
- `src/mother/broker/lifecycle.rs` — delete entire file (215 lines)
- `src/mother/broker/routing.rs` — delete entire file (395 lines) after verifying zero runtime callsites
- `src/mother/broker/mod.rs:7-12` — remove `pub mod lifecycle; pub mod routing; pub mod spawn;` declarations

### Step 4: Remove native connector

- `children/github-connector/` — delete entire directory
- `Cargo.toml` (workspace root) — remove `"children/github-connector"` from `members`
- `src/commands/schema/internal.rs:314-336` — replace `child.toml` ChildManifest check with `plugin.toml` PluginManifest check

### Step 5: Trim manifest surface

- `crates/patina-pipe-types/src/manifest.rs` — delete entire file (239 lines)
- `crates/patina-pipe-types/src/lib.rs` — remove `pub mod manifest;`

### Step 6: CI guard

- `resources/scripts/check-single-sdk-surface.sh` — add 4 checks:
  - `children/**/child.toml` presence
  - `patina_pipe::Child` / `impl Child for` in `children/` or `src/`
  - `ChildType` in `src/`, `children/`, `sdk/`, `crates/`
  - `resolve_child_binary` in `src/`

## Verification Plan

**After each commit:**
- `cargo check --workspace`
- `cargo test --workspace`

**After Step 4 (behavioral parity):**
- `patina mother run <source>` — single source sync completes
- `patina scrape` — full DuckLake ingestion, compare fact counts to pre-refactor baseline
- Load existing connection TOML with `auth.child = "github-connector"` — must parse without error

**After Step 6:**
- Run `resources/scripts/check-single-sdk-surface.sh` — must pass
- CI workflow validates on push

## Build Readiness

Ready. All code targets identified with line numbers. Each commit is independently testable. No blockers from other specs. No new architecture — pure deletion and decoupling.

## Open Questions

None — all questions resolved during design review with two audit agents in [[session-20260319-071818-503477000]].
