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
    text: "`patina mother toys pull <name>` fetches WIT file(s) from the upstream WASI repo at the pinned release tag, replaces the local copy, and updates the pinned hash in `toys-registry.toml`. Patina delta toys are rejected."
    checked: false

  - id: uts2-sync-reports-releases
    text: "`patina mother toys sync` checks upstream WASI repos for release tags newer than the pinned version. Reports: current pinned version, latest release available, age. Does not modify files."
    checked: false

  - id: uts3-check-uses-registry-hash
    text: "`patina mother toys check` compares local file hash against registry pinned hash. Offline, no HTTP. Passes when local files match what was last pulled."
    checked: false

  - id: uts4-pull-all
    text: "`patina mother toys pull --all` pulls all WASI toys at their pinned versions. Reports per-toy: success, no-change, or failure."
    checked: false

  - id: uts5-multi-file-mapping
    text: "Each WASI toy entry in `toys-registry.toml` declares which upstream file(s) to fetch and how they map to the local file. Per-toy mapping handles repos that split WIT across multiple files."
    checked: false

  - id: uts6-pull-then-verify
    text: "After `pull`, the command runs `cargo check -q -p patina-sdk --features child`. If check fails: local files are reverted, registry hash unchanged, error printed with guidance. No half-updated state."
    checked: false

  - id: uts7-upstream-pulled
    text: "All 6 WASI toy WIT files pulled at their pinned release versions. `mother toys check` all green. Local files match upstream release content."
    checked: false

  - id: uts8-trait-divergences-fixed
    text: "If pulled upstream WIT changed error types or interface shapes (e.g., keyvalue `error` from `type error = string` to `variant error`), SDK traits and host implementations are updated to match. This is separate work from the pull command itself."
    checked: false

  - id: uts9-command-tests
    text: "Tests exist for: pull rejects patina delta toys, pull updates registry hash on success, pull reverts on compile failure, check passes after pull, `--all` reports per-toy status."
    checked: false

  - id: uts10-compile-proof
    text: "SDK, 6 canon children (`patina-ai-child-*`), `patina-ai`, and `mother` all pass `cargo check -q`. `cargo test -q --lib` passes."
    checked: false
---
# fix: SDK Upstream Toy Sync

## Problem

WASI toy WIT files in `wit/toys/deps/` were hand-copied and are stale.
`mother toys sync` shows 3 of 6 WASI toys have upstream changes. There
is no command to pull updates. Updating requires manual work.

## Pin Model

Every WASI toy pins to a release version. This is the only truth.

```toml
[wasi-keyvalue]
source = "https://github.com/WebAssembly/wasi-keyvalue"
version = "0.2.0"        # pinned release version
hash = "sha256:abc..."    # hash of local file after last pull
upstream_files = ["wit/store.wit"]  # which files to fetch from this tag
file = "keyvalue.wit"     # local consolidated file
```

- **`toys check`** — local file hash vs registry hash. Offline.
- **`toys sync`** — queries upstream repo for release tags newer than
  pinned version. Reports what's available. Does not modify files.
- **`toys pull <name>`** — fetches from the pinned release tag (e.g.,
  `v0.2.0`), replaces local file, updates registry hash. Verifies SDK
  compiles. Reverts on failure.
- **`toys pull --all`** — pulls all WASI toys at their pinned versions.

To upgrade a toy version: edit `version` in the registry, then
`toys pull <name>`. Same as bumping a version in Cargo.toml and
running cargo update.

## Multi-File Mapping

Upstream WASI repos split WIT across multiple files. Each registry
entry declares which upstream files map to our local file:

```toml
[wasi-keyvalue]
upstream_files = ["wit/store.wit"]
file = "keyvalue.wit"

[wasi-filesystem]
upstream_files = ["wit/types.wit", "wit/preopens.wit"]
file = "filesystem.wit"

[wasi-http]
upstream_files = ["wit/types.wit", "wit/handler.wit"]
file = "http.wit"

[wasi-logging]
upstream_files = ["wit/logging.wit"]
file = "logging.wit"

[wasi-messaging]
upstream_files = ["wit/messaging.wit"]
file = "messaging.wit"

[wasi-sql]
upstream_files = ["wit/readwrite.wit"]
file = "sql.wit"
```

For single-upstream-file toys: direct replacement. For multi-file toys:
concatenate in declared order with the package declaration from the
first file.

## Pull Atomicity

`toys pull` is transactional per toy:

1. Fetch upstream file(s) from pinned tag to temp
2. Compose into local file (in temp)
3. Replace local file
4. Update registry hash
5. Run `cargo check -q -p patina-sdk --features child`
6. If check passes: done
7. If check fails: revert local file to pre-pull content, restore
   registry hash, print error

`pull --all` runs each toy independently. One failure does not revert
others that succeeded. Summary reports per-toy status.

## Verification

```bash
# Pull and verify upstream
patina mother toys pull --all
patina mother toys check      # all green (local matches pinned hashes)
patina mother toys sync       # shows pinned versions vs latest available releases

# Scoped compilation (not workspace-wide)
cargo check -q -p patina-sdk --features child
for child in file-system-monitor content-extractor schema-enforcer \
             dedup-filter record-writer lakehouse-catalog; do
  cargo check -q -p "patina-ai-child-$child"
done
cargo check -q -p patina-ai
cargo check -q -p mother      # crate name is "mother", not "patina-mother"
cargo test -q --lib
```
