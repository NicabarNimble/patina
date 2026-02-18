---
type: feat
id: spec-launcher-tmux
status: active
created: 2026-02-18
updated: 2026-02-18
related:
- layer/surface/build/feat/mother-design/SPEC.md
beliefs:
- patina-identity
- unix-philosophy
---

# feat: Launcher tmux Default — Resumable Sessions

> Wrap the launcher's `exec` with tmux so every `patina` session survives
> terminal close and can be reattached from any SSH client.

## Problem

Running `patina` execs the adapter CLI directly. Closing Ghostty (or any
terminal) kills the adapter process. There is no way to reconnect to the
session from another device. This is a hard blocker for mobile/remote
workflows — you can't plan when you'll need to step away from the desk.

## Solution

Replace the bare `exec claude` in `launch_adapter_cli()` with
`exec tmux new-session -A -s patina_<slug>_<hash> -c <path> claude`.
The `-A` flag makes tmux idempotent: attach if the session exists,
create if it doesn't. The `-c` flag explicitly sets the session's
start directory to the project root, ensuring the adapter boots in
the correct repo regardless of tmux server state. One line change in
the happy path. Everything before the exec (workspace check, mother,
branch safety, bootstrap) stays identical.

## Insertion Point

`src/commands/launch/internal.rs` line 498 — `launch_adapter_cli()`.

Today:
```rust
Command::new(adapter_name).current_dir(project_path).exec()
```

After (showing the full conditional flow):
```rust
let decision = resolve_tmux_decision(/* 6 bools */);
// Caller emits warnings for Off(NotInPath) / Off(TmuxTooOld)

match decision {
    TmuxDecision::Auto => {
        eprintln!("Launching {} in tmux session: {}", adapter_name, session_name);
        eprintln!("  Reconnect: tmux attach -t {}", session_name);
        std::io::stderr().flush().ok();

        let err = Command::new("tmux")
            .args(["new-session", "-A", "-s", &session_name, "-c"])
            .arg(project_path.as_os_str())  // non-UTF-8 safe
            .arg(adapter_name)
            .current_dir(project_path)
            .exec();
        // exec only returns on error — fall back to direct launch
        eprintln!("Warning: failed to exec tmux — launching {} directly", adapter_name);
        // fall through to direct exec below
    }
    TmuxDecision::Off(_) => {
        println!("\nLaunching {}...\n", adapter_name);
    }
}
// Direct exec (ForceOff path, or Auto fallback after tmux exec failure)
Command::new(adapter_name).current_dir(project_path).exec()
```

The `-c` path is passed via `.arg(OsStr)` — not `.to_str()` — so
non-UTF-8 paths (legal on Linux) work natively without lossy conversion
or silent fallback. Both `-c` and `.current_dir()` set the working
directory — belt and suspenders. `-c` tells tmux explicitly;
`.current_dir()` ensures the tmux client process starts in the right
place.

Requires tmux ≥ 1.9 (released 2014) for the `-c` flag. Patina
proactively checks `tmux -V` before exec and falls back to direct
adapter launch if the version is too old (see resolution step 6).

**exec semantics:** `CommandExt::exec()` replaces the Patina process
with tmux. If exec succeeds, Patina is gone — runtime tmux errors
(bad config, socket issues) happen inside tmux and produce tmux's
own error output, not Patina's. The only errors Patina can catch are
exec-level failures (binary not found, not executable, permission
denied). For those, Patina prints a retraction warning and falls
back to direct adapter exec. For runtime tmux errors, the user sees
tmux's error and can re-run with `--no-tmux` or `PATINA_TMUX=0`.

The entire launch pipeline (steps 1-8) runs unchanged. Only step 9
changes from "exec adapter" to "exec tmux wrapping adapter."

## Design

### Session Naming

`patina_<slug>_<hash>` where slug is the repo directory name converted
via `to_string_lossy()` (non-UTF-8 bytes become `U+FFFD`, which the
slug step replaces with underscore — intentional lossy conversion for
the human-readable part), then lowercased, non-alphanumeric replaced
with underscores, and truncated to 50 chars.

Hash is 8 hex chars (32 bits) computed via **FNV-1a** over the raw
path bytes (`as_os_str().as_encoded_bytes()`):

```rust
fn fnv1a_32(bytes: &[u8]) -> u32 {
    let mut hash: u32 = 2_166_136_261; // FNV offset basis
    for &b in bytes {
        hash ^= b as u32;
        hash = hash.wrapping_mul(16_777_619); // FNV prime
    }
    hash
}
// Output: format!("{:08x}", fnv1a_32(path_bytes))
```

FNV-1a is deterministic, platform-independent, and needs no crate.
The 8-char zero-padded lowercase hex output is the canonical format.

Deterministic — same project always gets the same session name. The
32-bit hash makes collisions extremely unlikely for practical use
(~4 billion buckets; birthday-paradox 50% threshold is ~65,000
projects). Collisions are not impossible but are statistically
negligible for any real user's project count. No collision-handling
mechanism is provided; if two projects happen to collide, one user
can rename the directory or use `--no-tmux`.

Maximum name length: `patina_` (7) + slug (≤50) + `_` (1) + hash (8) =
66 chars. Well under tmux's 200-byte `TMUX_NAME_LIMIT`. Truncation is
applied to the slug before hashing; the hash is always derived from the
full path so truncated slugs still produce unique names.

The path used for both slug and hash is `project_path` — the canonical
project root returned by `resolve_project_path()` in launch step 1.
This is always the repo root, never a subdirectory. Running `patina`
from `~/Projects/patina/examples/foo` still derives the name from
`~/Projects/patina`. On macOS, `/tmp/foo` canonicalizes to
`/private/tmp/foo` — this is correct (same physical path = same hash).

If `canonicalize()` fails (broken symlink, permission error), fall back
to the un-canonicalized path for hashing. Launch never panics over a
session name.

```
~/Projects/patina     → patina_patina_a3f2b81c
~/Projects/my-app     → patina_my_app_7c1be4d9
~/work/client-api     → patina_client_api_e4d90a2f
~/personal/client-api → patina_client_api_b81a3c7e
```

### TmuxDecision

Add to `LaunchOptions`:

```rust
pub enum TmuxDecision {
    Auto,
    Off(OffReason),
}

pub enum OffReason {
    CliFlag,     // --no-tmux
    EnvVar,      // PATINA_TMUX=0
    NoTty,       // stdout is not a terminal
    InsideTmux,  // $TMUX is set
    NotInPath,   // tmux binary not found
    TmuxTooOld,  // tmux < 1.9 (no -c flag)
}
```

The `OffReason` tells the caller *why* tmux was disabled, enabling
targeted messaging. The caller maintains the detected version string
separately (from the `tmux -V` probe) and pairs it with `TmuxTooOld`
when formatting warnings — no payload needed in the enum, no re-probe.

```rust
// Caller pattern:
let (version_ok, detected_version) = check_tmux_version();
let decision = resolve_tmux_decision(/* ..., */ version_ok);
match decision {
    Off(NotInPath) => eprintln!("Warning: tmux not found"),
    Off(TmuxTooOld) => eprintln!("Warning: tmux {} too old (need ≥ 1.9)", detected_version),
    Off(_) => {} // silent
    Auto => { /* tmux wrapping */ }
}
```

Resolution order (checked by `resolve_tmux_decision()`):
1. `--no-tmux` flag → `Off(CliFlag)`
2. `PATINA_TMUX=0` env → `Off(EnvVar)`
3. stdout not a TTY (`!std::io::stdout().is_terminal()`) → `Off(NoTty)`
   (CI, pipes, scripts — tmux refuses without a controlling terminal.
   No force-on override for piped stdout; the launcher is interactive.)
4. Already inside tmux (`$TMUX` set) → `Off(InsideTmux)` (avoid nesting.
   No force-on flag for nested tmux — if someone truly needs nesting,
   they can `tmux new-session` manually outside patina.)
5. `tmux` not in PATH (`which::which("tmux")` fails) → `Off(NotInPath)`.
   Uses the `which` crate (already in dep tree), not shell-out.
   Caller pattern-matches `Off(NotInPath)` to warn to stderr.
6. tmux version check → caller runs `check_tmux_version()` (see
   Implementation step 3) and passes `tmux_version_ok: bool` to the
   pure function. Version check rules:
   - **Success, parseable:** output like "tmux 3.4" or "tmux 1.8a" —
     parse major.minor, require ≥ 1.9. If < 1.9: `(false, "1.8")`.
   - **Success, unparseable:** unexpected format — assume ok (don't
     block on novel version strings). `(true, "unknown")`.
   - **Failure** (I/O error, non-zero exit, empty output) — treat as
     too old: `(false, "unknown (tmux -V failed)")`. Conservative:
     if we can't verify the version, don't exec it.
   Caller pairs the version string with `Off(TmuxTooOld)` for the
   warning: "tmux 1.8 too old (need ≥ 1.9)" or "tmux version unknown
   (tmux -V failed), skipping tmux".
7. Otherwise → `Auto` (wrap in tmux)

### Reconnecting

**When tmux is active (Auto mode):** Before exec, print the session
name and attach command to stderr (`eprintln!`) so the hint survives
even if stdout is redirected. Flush stderr explicitly before exec to
guarantee delivery.

```
Launching Claude Code in tmux session: patina_patina_a3f2b81c
  Reconnect: tmux attach -t patina_patina_a3f2b81c
```

If `exec()` returns (binary not found/executable/permission denied),
print a retraction before falling back to direct adapter exec:

```
Warning: failed to exec tmux — launching claude directly (no session created)
```

This prevents the user from seeing a reconnect hint for a session
that doesn't exist. Note: runtime tmux errors (bad config, socket
issues) happen after exec succeeds, so Patina can't catch those —
tmux prints its own error and the user re-runs with `--no-tmux`.
(Old tmux versions are caught proactively by the `tmux -V` probe
in resolution step 6, before exec is attempted.)

From any SSH client (Termius, Blink, plain ssh):
`tmux attach -t patina_patina_a3f2b81c`.

Running `patina` again in the same project also reconnects — the `-A`
flag attaches to the existing session instead of creating a new one.
There is no "print and exit" path: `patina` always puts you in the
session. It's idempotent.

**When tmux is disabled (Off):** Print only the existing
`Launching <adapter>...` message. No reconnect hint — there is no
tmux session to reconnect to.

### What Stays the Same

- All launch steps 1-8 (workspace, mother, project check, branch
  safety, allowed adapters, MCP config, bootstrap)
- Mother starts as a background daemon (line 53) — it already survives
  terminal close. tmux doesn't change this.
- The adapter runs as the tmux session's main process — when the adapter
  exits, the tmux session closes naturally.
- `#[cfg(not(unix))]` path (line 512) is dead code per patina-identity
  (macOS/Linux only) but left as-is — not this spec's concern.

## Non-Goals

- **`ForceOn` / `PATINA_TMUX=1`** — no flag to force tmux when inside
  tmux (nesting) or when stdout is piped. The launcher is interactive;
  piped output from an interactive adapter doesn't make sense. Users
  who need nested tmux can `tmux new-session` manually.
- Multiple tmux windows/panes (adapter + mother logs). Mother already
  runs as a daemon. One pane with the adapter is sufficient.
- `patina resume` command. `tmux attach -t <name>` or just re-running
  `patina` in the same directory already works via `-A`.
- iOS detection heuristics, layout presets, status-bar customization.
  Those are polish for a future spec after living with the core change.
- Supporting multiplexers other than tmux (screen, zellij).

## Rollback & Safety

- `--no-tmux` flag and `PATINA_TMUX=0` env disable wrapping instantly.
- If tmux is not installed, warn and fall back to direct exec — launch
  never fails because of tmux.
- If `exec tmux` fails at the OS level (binary not found despite
  `which` check, permission denied), print retraction and fall back
  to direct `exec adapter`. Launch never fails because of tmux.
- If tmux is too old (< 1.9, no `-c` flag), Patina detects this via
  `tmux -V` probe before exec and falls back to direct adapter launch
  with a helpful warning ("tmux X.Y too old, need ≥ 1.9").
- If tmux starts but fails at runtime (bad config, socket permission),
  tmux prints its own error. The user sees it and can re-run with
  `--no-tmux` or `PATINA_TMUX=0`. Patina cannot catch these errors
  because `exec()` has already replaced the process.
- If already inside tmux (`$TMUX` set), skip wrapping to avoid nesting.
- If stdout is not a TTY (CI, pipes, scripts), skip wrapping — tmux
  refuses to run without a controlling terminal.
- No behavior changes for CI/scripts that don't have tmux installed.

## Implementation

1. Add `TmuxDecision`/`OffReason` enums and `--no-tmux` flag to
   `LaunchOptions` / CLI (`src/main.rs` Cli struct +
   `src/commands/launch/mod.rs`).
2. Add `derive_session_name(project_path: &Path) -> String` — slug
   from dir name via `to_string_lossy()` (lossy for slug), truncated
   to 50 chars, + 8-char hex hash via FNV-1a 32-bit over raw path
   bytes (`as_os_str().as_encoded_bytes()`). Output format:
   `format!("patina_{}_{:08x}", slug, fnv1a_32(path_bytes))`.
   Input is the already-canonicalized `project_path` from launch
   step 1. If caller couldn't canonicalize, pass the original path.
3. Add `resolve_tmux_decision(cli_no_tmux, env_disabled, is_tty,
   inside_tmux, tmux_in_path, tmux_version_ok) -> TmuxDecision` —
   pure function taking all 6 environmental inputs as bool parameters,
   one per resolution step:
   - `cli_no_tmux`: step 1 → `Off(CliFlag)`
   - `env_disabled`: step 2 → `Off(EnvVar)`
   - `is_tty`: step 3 → `Off(NoTty)` when false
   - `inside_tmux`: step 4 → `Off(InsideTmux)` when true
   - `tmux_in_path`: step 5 → `Off(NotInPath)` when false
   - `tmux_version_ok`: step 6 → `Off(TmuxTooOld)` when false
   Add helper `check_tmux_version() -> (bool, String)` that runs
   `tmux -V`, parses "tmux X.Y" for ≥ 1.9, and returns (ok, version).
   On parse failure: `(true, "unknown")` (assume ok). On I/O error
   or non-zero exit: `(false, "unknown (tmux -V failed)")` (assume
   too old — conservative). The caller holds the version string and
   pairs it with `Off(TmuxTooOld)` for warnings. The pure function
   stays pure. All 7 branches unit-testable.
4. Modify `launch_adapter_cli()` to accept `TmuxDecision` and session
   name. When `Auto`: print reconnect hint to stderr (`eprintln!`),
   flush stderr, exec tmux with `-c project_path` (via `OsStr`,
   non-UTF-8 safe). If `exec()` returns (= kernel couldn't start
   tmux), print retraction and fall back to direct `exec adapter`.
   Note: runtime tmux errors (bad config, socket issues) happen after
   exec succeeds — Patina is gone and tmux prints its own errors.
   (Old versions are caught proactively by `tmux -V` in step 3.)
   When `Off(_)`: print "Launching <adapter>..." and exec adapter
   directly (current behavior, no reconnect hint).
5. Tests: session name derivation (deterministic, FNV-1a exact values,
   slug truncation at 50 chars, different paths with same dir name),
   mode resolution (all 7 branches via 6-parameter injection — no
   real tmux or TTY needed in CI).

## Exit Criteria

1. `patina` on a machine with tmux creates a session that survives
   closing Ghostty. `tmux attach -t patina_<slug>_<hash>` reconnects.
2. `patina` in a project with an existing tmux session reattaches
   instead of creating a second session (idempotent via `-A`).
3. tmux session starts in the correct project root regardless of
   tmux server state (verified by `-c` flag).
4. `--no-tmux` and `PATINA_TMUX=0` restore direct-exec behavior with
   zero regressions in branch safety or bootstrap generation. No
   reconnect hint printed when tmux is disabled.
5. Without tmux installed, `patina` warns to stderr and launches the
   adapter directly (no crash, no error exit).
6. If `exec tmux` fails at OS level (permission denied, binary
   corrupted), `patina` prints retraction and falls back to direct
   adapter exec. Runtime tmux errors (bad config) produce tmux's own
   error output; user uses `--no-tmux` to work around.
6a. If tmux version < 1.9 (detected via `tmux -V` probe), `patina`
    warns with version info and launches adapter directly — user
    doesn't have to discover `--no-tmux` by trial and error.
7. In CI (no TTY), `patina` skips tmux wrapping automatically.
8. Inside an existing tmux session, `patina` skips wrapping (no nesting).
9. Session name derivation is deterministic and tested — two projects
   with the same directory name but different paths produce different
   session names with extremely high probability (FNV-1a 32-bit,
   ~65K projects before 50% birthday collision). Slug truncated to
   50 chars; total name ≤ 66 chars. Hash algorithm, offset basis,
   and output format are specified so tests can assert exact values.
10. `resolve_tmux_decision()` returns `TmuxDecision` with `OffReason`
    — all 7 branches testable via 6-parameter injection. Caller
    pattern-matches reason for targeted warnings (`NotInPath` and
    `TmuxTooOld` warn; others are silent).
