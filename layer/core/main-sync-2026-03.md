---
type: core
updated: 2026-03-15
---

# Main Sync 2026-03 (PR #107)

This note explains the large `patina -> main` sync PR opened as #107 so future readers can understand intent without replaying every commit.

## What This PR Is

- A branch sync PR from `patina` into `main`.
- Divergence at PR open: `main` was behind by 318 commits.
- GitHub-reported size at PR open: 405 changed files, +37543 / -6738.

## Why It Exists

- `patina` has been used as the active integration lane.
- `main` lagged while architecture/runtime/session work landed in rapid sequence.
- This PR re-baselines `main` to the current architecture and operational model.

## Major Change Waves (Most Important)

1) Knowledge-child runtime and SDK foundation (Mar 11)
- `e606f548` — knowledge-child runtime + SDK build surfaces.
- `494cfc06` — multi-agent checkpoint integration.

2) Session/interface lifecycle consolidation (Mar 11-12)
- `f10f687c` — typed JSON session results.
- `c1915169` — MCP session lifecycle tool exposure.
- `6b115082` — opencode session/spec workflow support.
- `565d4f87`, `2692445d`, `1f755e2f` — drift fixes and wrapper UX restoration.

3) SDK/path realignment and architecture refactors (Mar 13-14)
- `ca680391` — patina-sdk consolidation + path realignment.
- `7f091dee` — broker ownership + interface consolidation.
- `73b06390` — toy host centralization under `src/toys`.
- `74586ab1` — ducklake wasm-only runtime finalization.
- `822eb078` — mother boundary shim removal.

4) Guardrails and stabilization (Mar 14)
- `5b282c33` — boundary drift CI checks.
- `79900e4a` — session file-id collision fix.
- `353442a5` — init check deflake under harness.

5) SSH/tmux runtime reliability follow-up in this PR cycle (Mar 15)
- `cc440992` — launcher emits `measure.evolve` runtime-hygiene telemetry
  (wrong-folder launches, tmux clipboard/passthrough diagnostics).

## Operational Context

- During this merge window, there were tmux/socket/session recovery incidents.
- Durable state remained intact in `layer/sessions/` and events.
- A belief capturing SSH tmux clipboard reliability was added:
  `layer/surface/epistemic/beliefs/tmux-clipboard-ssh-passthrough.md`.

## How To Reconstruct This Later

Use these commands from repo root:

```bash
git rev-list --left-right --count origin/main...patina
git log --since='2026-03-11' --oneline --decorate
git log --oneline main..patina
gh pr view 107 --json title,body,commits,changedFiles,additions,deletions,url
```

## Reader Guidance

- Treat this as a baseline sync PR, not a single feature PR.
- Review by subsystem and wave, not file-by-file across entire diff.
- For intent, use commit messages + this note + PR body together.
