---
type: fix
id: sdk-upstream-toy-sync
status: draft
created: 2026-04-06
sessions:
  origin: 20260405-133644-511306000
beliefs:
  - "[[wasi-is-foundation-not-option]]"
related:
  - src/commands/mother/toys.rs
  - wit/toys/deps/toys-registry.toml
  - wit/toys/deps/
exit_criteria:

  - id: uts1-pull-command
    text: "`patina mother toys pull <name>` fetches the upstream WIT file(s) for a WASI toy, replaces the local copy, and updates the pinned hash in `toys-registry.toml`. Patina delta toys are rejected."
    checked: false

  - id: uts2-multi-file-handling
    text: "Pull handles upstream repos that split WIT across multiple files (e.g., keyvalue has `store.wit`, `atomic.wit`, `batch.wit`, `watch.wit`, `world.wit`; filesystem has `types.wit`, `preopens.wit`, `world.wit`). Local consolidated file is replaced with the upstream content we need."
    checked: false

  - id: uts3-pull-all
    text: "`patina mother toys pull --all` pulls all WASI toys in the registry. Reports which succeeded, which failed, which had no changes."
    checked: false

  - id: uts4-verify-after-pull
    text: "After pull, the command runs `cargo check -q -p patina-sdk --features child` and reports pass/fail. If check fails, prints guidance: 'upstream change broke SDK — review diff, update traits, or revert with git checkout'."
    checked: false

  - id: uts5-upstream-files-current
    text: "All 6 WASI toy WIT files updated to current upstream content. `mother toys sync` reports no diffs. `mother toys check` reports all green."
    checked: false

  - id: uts6-error-type-alignment
    text: "Where upstream WASI changed error types (e.g., keyvalue `error` from `type error = string` to `variant error { no-such-store, access-denied, other(string) }`), SDK traits and host implementations are updated to match."
    checked: false

  - id: uts7-compile-proof
    text: "SDK, 6 canon children, `patina-ai`, and `mother` all compile. `cargo test -q --lib` passes."
    checked: false
---
# fix: SDK Upstream Toy Sync

## Problem

WASI toy WIT files in `wit/toys/deps/` were hand-copied and are now stale.
`mother toys sync` shows 3 of 6 WASI toys have upstream changes (filesystem,
http, keyvalue). Our local files are missing doc comments, have simplified
error types, and may be missing interface additions.

There is no command to pull upstream changes — `toys sync` reports diffs but
`toys pull` doesn't exist. Updating requires manual curl, manual file
replacement, manual hash updates.

## Root Cause

`mother toys sync` and `mother toys check` were built but `mother toys pull`
was not. The sync/check/pull triad is incomplete.

Additionally, upstream WASI repos split WIT across multiple files (e.g.,
`store.wit` + `atomic.wit` + `world.wit`) while our local copies are
single consolidated files. Pull needs to handle this mapping.

## Current State

From `mother toys sync`:
```
diff wasi-filesystem   pinned != latest
diff wasi-http         pinned != latest
diff wasi-keyvalue     pinned != latest
ok   wasi-logging      no upstream changes
ok   wasi-messaging    no upstream changes
ok   wasi-sql          no upstream changes
```

Known keyvalue divergence: our `error` is `type error = string`, upstream
uses `variant error { no-such-store, access-denied, other(string) }`.

## Fix

Add `mother toys pull` command. Pull upstream, replace local, update hash,
verify compilation. If upstream changes break the SDK, the developer
updates traits and host implementations to match — then re-verifies.

## Verification

```bash
patina mother toys pull --all
patina mother toys sync      # should show no diffs
patina mother toys check     # should show all green
cargo check -q -p patina-sdk --features child
for child in file-system-monitor content-extractor schema-enforcer \
             dedup-filter record-writer lakehouse-catalog; do
  cargo check -q -p "patina-ai-child-$child"
done
cargo check -q -p patina-ai
cargo check -q -p mother
cargo test -q --lib
```
