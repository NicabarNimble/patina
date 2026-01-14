# Spec: Forge Sync v2

**Status:** Complete (2026-01-12)
**Created:** 2026-01-11
**Origin:** Session 20260111-163411 (spec cleanup and reorg)
**Replaces:** spec-forge-sync.md, spec-forge-repo-flag.md

---

## Problem

Forge sync takes hours for large repos (17,488 refs × 750ms = 3.6 hours). Current options:

| Option | Problem |
|--------|---------|
| `--drain` | Locks terminal for hours |
| No flag | Must manually re-run |
| `nohup ... &` | Easy to forget, no status |

**User need:** Fire and forget. Check later. No babysitting.

---

## Design Decisions

### Advisor Review

| Advisor | Guidance |
|---------|----------|
| **Eskil** | "PID files. Been working since 1971. No magic." |
| **Ng** | "Minimum intervention. Fork existing code. No new infrastructure." |
| **Gjengset** | "Two states: Running or Not. Simple to reason about." |

### Key Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Background mechanism | Fork + PID file | Unix standard, no daemon needed |
| Pacing | Fixed 750ms | Conservative, never hits limits, no API calls to check rate |
| Default behavior | Discover only | Instant, no surprise waits |
| `--drain` flag | Remove | Replace with `--sync` |
| `--full` flag | Not used | Conflicts with existing "rebuild" meaning |

---

## CLI Interface

```bash
# Discover refs (instant, default)
patina scrape forge
# → "Discovered 847 PR refs from commits"
# → "Discovered 17,488 issue refs (1..17488)"
# → "Total: 18,335 pending. Run --sync to fetch."

# Sync in background
patina scrape forge --sync
# → "Syncing in background (PID 12345)"
# → "Log: ~/.patina/logs/forge-sync-patina.log"
# → "Check: patina scrape forge --status"

# Check progress
patina scrape forge --status
# → "Syncing: PID 12345 running"
# → "Progress: 4,521 / 18,335 (25%)"
# → "Rate: ~48 refs/min"
# → "ETA: ~4.8 hours remaining"

# View log
patina scrape forge --log
# → Tails ~/.patina/logs/forge-sync-patina.log

# Foreground sync (escape hatch)
patina scrape forge --limit 50
# → Syncs 50 refs in foreground, then exits

# Target ref repo
patina scrape forge --repo claude-code --sync
```

---

## File Layout

```
~/.patina/
├── run/                              # PID files
│   ├── forge-sync-patina.pid         # Contains: 12345
│   ├── forge-sync-claude-code.pid
│   └── forge-sync-dojo.pid
│
└── logs/                             # Log files
    ├── forge-sync-patina.log
    ├── forge-sync-claude-code.log
    └── forge-sync-dojo.log
```

---

## Implementation

### Constants

```rust
// src/forge/sync/internal.rs

/// Delay between forge API requests.
///
/// GitHub allows 5,000/hour. At 750ms we do 4,800/hour max.
/// Conservative. Never hits limits. Works forever.
///
/// Eskil: "No adaptive logic. No rate limit API calls. Just works."
const DELAY_BETWEEN_REQUESTS: Duration = Duration::from_millis(750);

/// Batch size for progress reporting.
/// At 750ms delay, 50 refs = ~37 seconds per batch.
const BATCH_SIZE: usize = 50;
```

### PID File Guard

```rust
// src/forge/sync/internal.rs

fn pid_file_path(repo: &str) -> PathBuf {
    let safe_name = repo.replace('/', "-");
    dirs::home_dir()
        .unwrap()
        .join(format!(".patina/run/forge-sync-{}.pid", safe_name))
}

fn can_start_sync(repo: &str) -> Result<SyncGuard> {
    let pid_file = pid_file_path(repo);

    // Ensure directory exists
    if let Some(parent) = pid_file.parent() {
        fs::create_dir_all(parent)?;
    }

    if pid_file.exists() {
        let content = fs::read_to_string(&pid_file)?;
        if let Ok(pid) = content.trim().parse::<u32>() {
            if process_is_running(pid) {
                bail!("Already syncing (PID {}). Check: --status", pid);
            }
        }
        // Stale PID file - process died
        fs::remove_file(&pid_file)?;
    }

    Ok(SyncGuard { pid_file })
}

fn process_is_running(pid: u32) -> bool {
    // Unix: kill -0 checks if process exists
    unsafe { libc::kill(pid as i32, 0) == 0 }
}

/// RAII guard that cleans up PID file on drop
struct SyncGuard {
    pid_file: PathBuf,
}

impl SyncGuard {
    fn write_pid(&self) -> Result<()> {
        fs::write(&self.pid_file, std::process::id().to_string())?;
        Ok(())
    }
}

impl Drop for SyncGuard {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.pid_file);
    }
}
```

### Fork to Background

```rust
// src/forge/sync/internal.rs

pub fn start_background_sync(repo: &str) -> Result<()> {
    // Check if already running
    let _guard = can_start_sync(repo)?;

    let log_path = log_file_path(repo);
    let pid_path = pid_file_path(repo);

    // Fork
    match unsafe { libc::fork() } {
        -1 => bail!("Fork failed"),
        0 => {
            // Child process

            // Detach from terminal
            unsafe { libc::setsid() };

            // Redirect stdout/stderr to log
            let log_file = fs::File::create(&log_path)?;
            // ... redirect fds ...

            // Write PID file
            fs::write(&pid_path, std::process::id().to_string())?;

            // Do the work
            let result = sync_all_refs(repo);

            // Clean up PID file
            let _ = fs::remove_file(&pid_path);

            std::process::exit(if result.is_ok() { 0 } else { 1 });
        }
        child_pid => {
            // Parent process
            println!("Syncing in background (PID {})", child_pid);
            println!("Log: {}", log_path.display());
            println!("Check: patina scrape forge --status");
            Ok(())
        }
    }
}
```

### Status Check

```rust
// src/forge/sync/internal.rs

pub fn check_status(repo: &str) -> Result<SyncStatus> {
    let pid_file = pid_file_path(repo);

    // Check if process running
    let running = if pid_file.exists() {
        let pid: u32 = fs::read_to_string(&pid_file)?.trim().parse()?;
        if process_is_running(pid) {
            Some(pid)
        } else {
            // Stale PID file
            let _ = fs::remove_file(&pid_file);
            None
        }
    } else {
        None
    };

    // Get counts from database
    let conn = open_db(repo)?;
    let pending = count_pending_refs(&conn, repo)?;
    let resolved = count_resolved_refs(&conn, repo)?;
    let failed = count_failed_refs(&conn, repo)?;
    let total = pending + resolved + failed;

    Ok(SyncStatus {
        running,
        pending,
        resolved,
        failed,
        total,
    })
}
```

---

## Behavior Matrix

| Command | Action |
|---------|--------|
| `scrape forge` | Discover refs, show pending count |
| `scrape forge --sync` | Fork to background, sync all |
| `scrape forge --sync` (already running) | Error: "Already syncing (PID X)" |
| `scrape forge --status` | Show progress from DB + PID check |
| `scrape forge --log` | Tail log file |
| `scrape forge --limit N` | Foreground sync of N refs |
| `scrape forge --repo X --sync` | Background sync for ref repo |

---

## Migration

### Flags to Remove

| Flag | Replacement |
|------|-------------|
| `--drain` | `--sync` |

### Flags to Add

| Flag | Purpose |
|------|---------|
| `--sync` | Fork to background, sync all pending |
| `--log` | Tail the sync log file |
| `--limit N` | Foreground sync, cap at N refs |

### Constants to Change

| Constant | Old | New |
|----------|-----|-----|
| `DELAY_BETWEEN_REQUESTS` | 500ms | 750ms |

---

## Success Criteria

1. `patina scrape forge` returns instantly (discovery only)
2. `patina scrape forge --sync` forks and returns in <1 second
3. Only one sync per repo (PID guard)
4. `--status` shows accurate progress
5. `--log` tails the log file
6. `--limit N` works in foreground
7. Rate limit never exceeded (750ms pacing)

