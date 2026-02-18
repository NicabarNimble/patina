---
type: feat
id: spec-launcher-tmux
status: draft
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
`exec tmux new-session -A -s <name> claude`. The `-A` flag makes tmux
idempotent: attach if the session exists, create if it doesn't. One
line change in the happy path. Everything before the exec (workspace
check, mother, branch safety, bootstrap) stays identical.

## Insertion Point

`src/commands/launch/internal.rs` line 498 — `launch_adapter_cli()`.

Today:
```rust
Command::new(adapter_name).current_dir(project_path).exec()
```

After:
```rust
Command::new("tmux")
    .args(["new-session", "-A", "-s", &session_name, adapter_name])
    .current_dir(project_path)
    .exec()
```

The entire launch pipeline (steps 1-8) runs unchanged. Only step 9
changes from "exec adapter" to "exec tmux wrapping adapter."

## Design

### Session Naming

`patina_<slug>` where slug is the repo directory name, lowercased,
non-alphanumeric replaced with underscores. Deterministic — same
project always gets the same session name.

```
~/Projects/patina     → patina_patina
~/Projects/my-app     → patina_my_app
~/work/client-api     → patina_client_api
```

### TmuxMode

Add to `LaunchOptions`:

```rust
pub enum TmuxMode {
    Auto,     // Default: enable if tmux exists and $TMUX unset
    ForceOff, // --no-tmux or PATINA_TMUX=0
}
```

Resolution order:
1. `--no-tmux` flag → `ForceOff`
2. `PATINA_TMUX=0` env → `ForceOff`
3. Already inside tmux (`$TMUX` set) → skip wrapping (avoid nesting)
4. `tmux` binary not found → warn, fall back to direct exec
5. Otherwise → `Auto` (wrap in tmux)

### Reconnecting

After launch, print the attach command:

```
🚀 Launching Claude Code in patina_patina
   Reconnect: tmux attach -t patina_patina
```

From any SSH client (Termius, Blink, plain ssh): `tmux attach -t patina_patina`.

Running `patina` again in the same project also reconnects — the `-A`
flag attaches to the existing session instead of creating a new one.

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
- If already inside tmux (`$TMUX` set), skip wrapping to avoid nesting.
- No behavior changes for CI/scripts that don't have tmux installed.

## Implementation

1. Add `TmuxMode` enum and `--no-tmux` flag to `LaunchOptions` / CLI.
2. Add `derive_session_name(project_path) -> String` helper.
3. Add `detect_tmux() -> bool` (checks `which tmux` + `$TMUX`).
4. Modify `launch_adapter_cli()` to wrap in tmux when mode is `Auto`.
5. Print reconnect hint after launch.
6. Tests: session name derivation, env/flag parsing, tmux-not-found
   fallback. No real tmux needed in CI.

## Exit Criteria

1. `patina` on a machine with tmux creates a session that survives
   closing Ghostty. `tmux attach -t patina_<repo>` reconnects.
2. `patina` in a project with an existing tmux session reattaches
   instead of creating a second session.
3. `--no-tmux` and `PATINA_TMUX=0` restore direct-exec behavior with
   zero regressions in branch safety or bootstrap generation.
4. Without tmux installed, `patina` warns and launches the adapter
   directly (no crash, no error exit).
5. Session name derivation is deterministic and tested.
