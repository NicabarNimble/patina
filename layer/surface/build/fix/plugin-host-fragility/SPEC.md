---
type: fix
id: plugin-host-fragility
status: ready
created: 2026-02-14
sessions:
  origin: 20260214-061751
blocked_by: []
blocks: []
related:
  - layer/surface/build/feat/plugin-system/SPEC.md
  - layer/surface/build/fix/plugin-system-audit-remediation/SPEC.md
beliefs:
  - dependable-rust
  - safety-boundaries
  - sanitize-at-data-level-not-just-control-flow
  - wit-deps-must-be-hard-links-verified
  - compiler-enforced-safety
references:
  - "Eskil Steenberg — simplicity-first, data-oriented, 'if it can break it will'"
  - "Jon Gjengset — Rust ownership rigor, API ergonomics, earned complexity"
---

# fix: Plugin Host Engine — Fragility Remediation

> Three independent analyses (internal audit + two outside agents) converge on
> the same structural fragility in the plugin host internals. The protocol
> boundary (WIT) is strong. The host implementation has accrued duplication debt
> from the rapid 4-world buildout (v0.18-v0.21). Fix before mother-v2 phase 2
> adds more integration surface.

## Problem

The plugin ecosystem (v0.21.0, [[plugin-ecosystem]]) is functionally complete
with 4 WASM worlds, defense-in-depth capability gating, and comprehensive
conformance tests. But the host-side implementation has 7 findings from a
Steenberg/Gjengset-style fragility audit, confirmed by two independent reviews.

### The Three Convergent Findings

All three analyses independently prioritize the same top-3:

| Finding | What | Why it matters |
|---------|------|---------------|
| **F1** | Host trait impls copy-pasted across 4 worlds (log in all 4, layer/query/http in 3) | Security fixes must be applied 3-4x. LLM workflow fragility multiplier — an assistant editing one world has no structural reason to edit the others. Compiler doesn't catch impl drift. |
| **F2** | `count_layer_files` path traversal | Plugin passes `../../..` as subdir, counts `.md` files outside layer tree. Violates `safety-boundaries.md`: "All paths relative to project root." |
| **F4** | Manifest world field is unchecked string | A pipeline manifest claiming `host_query` passes `check_capabilities`. World-specific capability restrictions not enforced. "Too many strings where enums should be." |

### Additional Findings

| Finding | What | Severity |
|---------|------|----------|
| **F5** | Mutex poison recovery on WasmChild with no logging or Store reset | Medium — silent corruption vector for daemon-resident children |
| **G5** | HTTP client builder duplicated in mother-child and task engines | Low — redirect policy changes must be synced manually |
| **G3** | `WasmCell<T>` `unsafe impl Sync` has no compile-time guard against WASM threads | Low — sound today, brittle if atomics enabled |
| **F3** | Pipeline creates fresh Store per `handle()` call | Low — performance, not correctness. Note for future optimization. |

### Source

- Internal audit: session [[20260214-061751]] (Steenberg/Gjengset framing)
- Outside Agent 1: LLM fragility multiplier insight, `CommonHostState` extraction
- Outside Agent 2: Line-level code confirmation, F5 severity escalation, bundled ordering

---

## Design

### Principle: One Logic Body, Thin WIT Wrappers

Each world's bindgen generates separate Rust types (`patina::host::layer::Host`
in `bindings` vs `command_bindings` vs `task_bindings`). These types are genuinely
different — you can't pass one where the other is expected.

The fix is NOT to unify the types (that would fight wasmtime). The fix is to
centralize the **logic** and let each world's `Host` impl be a 1-line delegation:

```rust
// Shared logic in src/plugin/internal/host_support.rs
pub(super) fn find_project_root(project_root: &Option<PathBuf>) -> Option<String> {
    project_root.as_ref().map(|p| p.to_string_lossy().to_string())
}

pub(super) fn count_layer_files(project_root: &Option<PathBuf>, subdir: &str) -> u32 {
    let root = match project_root.as_ref() {
        Some(r) => r,
        None => return 0,
    };
    // F2 FIX: sanitize subdir — reject path traversal
    let sub = std::path::Path::new(subdir);
    if sub.components().any(|c| matches!(c,
        std::path::Component::ParentDir | std::path::Component::RootDir
    )) {
        return 0;  // silent reject — no information leak
    }
    let path = root.join("layer").join(sub);
    // ... count logic
}

pub(super) fn sanitize_and_dispatch_query(
    plugin_name: &str,
    grants: &GrantedCapabilities,
    query_fn: &mut Option<QueryDispatchFn>,
    kind: &str,
    params: &str,
) -> Result<String, String> {
    // Single implementation of call-time gating + scope enforcement + sanitization
}
```

Then each world wrapper:

```rust
impl patina::host::layer::Host for HostState {
    fn find_project_root(&mut self) -> Option<String> {
        host_support::find_project_root(&self.project_root)
    }
    fn count_layer_files(&mut self, subdir: String) -> u32 {
        host_support::count_layer_files(&self.project_root, &subdir)
    }
}
```

This keeps wasmtime type safety, requires no `unsafe`, no macros, and makes
security-sensitive logic changes happen in exactly one place.

### World Enum and Per-World Capability Validation

```rust
/// Known plugin worlds — parsed from manifest, enforced at load time.
#[derive(Debug, Clone, PartialEq)]
pub enum PluginWorld {
    MotherChild,
    Command,
    Task,
    Pipeline,
}

impl PluginWorld {
    pub fn from_str(s: &str) -> Result<Self> {
        match s {
            "mother-child" => Ok(Self::MotherChild),
            "command" => Ok(Self::Command),
            "task" => Ok(Self::Task),
            "pipeline" => Ok(Self::Pipeline),
            other => Err(anyhow!("unknown plugin world: '{}'", other)),
        }
    }

    /// Capabilities this world is allowed to declare.
    pub fn allowed_capabilities(&self) -> &[&str] {
        match self {
            Self::MotherChild => &["host_log", "host_layer", "host_query", "host_http"],
            Self::Command     => &["host_log", "host_layer", "host_query"],
            Self::Task        => &["host_log", "host_layer", "host_query", "host_http"],
            Self::Pipeline    => &["host_log"],
        }
    }
}
```

`check_capabilities` then rejects manifests that claim capabilities their world
doesn't support (e.g., pipeline claiming `host_query`).

### Mutex Poison Policy

Replace silent poison recovery with logged recovery + Store health tracking:

```rust
let mut inner = self.inner.lock().unwrap_or_else(|e| {
    eprintln!(
        "[plugin:{}] WARN: mutex was poisoned, recovering. Previous call may have panicked.",
        self.name
    );
    e.into_inner()
});
```

Full propagation (letting poison crash the daemon) is too aggressive — a single
plugin panic shouldn't take down all children. But silent recovery without logging
is too permissive. The middle path: recover, log, and let the health check surface
the degraded state.

### HTTP Client Builder Extraction

Extract the redirect-rejecting client builder to a shared function:

```rust
// host_support.rs
pub(super) fn build_http_client() -> Result<reqwest::blocking::Client> {
    reqwest::blocking::Client::builder()
        .redirect(reqwest::redirect::Policy::custom(|attempt| {
            if attempt.url().host_str()
                != attempt.previous().last().and_then(|u| u.host_str())
            {
                attempt.stop()
            } else {
                attempt.follow()
            }
        }))
        .build()
        .map_err(|e| anyhow::anyhow!("build HTTP client: {}", e))
}
```

Used by `PluginEngine::instantiate_child()` and `TaskEngine::run_task()`.

### WasmCell Compile-Time Guard

Add a cfg guard to all 4 guest API crates so `unsafe impl Sync` fails to compile
if WASM threads are enabled:

```rust
#[cfg(all(target_arch = "wasm32", not(target_feature = "atomics")))]
unsafe impl<T> Sync for WasmCell<T> {}

#[cfg(all(target_arch = "wasm32", target_feature = "atomics"))]
compile_error!("WasmCell assumes single-threaded WASM. Use thread_local! with atomics.");
```

---

## Findings and Fixes

### Critical (2)

#### F1: Deduplicate host trait implementations

**Lines:** `mother_child.rs:99-297`, `command.rs:86-225`, `task.rs:55-239`, `pipeline.rs:43-53`

Create `src/plugin/internal/host_support.rs` with shared logic functions for:
- `log::Host` — 1 function (`log` with level + prefix formatting). Currently duplicated
  in all 4 worlds including pipeline. Pipeline only imports log, but its `log::Host` impl
  is identical to the other three — must delegate through `host_support` too.
- `layer::Host` — 7 functions (`find_project_root`, `read_config`, `detect_environment`,
  `get_stored_tools`, `count_layer_files`, `get_project_uid`, `check_adapter_version`)
- `query::Host` — 1 function (`query` with gating + scope + sanitization)
- `http::Host` — 2 functions (`http_post`, `http_get` with validation + domain check)

**All 4 worlds** delegate through `host_support` — including pipeline for log. The
"one logic body, thin wrappers" guarantee must cover every world, not just the three
with the largest surface area. If we later need to adjust logging (format, rate
limiting, masking), the change lands once in `host_support`, not once plus a
hand-edit of pipeline.rs.

Each world's `Host` impl becomes a thin delegation wrapper (1-3 lines per method).

Total: ~200 lines of shared logic replaces ~700 lines of quadruplicated logic.

#### F2: Path traversal in count_layer_files

**Lines:** `mother_child.rs:157`, `command.rs:141`, `task.rs:109`

Fix lives inside `host_support::count_layer_files` (the shared implementation from F1).
Validate `subdir` components before joining:

```rust
let sub = std::path::Path::new(subdir);
if sub.components().any(|c| matches!(c,
    std::path::Component::ParentDir | std::path::Component::RootDir
)) {
    return 0;
}
```

Returns 0 on invalid input — no information leak, no error message that confirms
the traversal was attempted. Matches `safety-boundaries.md`: "All paths relative
to project root."

---

### Important (2)

#### F4: World enum with per-world capability validation

**Lines:** `mod.rs:42-61` (PluginManifest), `mother_child.rs:351-411` (check_capabilities)

1. Add `PluginWorld` enum to `mod.rs` with `Display` impl (returns the
   kebab-case string: `"mother-child"`, `"command"`, `"task"`, `"pipeline"`).
2. Change `PluginManifest.world` from `String` to `PluginWorld`.
3. `from_path` parses and validates the world string into the enum.
4. `check_capabilities` receives the world and enforces world-specific
   capability restrictions. A pipeline manifest claiming `host_query` is
   rejected at load time, not at instantiation.

**Downstream consumers that must update** (the enum replaces `.as_str()`
and `==` string comparisons — these won't compile if missed):

| File | Line | Current usage | Fix |
|------|------|---------------|-----|
| `src/main.rs` | 1373 | `manifest.world.as_str()` match dispatch | Match on `PluginWorld::Task`, `PluginWorld::Command` variants |
| `src/plugin/internal/pipeline.rs` | 172 | `manifest.world != "pipeline"` filter | `manifest.world != PluginWorld::Pipeline` |
| `src/commands/plugin.rs` | 47 | `manifest.world` in format string | Use `Display` impl (`{}`prints kebab-case) |
| `src/plugin/internal/tests.rs` | 32 | `assert_eq!(m.world, "mother-child")` | `assert_eq!(m.world, PluginWorld::MotherChild)` |

The compiler will catch any missed site (type mismatch), but listing them
explicitly prevents the "fix one file, discover 4 more won't compile" churn.

#### F5: Mutex poison logging

**Lines:** `mother_child.rs:508,517,523,551,564`

Replace all 5 instances of silent poison recovery with logged recovery.
Add `[plugin:NAME] WARN: mutex was poisoned` so daemon operators can see
when a child has recovered from a panic.

---

### Minor (2)

#### G5: HTTP client builder extraction

**Lines:** `mother_child.rs:435-445`, `task.rs:282-292`

Extract to `host_support::build_http_client()`. Both callsites become
`host_support::build_http_client()?`.

#### G3: WasmCell compile-time guard

**Files:** `patina-plugin-api/src/lib.rs:121`, `patina-command-api/src/lib.rs:131`,
`patina-task-api/src/lib.rs` (equivalent line), `patina-pipeline-api/src/lib.rs:155`

Add `cfg` guard on the `unsafe impl Sync` in all 4 guest API crates.

---

### Deferred (3)

#### G2: QueryDispatchFn type-level modeling

`command.rs:15`: `Option<QueryDispatchFn>` means the type system doesn't encode
whether a plugin has query access. Every call site guards with `.as_mut().ok_or_else(...)`.
Gjengset's recommendation: model as `enum QueryAccess { None, Granted(dispatch_fn) }` or
make HostState generic over query access so plugins without grants can't call it.

**Deferred because**: Once F1 centralizes query dispatch in `host_support.rs`, this
becomes a single-site refactor instead of a 3-file change. Do after this spec lands.
This is the natural next evolution of the host scaffolding.

#### F3: Pipeline instance caching

`pipeline.rs:83-109` creates a fresh Store per handle() call. For batch workloads
(scrape), this pays cold-start on every file. Fix: allow PipelineEngine to cache
a `Store + instance` for reuse within a batch.

**Deferred because**: Performance optimization, not correctness. Pipeline
statelessness is architecturally correct per `unix-philosophy.md`. The
optimization is holding a live instance, not changing the model. Do after
the structural fixes are in place.

#### G4: probe_host_state side effects

`command.rs:315-326`, `task.rs:362-374` instantiate full WASM modules to
read metadata. Probing is mutation.

**Deferred because**: Practically harmless (instances are discarded). Fixing
would require a metadata-only instantiation path in wasmtime, which doesn't
exist. Note for future API version.

---

## Exit Criteria

### Critical
- [ ] `host_support.rs` exists with shared logic for layer, query, http, and log hosts
- [ ] All 4 worlds delegate `Host` impls to `host_support` (including pipeline for log)
- [ ] `count_layer_files` rejects path traversal (`../../..` returns 0)
- [ ] Path traversal test: `count_layer_files("../../etc")` returns 0
- [ ] No duplicated logic bodies across world files (grep test: no `log::Host` impl body outside host_support)

### Important
- [ ] `PluginWorld` enum replaces `String` for manifest world field
- [ ] `PluginManifest::from_path` rejects unknown world strings
- [ ] All downstream consumers compile with enum (`main.rs`, `pipeline.rs`, `plugin.rs`, `tests.rs`)
- [ ] `check_capabilities` enforces per-world capability restrictions
- [ ] Test: pipeline manifest with `host_query = ["scry"]` rejected at check_capabilities
- [ ] Test: manifest with `world = "oracle"` rejected at parse time
- [ ] Mutex poison recovery logs a warning on all 5 lock sites

### Minor
- [ ] `host_support::build_http_client()` shared by mother-child and task engines
- [ ] `WasmCell` has `cfg(not(target_feature = "atomics"))` guard in all 4 guest API crates
- [ ] Compile test: `WasmCell` fails to compile with `target_feature = "atomics"`

### Pre-push
- [ ] `cargo fmt --all`
- [ ] `cargo clippy --workspace`
- [ ] `cargo test --workspace`
- [ ] All existing plugin tests pass (no regressions)
- [ ] New tests pass for path traversal, world validation, capability gating

---

## Files to Change

```
# New file — shared host logic (F1 + F2 + G5)
src/plugin/internal/host_support.rs     # Shared layer/query/http/log implementations

# Modified — thin wrappers replacing duplicated logic (ALL 4 worlds)
src/plugin/internal/mother_child.rs     # layer/query/http/log Host impls → delegate
src/plugin/internal/command.rs          # layer/query/log Host impls → delegate
src/plugin/internal/task.rs             # layer/query/http/log Host impls → delegate
src/plugin/internal/pipeline.rs         # log Host impl → delegate

# Modified — world enum + manifest validation (F4)
src/plugin/internal/mod.rs              # PluginManifest.world: String → PluginWorld enum
src/main.rs                             # F4: world match dispatch (line ~1373)
src/plugin/internal/pipeline.rs         # F4: discover() world filter (line 172)
src/commands/plugin.rs                  # F4: plugin list display (line 47)
src/plugin/internal/tests.rs            # F4: manifest parsing assertions

# Modified — mutex poison logging (F5)
src/plugin/internal/mother_child.rs     # 5 lock sites → logged recovery

# Modified — HTTP client extraction (G5)
src/plugin/internal/mother_child.rs     # instantiate_child → host_support::build_http_client
src/plugin/internal/task.rs             # run_task → host_support::build_http_client

# Modified — WasmCell guard (G3)
patina-plugin-api/src/lib.rs            # cfg guard on unsafe impl Sync
patina-command-api/src/lib.rs           # cfg guard on unsafe impl Sync
patina-task-api/src/lib.rs              # cfg guard on unsafe impl Sync (if exists)
patina-pipeline-api/src/lib.rs          # cfg guard on unsafe impl Sync

# Tests (new or extended)
src/plugin/internal/tests.rs            # Path traversal, world enum, capability gating tests
```

---

## Build Order

Dependencies flow downward — complete top items before bottom:

1. **F1** — Create `host_support.rs`, extract shared logic. This is the structural
   change that everything else builds on. F2 (path fix) lives inside the shared
   `count_layer_files` implementation, so it's fixed automatically.
2. **F4** — `PluginWorld` enum + per-world capability validation. Depends on F1
   because `check_capabilities` moves to use world context.
3. **F5** — Mutex poison logging. Independent of F1/F4 but touch the same file
   (mother_child.rs), so do after F1 to avoid merge conflicts.
4. **G5** — HTTP client extraction into `host_support`. Quick, mechanical.
5. **G3** — WasmCell cfg guard in 4 guest API crates. Independent, do last.
6. **Tests** — Path traversal, world validation, capability gating. Run after
   all code changes.

Target: 6 commits, one per build step. Each commit passes `cargo test`.

---

## Status Log

| Date | Status | Note |
|------|--------|------|
| 2026-02-14 | ready | Created from fragility audit session [[20260214-061751]]. 7 findings from Steenberg/Gjengset audit, confirmed by 2 independent outside agents. 6 in scope (F1, F2, F4, F5, G3, G5), 2 deferred (F3, G4). Ordered per Agent 2 bundling: normalize first, validate second, hygiene third. |
| 2026-02-14 | ready | **Amendment:** F1 scope expanded to include pipeline.rs log delegation. Original spec listed 3 worlds for F1 but pipeline has its own identical `log::Host` impl at `pipeline.rs:43-53`. Leaving it out would create exactly the "copy missed one world" fragility F1 aims to eliminate. Files to Change and exit criteria updated to cover all 4 worlds. Outside review caught the gap. |
| 2026-02-14 | ready | **Amendment:** F4 downstream consumers enumerated. Changing `world` from `String` to `PluginWorld` enum breaks 4 call sites beyond `mod.rs`: `main.rs:1373` (match dispatch), `pipeline.rs:172` (discover filter), `plugin.rs:47` (list display), `tests.rs:32` (assertions). All added to Files to Change and exit criteria. `Display` impl added to design for format-string compatibility. Outside review caught the gap. |
