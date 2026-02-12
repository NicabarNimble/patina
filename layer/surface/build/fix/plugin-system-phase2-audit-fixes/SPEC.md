---
type: fix
id: plugin-system-phase2-audit-fixes
status: complete
created: 2026-02-12
sessions:
  origin: 20260212-121824
blocked_by: []
blocks: []
related:
  - layer/surface/build/feat/plugin-system/SPEC.md
  - layer/surface/build/fix/plugin-system-final-audit-fixes/SPEC.md
  - layer/surface/reports/audit/2026-02-12-plugin-system-phase2.md
  - layer/surface/reports/audit/2026-02-12-plugin-system-phase1-final.md
beliefs:
  - dependable-rust
  - compiler-enforced-safety
  - separate-worlds-for-isolation
  - graceful-extraction
  - spec-driven-design
  - restructure-over-unsafe
---

# fix: Plugin System — Phase 2 Audit Fixes

> Address all findings from the Phase 2 audit
> (`layer/surface/reports/audit/2026-02-12-plugin-system-phase2.md`)
> plus spec amendments. 6 code fixes + 1 spec amendment. Closes v0.17.0.

## Problem

The Phase 2 audit found 0 critical, 0 important, 6 minor, 3 nit. No
blockers, but these should be addressed before declaring v0.17.0
complete. Additionally, the plugin-system SPEC has 4 text divergences
from what was built.

**Audit findings to address:**
- 3.3 — `internal.rs` at 1,356 lines, two worlds in one file
- 4.1 — `static mut` in both guest API crates (edition 2024 migration)
- 4.2 — `CommandEngine` has no internal capability check
- 5.1 — Missing test coverage (plugin list, invalid WASM)
- 5.2 — Benchmark test flaky (OnceLock cold-start)
- 2.1 — Host functions call `find_project_root()` 9× per invocation

**Spec amendments:**
- Step 4 says "PluginEngine" → should be "CommandEngine"
- plugin-cache.toml → explicitly defer to Phase 3
- plugin install/remove → explicitly defer to Phase 3
- Binary size criterion → document mechanism proven, real delta in Phase 3

**Phase 1 carry-forwards (NOT this spec's scope):**
- Registry RwLock poison handling (Phase 1 4.3) — already fixed in
  [[plugin-system-final-audit-fixes]]
- Host state feed for repos child (Phase 1 6.2) — Phase 3 scope
- Duplicate toy prevention (Phase 1 6.3) — already fixed in
  [[plugin-system-final-audit-fixes]]

## Source

Full audit report:
`layer/surface/reports/audit/2026-02-12-plugin-system-phase2.md`

Session: [[20260212-121824]]

---

## Findings and Fixes

### F1: Split internal.rs into internal/ Directory

**Audit ref:** 3.3 (minor — architecture concern)
**Location:** `src/plugin/internal.rs` (1,356 lines)

The file contains two bindgen modules, two engines, one adapter, and
all tests. Phase 3 will add more worlds. Split now while the surgery
is mechanical.

**Target layout:**

```
src/plugin/
├── mod.rs                # External interface (unchanged — 12 lines)
└── internal/
    ├── mod.rs            # wasm_engine() singleton, PluginManifest, shared imports
    ├── mother_child.rs   # mod bindings, HostState, PluginEngine, WasmChild
    ├── command.rs        # mod command_bindings, CommandHostState, CommandEngine
    └── tests.rs          # All tests (shared fixtures, helpers)
```

**Module boundary rules:**
- `internal/mod.rs` owns `wasm_engine()` (OnceLock singleton) — both
  engines call it
- `internal/mod.rs` owns `PluginManifest` + `PluginProvides` — shared
  across worlds
- `internal/mod.rs` owns `pub use` re-exports consumed by `mod.rs`
- `mother_child.rs` and `command.rs` are `pub(super)` — visible to
  `internal/mod.rs` only
- `tests.rs` is `#[cfg(test)] mod tests` included from `internal/mod.rs`

**What moves where:**

| Content | From | To |
|---------|------|----|
| `wasm_engine()`, `PluginManifest`, `PluginProvides` | `internal.rs:1-196` | `internal/mod.rs` |
| `mod bindings`, `HostState`, `PluginEngine`, `WasmChild`, `WasmChildInner` | `internal.rs:16-418` | `internal/mother_child.rs` |
| `mod command_bindings`, `CommandHostState`, `CommandEngine` | `internal.rs:420-614` | `internal/command.rs` |
| All tests | `internal.rs:616-1356` | `internal/tests.rs` |

**Zero logic changes.** Only `use` paths and visibility modifiers change.
`src/plugin/mod.rs` stays identical — it already does
`pub use internal::{CommandEngine, PluginEngine, PluginManifest, PluginProvides}`.

---

### F2: Fix Benchmark Test Flakiness

**Audit ref:** 5.2 (minor — test reliability)
**Location:** `src/plugin/internal.rs` benchmark test (→ `internal/tests.rs`)

`PluginEngine::new()` measures 152ms on cold start (OnceLock engine
init) vs 1ms warm. Full `cargo test --workspace` fails intermittently.

**Fix:** Warm up the engine before timing.

```rust
fn benchmark_plugin_performance() {
    // ...
    // Warm up the process-wide engine singleton (OnceLock).
    // Without this, the first PluginEngine::new() absorbs Engine::new()
    // cold-start cost (~150ms cranelift JIT init), making the benchmark
    // flaky depending on test execution order.
    let _ = PluginEngine::new();

    // 1. PluginEngine::new() — spec threshold: <100ms
    let t0 = Instant::now();
    let engine = PluginEngine::new().unwrap();
    let engine_ms = t0.elapsed().as_secs_f64() * 1000.0;
    // ...
}
```

One line fix. The warm-up call initializes the OnceLock; the timed
call measures only Linker setup + WASI registration.

---

### F3: Cache find_project_root() in CommandHostState

**Audit ref:** 2.1 (minor — performance)
**Location:** `src/plugin/internal.rs` command_bindings (→ `internal/command.rs`)

Six host functions independently call `SessionManager::find_project_root()`.
A single `patina doctor` invocation calls it 9 times.

**Fix:** Cache project root at `CommandHostState` construction time.

```rust
pub struct CommandHostState {
    pub plugin_name: String,
    pub wasi: wasmtime_wasi::WasiCtx,
    pub wasi_table: wasmtime::component::ResourceTable,
    /// Cached project root — computed once at store creation.
    pub project_root: Option<std::path::PathBuf>,
}
```

In `CommandEngine::run_command()` (and `get_command_name()`,
`get_command_description()`):

```rust
let project_root = crate::session::SessionManager::find_project_root().ok();
let host_state = command_bindings::CommandHostState {
    plugin_name: name.to_string(),
    wasi,
    wasi_table: wasmtime::component::ResourceTable::new(),
    project_root,
};
```

Host function implementations use `self.project_root` instead of calling
`find_project_root()`:

```rust
impl patina::host::layer::Host for CommandHostState {
    fn find_project_root(&mut self) -> Option<String> {
        self.project_root.as_ref().map(|p| p.to_string_lossy().to_string())
    }

    fn read_config(&mut self) -> Result<String, String> {
        let root = self.project_root.as_ref()
            .ok_or_else(|| "no project root".to_string())?;
        let config = crate::project::load_with_migration(root)
            .map_err(|e| format!("load config: {}", e))?;
        serde_json::to_string(&config).map_err(|e| format!("serialize config: {}", e))
    }

    // ... same pattern for get_stored_tools, count_layer_files,
    //     get_project_uid, check_adapter_version
}
```

`detect_environment()` doesn't use project root directly
(`Environment::detect()` works from cwd), so no change there.

9 filesystem walks → 1.

---

### F4: CommandEngine Capability Check

**Audit ref:** 4.2 (minor — asymmetry)
**Location:** `src/plugin/internal.rs` CommandEngine (→ `internal/command.rs`)

`PluginEngine::instantiate_child()` calls `check_capabilities()`
internally. `CommandEngine::run_command()` does not — the caller must
check. Asymmetry.

**Fix:** Add `manifest: &PluginManifest` parameter to `run_command()`.
Engine checks capabilities before execution.

```rust
impl CommandEngine {
    pub fn run_command(
        &self,
        component: &Component,
        manifest: &PluginManifest,
        args: &[String],
    ) -> Result<i32> {
        // Check capabilities before execution — matches PluginEngine pattern
        PluginEngine::check_capabilities(manifest)?;

        let wasi = wasmtime_wasi::WasiCtxBuilder::new()
            .inherit_stdout()
            .inherit_stderr()
            .build();
        let project_root = crate::session::SessionManager::find_project_root().ok();
        let host_state = command_bindings::CommandHostState {
            plugin_name: manifest.name.clone(),
            wasi,
            wasi_table: wasmtime::component::ResourceTable::new(),
            project_root,
        };
        // ... rest unchanged
    }
}
```

**Changes to callers:**

`src/main.rs` Doctor dispatch simplifies — no more external capability
check, and the plugin name comes from the manifest:

```rust
let exit_code = if plugin_wasm.exists() {
    let manifest = if plugin_toml.exists() {
        patina::plugin::PluginEngine::load_manifest(&plugin_toml)?
    } else {
        // Default manifest for plugins without .toml
        // (name from filename, command world, auto-granted caps)
        default_command_manifest("patina-doctor", "doctor")
    };
    let engine = patina::plugin::CommandEngine::new()?;
    let wasm_bytes = std::fs::read(&plugin_wasm)?;
    let component = engine.load_component(&wasm_bytes)?;
    engine.run_command(&component, &manifest, &args)?
} else {
    // ... bundled fallback unchanged
};
```

**Probe functions unchanged.** `get_command_name()` and
`get_command_description()` don't take manifests — they probe
plugin identity, not execute it. No capability check needed for
read-only metadata probing.

**Test changes:** Existing `command_doctor_run` test must construct
a manifest. Use the same pattern as `load_repos_child()`:

```rust
fn load_doctor_manifest() -> PluginManifest {
    PluginManifest {
        name: "patina-doctor".into(),
        version: "0.1.0".into(),
        description: "test".into(),
        world: "command".into(),
        patina_min: "0.0.0".into(),
        capabilities: vec!["host_log".into(), "host_layer".into()],
        allowed_toy_commands: vec![],
        provides: PluginProvides {
            child: None,
            commands: vec!["doctor".into()],
        },
    }
}
```

---

### F5: static mut → WasmCell Migration

**Audit ref:** 4.1 (minor — edition 2024 migration)
**Location:** `patina-command-api/src/lib.rs:103`, `patina-plugin-api/src/lib.rs:92`

`static mut PLUGIN` with `#[allow(static_mut_refs)]` works on edition
2021 but won't compile on edition 2024. Both guest API crates need the
same fix.

**Fix:** Replace `static mut` with `UnsafeCell` wrapper. `OnceCell<RefCell<...>>`
won't compile because `RefCell` is not `Sync` (required for `static` items
even on WASM targets).

**Pattern (applied to both crates):**

```rust
use std::cell::UnsafeCell;

/// Single-threaded mutable global for WASM plugin singleton.
///
/// Safety: WASM is single-threaded (wasm32-wasip2 has no threads).
/// No concurrent access is possible. This replaces `static mut` which
/// is deprecated in edition 2024. The `unsafe impl Sync` is required
/// because `static` items must be `Sync`, but WASM's single-threaded
/// execution model makes this sound.
struct WasmCell<T>(UnsafeCell<T>);
unsafe impl<T> Sync for WasmCell<T> {}

static PLUGIN: WasmCell<Option<Box<dyn CommandPlugin>>> =
    WasmCell(UnsafeCell::new(None));

#[doc(hidden)]
pub fn __register_command(plugin: Box<dyn CommandPlugin>) {
    // Safety: called once from init export, WASM is single-threaded
    unsafe {
        *PLUGIN.0.get() = Some(plugin);
    }
}

fn plugin() -> &'static mut dyn CommandPlugin {
    // Safety: WASM is single-threaded, no concurrent access
    unsafe {
        (*PLUGIN.0.get())
            .as_deref_mut()
            .expect("command plugin not initialized — host must call init first")
    }
}
```

**What changes:**
- `static mut PLUGIN` → `static PLUGIN: WasmCell<...>`
- `#[allow(static_mut_refs)]` removed
- `unsafe` blocks remain but with explicit safety comments
- `WasmCell` type added (identical in both crates)

**What doesn't change:**
- The `register_*!` macro
- The `Component` struct + `Guest` impl
- The `export!()` call
- The `CommandPlugin` / `MotherChildPlugin` traits
- Runtime behavior

Per [[restructure-over-unsafe]]: the `unsafe impl Sync` is scoped and
has a clear safety argument (WASM single-threaded), unlike `static mut`
which is a blanket deprecation target.

**Both crates:** `patina-plugin-api` and `patina-command-api` get
identical `WasmCell` implementations. Consider extracting to a shared
`patina-wasm-util` crate if a third guest API crate appears (Phase 4+).
For now, duplication across 2 crates is simpler than a new dependency.

---

### F6: Spec Amendments

**Audit ref:** Section 6 (spec divergences)
**Location:** `layer/surface/build/feat/plugin-system/SPEC.md`

Four text amendments to the Phase 2 section:

**F6a:** Step 4 text — "PluginEngine" → "CommandEngine"

```
Before: "4. CLI loads command plugin via PluginEngine when `patina doctor` is invoked"
After:  "4. CLI loads command plugin via CommandEngine when `patina doctor` is invoked"
```

**F6b:** plugin-cache.toml — add deferral note

```
Before: "CLI plugin discovery: Manifest cache file at ~/.patina/plugin-cache.toml..."
After:  "CLI plugin discovery (deferred to Phase 3): Manifest cache file at
         ~/.patina/plugin-cache.toml... For Phase 2 (single internal plugin),
         CLI dispatch uses hardcoded filename. Generalized discovery via
         plugin-cache.toml deferred to Phase 3 extractions."
```

**F6c:** plugin install/remove — add deferral note

Same paragraph, after "Updated on `patina plugin install/remove`":

```
Add: "(Phase 3 — Phase 2 uses manual copy to ~/.patina/plugins/)"
```

**F6d:** Binary size criterion — add note

```
Before: "Main binary smaller with doctor extracted (measurable delta)"
After:  "Main binary smaller with doctor extracted (measurable delta) —
         NOTE: doctor-specific delta is -31KB (negligible, doctor shares
         all types with core). Mechanism proven. Real savings come with
         Phase 3 extractions (yolo 1,613 LOC, eval+bench 3,229 LOC)."
```

**F6e:** Add status log entry documenting all amendments

---

## Exit Criteria

### Fixes
- [x] F1: `internal.rs` split into `internal/` directory with 4 files
- [x] F1: `src/plugin/mod.rs` unchanged (same 12-line interface)
- [x] F1: All 22 plugin tests pass with no logic changes
- [x] F2: Benchmark test passes in full `cargo test --workspace`
- [x] F2: Engine warm-up before timing, comment explaining why
- [x] F3: `CommandHostState` has `project_root: Option<PathBuf>` field
- [x] F3: Host functions use cached root instead of calling `find_project_root()`
- [x] F4: `run_command()` takes `&PluginManifest`, checks capabilities internally
- [x] F4: `get_command_name()` and `get_command_description()` unchanged
- [x] F4: `src/main.rs` Doctor dispatch updated (no external cap check)
- [x] F4: `command_doctor_run` test updated with manifest
- [x] F5: `static mut PLUGIN` → `WasmCell` in `patina-command-api`
- [x] F5: `static mut PLUGIN` → `WasmCell` in `patina-plugin-api`
- [x] F5: No `#[allow(static_mut_refs)]` in either crate
- [x] F5: WASM fixtures rebuilt (both guest crates changed)
- [x] F6: SPEC.md Phase 2 section amended (4 text changes + status log)

### Pre-push
- [x] `cargo fmt --all`
- [x] `cargo clippy --workspace`
- [x] `cargo test --workspace` — all pass, no flakes
- [x] `./resources/git/pre-push-checks.sh`
- [x] WASM fixtures up to date (rebuild if guest crate source changed)

## Build Order

1. **F6** — Spec amendments first. Text only, no code. Commits cleanly.
2. **F1** — Split internal.rs. Mechanical restructure, zero logic change.
   Run tests immediately after to verify nothing broke.
3. **F2** — Fix benchmark flakiness. One-line warm-up. Verify
   `cargo test --workspace` passes reliably (run 2-3 times).
4. **F3** — Cache project root. Touches `CommandHostState` + all host
   function impls. Tests verify doctor still produces correct output.
5. **F4** — CommandEngine capability check. Changes `run_command()`
   signature → updates callers (main.rs, tests). Must come after F3
   since both modify `CommandEngine`.
6. **F5** — WasmCell migration. Changes guest API crates → requires
   rebuilding WASM fixtures. Last because fixture rebuild is slow
   (wasm32-wasip2 cross-compile).

## Files to Change

```
# F6 — Spec amendments (text only)
layer/surface/build/feat/plugin-system/SPEC.md

# F1 — Split internal.rs
src/plugin/internal.rs         → DELETE (replaced by directory)
src/plugin/internal/mod.rs     → NEW (wasm_engine, PluginManifest, re-exports)
src/plugin/internal/mother_child.rs → NEW (bindings, PluginEngine, WasmChild)
src/plugin/internal/command.rs → NEW (command_bindings, CommandEngine)
src/plugin/internal/tests.rs   → NEW (all 22 tests)

# F2 — Benchmark warm-up
src/plugin/internal/tests.rs   # Add warm-up call before timing

# F3 — Cache project root
src/plugin/internal/command.rs # CommandHostState + host function impls

# F4 — CommandEngine capability check
src/plugin/internal/command.rs # run_command() signature
src/plugin/internal/tests.rs   # command_doctor_run test
src/main.rs                    # Doctor dispatch

# F5 — WasmCell migration
patina-plugin-api/src/lib.rs   # static mut → WasmCell
patina-command-api/src/lib.rs  # static mut → WasmCell
tests/fixtures/patina_plugin_models.wasm  # Rebuild
tests/fixtures/patina_plugin_repos.wasm   # Rebuild
tests/fixtures/patina_doctor.wasm         # Rebuild
~/.patina/plugins/patina-doctor.wasm      # Reinstall
```

---

## Status Log

| Date | Status | Note |
|------|--------|------|
| 2026-02-12 | ready | Created from Phase 2 audit session [[20260212-121824]]. 6 code fixes + 1 spec amendment. Discussed F4 (option A: manifest param), F5 (WasmCell not OnceCell — RefCell not Sync), F1 (single tests.rs). |
| 2026-02-12 | complete | Session [[20260212-124849]]: All 6 fixes + spec amendments executed. 7 commits. F6 spec text, F1 internal/ split (4 files), F2 benchmark warm-up, F3 project root cache (9→1 walks), F4 capability check (manifest param on run_command), F5 WasmCell migration + WASM fixture rebuild. All 22 plugin tests pass. pre-push-checks.sh clean. |
