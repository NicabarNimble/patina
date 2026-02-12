---
type: fix
id: plugin-system-final-audit-fixes
status: ready
created: 2026-02-12
sessions:
  origin: 20260212-093831
blocked_by: []
blocks: []
related:
  - layer/surface/build/feat/plugin-system/SPEC.md
  - layer/surface/build/feat/mother-repos/SPEC.md
  - layer/surface/build/fix/plugin-system-audit-remediation/SPEC.md
  - layer/surface/reports/audit/2026-02-12-plugin-system-phase1-final.md
  - layer/surface/reports/audit/2026-02-12-plugin-system-phase1.md
beliefs:
  - compiler-enforced-safety
  - dependable-rust
  - mother-is-the-daemon
---

# fix: Plugin System — Final Audit Fixes

> Address findings from the Phase 1 final audit (2026-02-12) plus
> soundness improvement identified during review. 4 fixes. No deferrals.

## Problem

The final audit (`layer/surface/reports/audit/2026-02-12-plugin-system-phase1-final.md`)
found 2 important and 4 minor findings. Post-audit review identified
an additional soundness concern with `unsafe impl Sync` at the WASM
isolation boundary. All 4 fixes are concrete code changes:

1. **`unsafe impl Sync` for WasmChild** — the `bindings::MotherChild`
   instance sits outside the Mutex as a sibling field. The safety argument
   assumes the generated type has no interior mutability, but we don't
   control that type — `wasmtime::component::bindgen!` generates it. A
   future wasmtime version could add `Cell`/`RefCell` inside for caching
   or lazy init, silently breaking soundness. The fix costs nothing: put
   the instance behind the same Mutex as the store. We already lock on
   every call. The `unsafe` disappears entirely.

2. **Registry RwLock poison inconsistency** — 4 bare `.unwrap()` calls
   on RwLock in registry.rs, while WasmChild and SecretsCacheChild both
   use `unwrap_or_else(|e| e.into_inner())`. Same codebase, two patterns.

3. **No toy deduplication** — heartbeat spawns toys every 60s. If a toy
   takes >60s (git pull on large repo, patina scrape on 25K files), the
   next heartbeat spawns duplicates. With 25 stale repos that's 50
   duplicate threads per minute.

4. **WIT directories are copies, not symlinks** — four copies of the same
   WIT files. Future WIT changes require updating all four. Mismatches
   cause cryptic wasmtime instantiation errors.

## Source

Full audit report:
`layer/surface/reports/audit/2026-02-12-plugin-system-phase1-final.md`

Findings: 4.1 (unsafe Sync — sound but fragile), 4.3 (registry poison),
4.6 + 6.3 (toy deduplication), 5.3 (WIT copies).

---

## Findings and Fixes

### F0: Eliminate `unsafe impl Sync` for WasmChild

**Audit ref:** 4.1 (signed off as "sound") — but soundness depends on
an assumption about a generated type we don't control.
**Location:** `src/plugin/internal.rs:286-296`

**Current structure:**

```rust
struct WasmChild {
    name: String,
    store: Mutex<Store<HostState>>,
    instance: bindings::MotherChild,  // ← outside the Mutex
}

// Safety: bindings::MotherChild is Send + !Sync. Its call_*() methods
// take &self (immutable) and require &mut Store (mutable). The Mutex
// on store serializes all WASM calls, preventing concurrent access.
// The instance is effectively immutable between calls.
unsafe impl Sync for WasmChild {}
```

The safety argument says "effectively immutable." That means: no interior
mutability *that we know of*. But `bindings::MotherChild` is generated
by `wasmtime::component::bindgen!`. If a future wasmtime version adds
a `Cell` or `RefCell` inside that type for caching, lazy initialization,
or instrumentation, the `Sync` impl becomes unsound — and nothing in our
code changes to warn us.

This is the WASM isolation boundary. Having `unsafe` here is ironic.

**Fix:** Put the instance behind the Mutex with the store. Zero
performance cost — we already acquire the lock on every call.

```rust
struct WasmChild {
    name: String,
    inner: Mutex<WasmChildInner>,
}

struct WasmChildInner {
    store: Store<HostState>,
    instance: bindings::MotherChild,
}
// No unsafe impl Sync needed — Mutex<T> is Sync when T is Send.
// WasmChildInner is Send because both Store<HostState> and
// bindings::MotherChild are Send.
```

**Changes to `instantiate_child()`:**

```rust
Ok(Box::new(WasmChild {
    name,
    inner: Mutex::new(WasmChildInner { store, instance }),
}))
```

**Changes to all trait methods** (same pattern for each):

```rust
fn health(&self) -> ChildHealth {
    let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
    match inner.instance.call_health(&mut inner.store) {
        // ... same match arms
    }
}

fn handle(&self, request: &ChildRequest) -> Result<ChildResponse> {
    let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
    let payload_json = serde_json::to_string(&request.payload)?;
    let result = inner
        .instance
        .call_handle(&mut inner.store, &request.action, &payload_json)?;
    // ... same match
}
```

The `name` field stays outside the Mutex — it's an immutable `String`
set at construction. No lock needed for `fn name(&self) -> &str`.

---

### F1: Registry RwLock Poison Handling

**Audit ref:** 4.3 (important)
**Location:** `src/commands/mother/registry.rs:29,41,78,81`

The file is internally inconsistent. `tick_all()` (line 55) and
`health_all()` (line 67) already handle poison gracefully:

```rust
// tick_all — graceful
if let Ok(mut child) = entry.write() { ... }

// health_all — graceful
let child = entry.read().ok()?;
```

But `register()`, `load_all()`, and `handle()` use bare `.unwrap()`:

```rust
// register — panics on poison
.any(|c| c.read().unwrap().name() == name)

// load_all — panics on poison
let mut child = entry.write().unwrap();

// handle — panics on poison (2 sites)
.find(|c| c.read().unwrap().name() == child_name)
let child = entry.read().unwrap();
```

**Fix:** Replace all 4 bare unwrap sites with `unwrap_or_else(|e| e.into_inner())`.
This matches WasmChild (`src/plugin/internal.rs`) and SecretsCacheChild
(`src/commands/mother/secrets.rs`). One pattern across the entire daemon.

**Specific changes:**

```rust
// registry.rs:29
.any(|c| c.read().unwrap_or_else(|e| e.into_inner()).name() == name)

// registry.rs:41
let mut child = entry.write().unwrap_or_else(|e| e.into_inner());

// registry.rs:78
.find(|c| c.read().unwrap_or_else(|e| e.into_inner()).name() == child_name)

// registry.rs:81
let child = entry.read().unwrap_or_else(|e| e.into_inner());
```

**Phase 2 note:** Poison recovery gives you the data, but that data may be
in an inconsistent state (e.g. repos child's `self.repos` half-updated).
Phase 2 should consider whether a poisoned child should be marked
unhealthy and removed from routing rather than silently recovered. For
now, recovery matches the existing pattern and is strictly better than
panicking the daemon.

---

### F2: Toy Deduplication

**Audit ref:** 4.6 (minor) + 6.3 (minor)
**Location:** `src/commands/mother/daemon.rs:121-164`

**The problem in detail:**

```
T=0s:   heartbeat → tick_all() → repos child returns [pull-foo, scrape-foo]
        spawn_toy(pull-foo), spawn_toy(scrape-foo) — both in background threads
T=60s:  heartbeat → tick_all() → repos child STILL returns [pull-foo, scrape-foo]
        (because last_indexed unchanged — scrape-foo is still running)
        spawn_toy(pull-foo) AGAIN, spawn_toy(scrape-foo) AGAIN
T=120s: same thing — now 6 threads doing redundant work on foo
```

**Fix:** Track in-flight toy names in the heartbeat loop. Skip toys whose
name is already running. Clean up when the thread completes.

```rust
use std::collections::HashSet;
use std::sync::{Arc, Mutex};

fn spawn_heartbeat(state: Arc<ServerState>) {
    let in_flight: Arc<Mutex<HashSet<String>>> = Arc::new(Mutex::new(HashSet::new()));

    std::thread::Builder::new()
        .name("mother-heartbeat".to_string())
        .spawn(move || loop {
            std::thread::sleep(Duration::from_secs(HEARTBEAT_INTERVAL_SECS));
            let toys = state.registry.tick_all();
            for toy in toys {
                let mut flight = in_flight.lock().unwrap_or_else(|e| e.into_inner());
                if flight.contains(&toy.name) {
                    eprintln!("[mother:toy] skipping '{}' (already in flight)", toy.name);
                    continue;
                }
                flight.insert(toy.name.clone());
                drop(flight); // release lock before spawning thread

                let in_flight_clone = Arc::clone(&in_flight);
                spawn_toy_tracked(toy, in_flight_clone);
            }
        })
        .expect("failed to spawn heartbeat thread");
}
```

The original `spawn_toy()` is replaced by `spawn_toy_tracked()` which
takes ownership of the in-flight set reference:

```rust
fn spawn_toy_tracked(toy: patina::mother::Toy, in_flight: Arc<Mutex<HashSet<String>>>) {
    let toy_name = toy.name.clone();
    let in_flight_cleanup = Arc::clone(&in_flight);

    match std::thread::Builder::new()
        .name(format!("toy-{}", toy.name))
        .spawn(move || {
            eprintln!(
                "[mother:toy] spawning '{}': {} {:?}",
                toy.name, toy.command, toy.args
            );
            match std::process::Command::new(&toy.command)
                .args(&toy.args)
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::piped())
                .status()
            {
                Ok(status) if status.success() => {
                    eprintln!("[mother:toy] '{}' completed successfully", toy.name);
                }
                Ok(status) => {
                    eprintln!("[mother:toy] '{}' failed with {}", toy.name, status);
                }
                Err(e) => {
                    eprintln!("[mother:toy] '{}' failed to spawn: {}", toy.name, e);
                }
            }
            // Remove from in-flight set when done (success or failure)
            let mut flight = in_flight_cleanup.lock().unwrap_or_else(|e| e.into_inner());
            flight.remove(&toy_name);
        })
    {
        Ok(_) => {} // thread owns cleanup via in_flight_cleanup
        Err(e) => {
            // Thread failed to spawn — remove from in-flight so it's
            // eligible for retry on next heartbeat. Don't leave the name
            // permanently stuck.
            eprintln!("[mother:toy] thread spawn failed for '{}': {}", toy_name, e);
            let mut flight = in_flight.lock().unwrap_or_else(|e| e.into_inner());
            flight.remove(&toy_name);
        }
    }
}
```

**Key properties:**
- Completed toy (success or failure) → removed from in-flight, eligible for retry
- Thread spawn failure → removed from in-flight immediately, not permanently stuck
- Daemon self-heals — no state accumulates that requires restart to clear

---

### F3: WIT Single Source of Truth

**Audit ref:** 5.3 (minor)
**Location:** `patina-plugin-api/wit/`, `patina-plugin-models/wit/`, `patina-plugin-repos/wit/`

**Current state:** Four independent copies of `mother-child.wit` and
`deps/patina-host/host.wit`. Currently byte-identical.

**Fix:** Add a CI check to `resources/git/pre-push-checks.sh` that fails
if any guest crate's WIT diverges from the canonical `wit/` directory.

Symlinks are fragile across platforms — `cargo publish` doesn't follow
them, some CI environments check them out as plain files, and Windows
WSL2 has its own symlink story. Copies with a loud CI gate are more
robust than symlinks that fail mysteriously.

```bash
# In resources/git/pre-push-checks.sh, after existing checks:

echo "Checking WIT consistency..."
wit_ok=true
for crate_dir in patina-plugin-api patina-plugin-models patina-plugin-repos; do
    if ! diff -r wit/ "$crate_dir/wit/" > /dev/null 2>&1; then
        echo "ERROR: $crate_dir/wit/ differs from canonical wit/"
        echo "  Fix: cp -r wit/ $crate_dir/wit/"
        wit_ok=false
    fi
done
if [ "$wit_ok" = false ]; then
    exit 1
fi
echo "  WIT files consistent across all crates"
```

**Alternative considered and rejected: symlinks.** Symlinks work on
macOS/Linux but create portability concerns (cargo publish, CI checkout,
Windows). The CI check is one line of defense away from the problem
(pre-push, not build-time), but it fails loudly and is universally
portable.

**Alternative considered for future: build.rs.** A `build.rs` in
`patina-plugin-api` that copies from `../wit/` at build time would
eliminate the synchronization problem entirely — one source of truth,
copies always fresh. This is the cleanest solution but adds build script
complexity. If WIT changes become frequent in Phase 2+, upgrade to this
approach.

---

## Exit Criteria

### Fixes
- [ ] F0: `unsafe impl Sync` eliminated — instance behind Mutex with store
- [ ] F0: No `unsafe` in `src/plugin/internal.rs`
- [ ] F0: All 16 plugin tests pass (same behavior, safe implementation)
- [ ] F1: Registry RwLock — all 4 bare `.unwrap()` replaced with poison recovery
- [ ] F1: Existing registry tests still pass
- [ ] F2: Toy deduplication — in-flight tracking in heartbeat loop
- [ ] F2: Duplicate toy is logged and skipped
- [ ] F2: Completed toy is removed from in-flight (eligible for retry)
- [ ] F2: Thread spawn failure removes name from in-flight (self-healing)
- [ ] F3: WIT consistency check added to `pre-push-checks.sh`
- [ ] F3: Check passes (all copies currently identical)

### Pre-push
- [ ] `cargo fmt --all`
- [ ] `cargo clippy --workspace`
- [ ] `cargo test --workspace`
- [ ] `./resources/git/pre-push-checks.sh`
- [ ] All 16 plugin tests pass
- [ ] No regressions

## Build Order

1. **F0** — Eliminate unsafe Sync first. This is a soundness fix at the
   isolation boundary. If anything breaks, we find out before touching
   other code.
2. **F3** — WIT consistency check. Build system concern, verify before
   making runtime code changes.
3. **F1** — Registry poison handling (small, self-contained).
4. **F2** — Toy deduplication (largest change, touches spawn_heartbeat
   + spawn_toy). Last because it's the most code.

## Files to Change

```
# F0 — Eliminate unsafe Sync
src/plugin/internal.rs                  # WasmChild struct + all trait methods

# F1 — Registry poison
src/commands/mother/registry.rs         # 4 unwrap sites → unwrap_or_else

# F2 — Toy deduplication
src/commands/mother/daemon.rs           # spawn_heartbeat + spawn_toy → tracked

# F3 — WIT consistency check
resources/git/pre-push-checks.sh        # Add diff check for WIT dirs
```

---

## Status Log

| Date | Status | Note |
|------|--------|------|
| 2026-02-12 | ready | Created from final audit session [[20260212-093831]]. 3 findings from audit, all concrete code fixes. Linked to both audits and both prior specs. |
| 2026-02-12 | amended | Post-audit review added F0 (eliminate unsafe Sync). Amended F2 (handle spawn failure — daemon self-healing). Changed F3 from symlinks to CI check (portability). Reordered build: F0 first (soundness), F3 second (build system), F1 third, F2 last (largest). |
