---
id: spec-preflight
status: implemented
created: 2026-01-14
tags: [spec, startup, self-healing, processes]
references: [dependable-rust, unix-philosophy]
---

# Spec: Preflight Process Cleanup

**Problem:** Stale `patina serve` processes cause mysterious OOM kills on startup. Users see `[1] 52154 killed patina` with no explanation.

**Goal:** Self-healing startup that automatically cleans up stale processes and tells the user what happened.

---

## Background

### The Failure Mode

1. User runs `patina` which spawns `patina serve` (mothership)
2. Session ends, but serve process keeps running
3. Days later, user runs `patina`
4. Two `patina serve` processes conflict, or system resources exhausted
5. OS kills process with SIGKILL (exit 137)
6. User sees cryptic `killed` message, no diagnostics

### Why Doctor Is Wrong

Initial implementation added detection to `patina doctor`. Problems:

1. **Reactive, not proactive** - User must remember to run doctor
2. **Wrong category** - Doctor checks *project* state (files, config), not *runtime* state (processes)
3. **Doesn't fix** - Just reports, user must manually kill

### The Gjengset Approach

Jon Gjengset's philosophy: **Don't make the user fix it. Fix it for them.**

- Self-healing where safe
- Fail fast, fail clearly
- No silent magic (tell user what you did)

---

## Design

### Do X Test

**"Ensure system is ready to run patina"** - Clear, single responsibility.

### Module Location

```
src/
├── preflight.rs          # New: ensure_clean_state()
└── main.rs               # Calls preflight before CLI dispatch
```

Not in `commands/` - this isn't a user command. Not in `health/` - we're not building a health subsystem. Simple top-level module with one job.

### Interface

```rust
// src/preflight.rs

/// Ensure system is ready to run patina.
/// Kills stale processes (>24h) and reports what was cleaned.
/// Returns Ok(()) - cleanup failures are warnings, not errors.
pub fn ensure_clean_state() {
    let stale = find_stale_patina_processes(Duration::hours(24));
    for proc in stale {
        if kill_process(proc.pid).is_ok() {
            eprintln!("Cleaned up stale {} (PID {}, running {})",
                proc.command, proc.pid, proc.elapsed);
        }
    }
}
```

### Behavior

| Condition | Action |
|-----------|--------|
| No stale processes | Silent, continue |
| Stale process killed | Print one line, continue |
| Kill fails (permission) | Print warning, continue |
| ps command fails | Silent, continue (not critical) |

**Key:** Preflight never fails the command. It's best-effort cleanup.

### What Counts as Stale?

- Process name contains "patina"
- Running for >24 hours
- Excludes current process (the one running preflight)

24 hours is conservative. A `patina serve` should restart with each session, so anything >24h is definitely stale.

### Output Format

```
$ patina
Cleaned up stale patina serve (PID 12345, running 2d3h)
🚀 Launching Claude Code in /path/to/project
```

Single line, then normal operation. Not noisy.

---

## Implementation

### Phase 1: Core (~50 lines)

```rust
// src/preflight.rs
use std::process::Command;
use std::time::Duration;

const STALE_THRESHOLD_HOURS: u64 = 24;

pub fn ensure_clean_state() {
    let threshold = STALE_THRESHOLD_HOURS * 60; // minutes
    for proc in find_stale_processes(threshold) {
        cleanup_process(&proc);
    }
}

struct StaleProcess {
    pid: u32,
    command: String,
    elapsed_minutes: u64,
}

fn find_stale_processes(threshold_minutes: u64) -> Vec<StaleProcess> {
    // Use: ps -eo pid,etime,command
    // Parse elapsed time format: [[dd-]hh:]mm:ss
    // Filter: contains "patina", elapsed > threshold, not current PID
    todo!()
}

fn cleanup_process(proc: &StaleProcess) {
    // Kill and report
    todo!()
}
```

### Phase 2: Wire into main.rs

```rust
// src/main.rs
mod preflight;

fn main() -> Result<()> {
    preflight::ensure_clean_state();

    // ... existing CLI dispatch
}
```

### Phase 3: Optional doctor integration

Doctor can optionally report stale processes for `--json` consumers, but doesn't need to since preflight handles cleanup automatically.

---

## Core Values Alignment

### Unix Philosophy

- **One tool, one job**: `preflight.rs` does exactly one thing
- **Compose with Unix tools**: Uses `ps`, not a process library
- **Text interface**: Output is human-readable

### Dependable Rust

- **Small interface**: One public function `ensure_clean_state()`
- **Black box**: Implementation (ps parsing) hidden from callers
- **No leaky abstractions**: main.rs doesn't know about PIDs or elapsed times

---

## Testing

### Manual Test

```bash
# Start a fake stale process
(exec -a "patina serve --test" sleep 86500) &

# Run patina (should clean up)
patina --help

# Expected output:
# Cleaned up stale patina serve --test (PID 12345, running 1d0h)
# [normal help output]
```

### Unit Tests

- `parse_elapsed("2-03:45:12")` → 3105 minutes
- `parse_elapsed("45:30")` → 45 minutes
- `format_elapsed(1445)` → "1d0h"

---

## Rejected Alternatives

### PID File

Could write `.patina/local/serve.pid` and check staleness.

**Rejected:** More state to manage, can get out of sync with reality. `ps` is ground truth.

### Doctor Only

Just report in doctor, let user fix.

**Rejected:** Users don't run doctor proactively. Self-healing is better UX.

### sysinfo Crate

Use Rust library instead of `ps`.

**Rejected:** Adds dependency, `ps` is universal on Unix, follows Unix philosophy.

---

## Success Criteria

- [x] Stale processes (>24h) killed automatically on startup
- [x] User sees one-line message explaining cleanup
- [x] Normal operation continues after cleanup
- [x] No new dependencies
- [x] <50 lines of code
