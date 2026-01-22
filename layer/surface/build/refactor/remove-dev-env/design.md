# Design: Remove dev_env Subsystem

## Origin Story

The `dev_env` subsystem was part of an early "awaken layer" vision where patina would manage builds, tests, and development environments. The idea was to abstract over docker/dagger/native environments.

**What was built:**
- `DevEnvironment` trait with `build()`, `test()`, `init_project()`, `is_available()`, `fallback()`
- Docker implementation (the only one)
- `patina build` command (wrapper for `docker build`)
- `patina test` command (wrapper for `cargo test` in docker)
- `--dev` flag on init (accepts docker/dagger/native)
- `[dev]` section in config.toml

**What happened:**
- Dagger and native were never implemented
- `yolo` command (1,613 lines) became the real devcontainer generator
- Build/test commands add zero value over running docker/cargo directly
- The abstraction serves no purpose with only one implementation

---

## Dependency Graph

```
┌─────────────────────────────────────────────────────────────────────┐
│                         DELETE ENTIRELY                              │
├─────────────────────────────────────────────────────────────────────┤
│  src/commands/build.rs          (32 lines)                          │
│  src/commands/test.rs           (31 lines)                          │
│  src/dev_env/mod.rs             (43 lines)                          │
│  src/dev_env/docker.rs          (179 lines)                         │
└─────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────┐
│                              MODIFY                                  │
├─────────────────────────────────────────────────────────────────────┤
│  src/lib.rs                     remove: pub mod dev_env             │
│  src/main.rs                    remove: Build, Test commands        │
│                                 remove: --dev flag from Init        │
│  src/commands/mod.rs            remove: build, test exports         │
│  src/version.rs                 remove: DOCKER_VERSION, docker      │
│                                         component tracking          │
│  src/commands/init/internal/                                        │
│    mod.rs                       remove: dev param, devcontainer gen │
│    config.rs                    remove: dev_env param, DevSection   │
│    validation.rs                remove: determine_dev_environment() │
│  src/commands/doctor.rs         simplify: is_tool_required()        │
│  src/project/internal.rs        deprecate: DevSection (backwards    │
│                                            compat on load only)     │
└─────────────────────────────────────────────────────────────────────┘
```

---

## What's Dead

### 1. `patina build` and `patina test`

```rust
// src/commands/build.rs - 32 lines
// Loads config → calls dev_env.build() → runs docker build

// src/commands/test.rs - 31 lines
// Loads config → calls dev_env.test() → runs docker exec cargo test
```

Users can just run `docker build` or `cargo test` directly.

### 2. `DevEnvironment` Trait

```rust
pub trait DevEnvironment {
    fn name(&self) -> &'static str;
    fn version(&self) -> &'static str;
    fn init_project(&self, ...) -> Result<()>;  // Only real use
    fn build(&self, ...) -> Result<()>;          // Dead
    fn test(&self, ...) -> Result<()>;           // Dead
    fn is_available(&self) -> bool;
    fn fallback(&self) -> Option<&'static str>; // Dead
}
```

After removing build/test, only `init_project()` and `is_available()` are used. But `init_project()` duplicates `yolo`.

### 3. `--dev` Flag on Init

```rust
// Accepts: docker, dagger, native
// Reality: Only docker works, dagger/native never implemented
// Result: Always returns "docker"
```

The `determine_dev_environment()` function just returns `"docker".to_string()`.

### 4. `[dev]` Section in Config

```toml
[dev]
type = "docker"  # Always "docker"
```

Used by doctor to check `dev_type == "docker"` which is always true.

### 5. Devcontainer Generation in Init

`init_project()` generates `.devcontainer/Dockerfile` and `devcontainer.json`. But `yolo` does this better with profiles, language detection, and features.

---

## Implementation Details

### Phase 1: Delete Commands and Module

```bash
rm src/commands/build.rs
rm src/commands/test.rs
rm -r src/dev_env/
```

**Edit `src/commands/mod.rs`:**
```rust
// Remove:
pub mod build;
pub mod test;
```

**Edit `src/lib.rs`:**
```rust
// Remove:
pub mod dev_env;
```

**Edit `src/main.rs`:**
- Remove `Build` and `Test` from `Commands` enum
- Remove their match arms
- Remove `--dev` flag from `Init` struct

### Phase 2: Clean Up Init

**Edit `src/commands/init/internal/mod.rs`:**
- Remove `dev: Option<String>` parameter from `execute_init()`
- Remove import of `determine_dev_environment`
- Remove the call to `dev_env.init_project()`
- Remove the "Created {dev} environment files" print

**Edit `src/commands/init/internal/config.rs`:**
- Remove `use patina::dev_env::DevEnvironment;`
- Remove `_dev_env: &dyn DevEnvironment` parameter
- Remove `dev: &str` parameter
- Stop creating `DevSection` in new configs (or set to empty default)

**Edit `src/commands/init/internal/validation.rs`:**
- Delete `determine_dev_environment()` function entirely

### Phase 3: Clean Up Version Tracking

**Edit `src/version.rs`:**
- Remove `use crate::dev_env::docker::DOCKER_VERSION;`
- Remove "docker" component from `VersionManifest::new()`
- Remove from `UpdateChecker::get_available_versions()`
- Update tests: change `assert_eq!(manifest.components.len(), 3)` to `2`
- Remove docker-specific test assertions

### Phase 4: Simplify Doctor

**Edit `src/commands/doctor.rs`:**
- Remove `dev_type` parameter from `analyze_environment()`
- Simplify `is_tool_required()`:

```rust
fn is_tool_required(tool: &str) -> bool {
    matches!(tool, "cargo" | "rust" | "git")
}
```

Docker becomes optional (detected but not required).

### Phase 5: Config Backwards Compatibility

**Edit `src/project/internal.rs`:**
- Keep `DevSection` struct with `#[serde(default)]` so old configs load
- New configs won't write `[dev]` section
- Add `#[serde(skip_serializing_if = "DevSection::is_default")]`

```rust
impl DevSection {
    fn is_default(&self) -> bool {
        self.dev_type == "docker" && self.version.is_none()
    }
}
```

### Phase 6: Verify

```bash
cargo build --release
cargo test --workspace
cargo clippy --workspace
cargo install --path .

# Verify commands removed
patina build 2>&1 | grep -q "unrecognized subcommand"
patina test 2>&1 | grep -q "unrecognized subcommand"

# Verify init still works (without devcontainer generation)
cd /tmp && mkdir test-init && cd test-init && git init
patina init test-project
ls .devcontainer/  # Should not exist

# Verify doctor works
patina doctor
```

---

## Risk Assessment

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Hidden runtime caller | None | Grep confirms no callers |
| Breaking change | None | Commands are unused |
| Config migration | Low | Serde defaults handle missing `[dev]` |

---

## What Remains After

- `patina init` creates `.patina/` skeleton (no devcontainer)
- `patina yolo` handles devcontainer generation (full-featured)
- `patina doctor` checks cargo/rust/git as required
- Old configs with `[dev]` still load (backwards compat)
- New configs don't write `[dev]` section

---

## Note on Adapters

The `post_init()` trait method takes `_dev_env: &str` (already ignored):

```rust
fn post_init(&self, _project_path: &Path, _dev_env: &str) -> Result<()>
```

**Decision:** Leave for now. Dead parameter, separate cleanup. Underscore signals unused.

---

## Session References

- `20260121-170710` - Identified build/test as dead code during spec review
- `20260121-190217` - Deep analysis, expanded to full dev_env removal
