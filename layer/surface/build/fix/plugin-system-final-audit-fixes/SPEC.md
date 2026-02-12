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

> Address 3 findings from the Phase 1 final audit (2026-02-12).
> All are concrete code fixes. No deferrals.

## Problem

The final audit (`layer/surface/reports/audit/2026-02-12-plugin-system-phase1-final.md`)
found 2 important and 4 minor findings. Of these, 3 are actionable code
fixes that directly affect Phase 1 code quality and Phase 2 readiness:

1. **Registry RwLock poison inconsistency** — 4 bare `.unwrap()` calls
   on RwLock in registry.rs, while WasmChild and SecretsCacheChild both
   use `unwrap_or_else(|e| e.into_inner())`. Same codebase, two patterns.

2. **No toy deduplication** — heartbeat spawns toys every 60s. If a toy
   takes >60s (git pull on large repo, patina scrape on 25K files), the
   next heartbeat spawns duplicates. With 25 stale repos that's 50
   duplicate threads per minute.

3. **WIT directories are copies, not symlinks** — four copies of the same
   WIT files. Future WIT changes require updating all four. Mismatches
   cause cryptic wasmtime instantiation errors.

## Source

Full audit report:
`layer/surface/reports/audit/2026-02-12-plugin-system-phase1-final.md`

Findings: 4.3 (registry poison), 4.6 + 6.3 (toy deduplication), 5.3 (WIT copies).

---

## Findings and Fixes

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

                let in_flight = Arc::clone(&in_flight);
                spawn_toy_tracked(toy, in_flight);
            }
        })
        .expect("failed to spawn heartbeat thread");
}

fn spawn_toy_tracked(toy: patina::mother::Toy, in_flight: Arc<Mutex<HashSet<String>>>) {
    let toy_name = toy.name.clone();
    std::thread::Builder::new()
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
            let mut flight = in_flight.lock().unwrap_or_else(|e| e.into_inner());
            flight.remove(&toy_name);
        })
        .expect("failed to spawn toy thread");
}
```

The original `spawn_toy()` function is replaced by `spawn_toy_tracked()`.
The `in_flight` set lives in the heartbeat thread's scope and is shared
with each toy thread via `Arc<Mutex<HashSet<String>>>`.

**Key property:** Cleanup happens unconditionally — success, failure, or
spawn error all remove the name. A failed toy is eligible for retry on the
next heartbeat.

---

### F3: WIT Symlinks

**Audit ref:** 5.3 (minor)
**Location:** `patina-plugin-api/wit/`, `patina-plugin-models/wit/`, `patina-plugin-repos/wit/`

**Current state:** Four independent copies of `mother-child.wit` and
`deps/patina-host/host.wit`. Currently byte-identical.

**Fix:** Replace the copies in guest crates with symlinks to the canonical
`wit/` directory at project root. The host-side `wit/` (used by
`wasmtime::component::bindgen!`) is the single source of truth. Guest
crates symlink to it.

```bash
# Remove copies
rm -rf patina-plugin-api/wit
rm -rf patina-plugin-models/wit
rm -rf patina-plugin-repos/wit

# Create symlinks (relative paths for portability within the repo)
cd patina-plugin-api && ln -s ../wit wit && cd ..
cd patina-plugin-models && ln -s ../wit wit && cd ..
cd patina-plugin-repos && ln -s ../wit wit && cd ..
```

**Verify:** After symlink creation:
1. `cargo build -p patina-plugin-api --target wasm32-wasip2` — guest bindgen resolves
2. `cargo build -p patina-ai` — host bindgen resolves (unchanged)
3. `cargo test -p patina-ai -- plugin` — all 16 tests pass

**Git consideration:** Git tracks symlinks as the link target path (a text
file containing `../wit`). This works on macOS and Linux. On Windows,
symlinks require developer mode or admin privileges — but Patina is
macOS/Linux-first (per `#[cfg(unix)]` usage throughout the codebase).

If symlinks prove problematic, fallback is a CI check:
```bash
# In pre-push-checks.sh
diff -r wit/ patina-plugin-api/wit/ || { echo "WIT files out of sync"; exit 1; }
```

---

## Exit Criteria

### Fixes
- [ ] F1: Registry RwLock — all 4 bare `.unwrap()` replaced with poison recovery
- [ ] F1: Existing registry tests still pass
- [ ] F2: Toy deduplication — in-flight tracking in heartbeat loop
- [ ] F2: Duplicate toy is logged and skipped
- [ ] F2: Completed toy is removed from in-flight (eligible for retry)
- [ ] F3: WIT symlinks — 3 guest crate `wit/` dirs are symlinks to `../wit`
- [ ] F3: Guest WASM compiles with symlinked WIT
- [ ] F3: Host compiles with canonical WIT (unchanged)

### Pre-push
- [ ] `cargo fmt --all`
- [ ] `cargo clippy --workspace`
- [ ] `cargo test --workspace`
- [ ] All 16 plugin tests pass
- [ ] No regressions

## Build Order

1. **F3** — WIT symlinks first (build system change, verify compilation)
2. **F1** — Registry poison handling (small, self-contained)
3. **F2** — Toy deduplication (largest change, touches spawn_heartbeat + spawn_toy)

F3 first because if symlinks break compilation, we want to know before
making code changes. F1 before F2 because F2 is the most code and should
be the final commit.

## Files to Change

```
# F1 — Registry poison
src/commands/mother/registry.rs         # 4 unwrap sites → unwrap_or_else

# F2 — Toy deduplication
src/commands/mother/daemon.rs           # spawn_heartbeat + spawn_toy → tracked

# F3 — WIT symlinks
patina-plugin-api/wit/                  # directory → symlink
patina-plugin-models/wit/               # directory → symlink
patina-plugin-repos/wit/                # directory → symlink
```

---

## Status Log

| Date | Status | Note |
|------|--------|------|
| 2026-02-12 | ready | Created from final audit session [[20260212-093831]]. 3 findings, all concrete code fixes. Linked to both audits and both prior specs. |
