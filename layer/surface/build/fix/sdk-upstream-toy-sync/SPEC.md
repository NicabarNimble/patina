---
type: fix
id: sdk-upstream-toy-sync
status: active
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
    checked: true

  - id: uts2-sync-reports-releases
    text: "`patina mother toys sync` queries GitHub Releases API for each WASI toy repo. Filters to stable releases only (no prereleases, no drafts). Normalizes `v` prefix (v0.2.0 → 0.2.0). Sorts by semver. Reports per toy: pinned version, latest stable release, age. On API failure or rate limit: reports error for that toy, continues to next. Does not modify files."
    checked: true

  - id: uts3-check-uses-registry-hash
    text: "`patina mother toys check` compares local file hash against registry pinned hash. Offline, no HTTP. Passes when local files match what was last pulled."
    checked: true

  - id: uts4-pull-all
    text: "`patina mother toys pull --all` pulls all WASI toys at their pinned versions. Reports per-toy: success, no-change, or failure."
    checked: true

  - id: uts5-multi-file-mapping
    text: "Each WASI toy entry in `toys-registry.toml` declares which upstream file(s) to fetch and how they map to the local file. Per-toy mapping handles repos that split WIT across multiple files."
    checked: true

  - id: uts6-pull-then-verify
    text: "After `pull`, the command runs `cargo check -q -p patina-sdk --features child`. If check fails: local files are reverted, registry hash unchanged, error printed with guidance. No half-updated state."
    checked: true

  - id: uts7-upstream-pulled
    text: "All WASI toy WIT files pulled at their pinned versions (release tag for monorepo, commit for standalone). `mother toys check` all green."
    checked: false

  - id: uts8-trait-divergences-fixed
    text: "If pulled upstream WIT changed error types or interface shapes, SDK traits and host implementations are updated to match. This is separate work from the pull command itself."
    checked: false

  - id: uts9-command-tests
    text: "Tests exist for: pull rejects patina delta toys, pull updates registry hash on success, pull reverts on compile failure, check passes after pull, `--all` reports per-toy status and reverts all on compile failure. Sync tests: monorepo release discovery, standalone commit comparison, prerelease filtering, API failure graceful continue."
    checked: false

  - id: uts10-monorepo-sources-fixed
    text: "Registry sources corrected: filesystem, http, cli, clocks, io, random, sockets point to `WebAssembly/WASI` monorepo with `source_type = monorepo` and correct `path`. Standalone repos (keyvalue, logging, messaging, sql) have `source_type = standalone` with `commit` field."
    checked: false

  - id: uts11-new-toys-added
    text: "Registry includes all monorepo proposals: cli, clocks, io, random, sockets. WIT files pulled and added to `wit/toys/deps/`. Not all need SDK traits immediately — they are tracked for freshness."
    checked: false

  - id: uts12-sync-dual-strategy
    text: "`toys sync` uses two strategies: GitHub Releases API for monorepo toys, commit comparison (pinned commit vs HEAD) for standalone toys. Reports per toy: version/commit status, how far behind, source type."
    checked: false

  - id: uts13-compile-proof
    text: "SDK, 6 canon children (`patina-ai-child-*`), `patina-ai`, and `mother` all pass `cargo check -q`. `cargo test -q --lib` passes."
    checked: false
---
# fix: SDK Upstream Toy Sync

## Problem

WASI toy WIT files in `wit/toys/deps/` were hand-copied and are stale.
`mother toys sync` shows 3 of 6 WASI toys have upstream changes. There
is no command to pull updates. Updating requires manual work.

## Upstream Reality

WASI toys come from two places:

**WASI monorepo** (`github.com/WebAssembly/WASI`) — proposals that have
graduated to the main WASI repo. Has formal releases (latest stable:
`v0.2.10`, latest RC: `v0.3.0-rc-2026-03-15`). WIT files live under
`proposals/{name}/wit/`. Contains: cli, clocks, filesystem, http, io,
random, sockets.

**Standalone repos** (`github.com/WebAssembly/wasi-{name}`) — earlier
proposals not yet in the monorepo. No formal releases, sparse tagging.
Contains: keyvalue, logging, messaging, sql.

## Pin Model

Two pin strategies based on where the toy lives:

### Monorepo toys — pin to WASI release version

```toml
[wasi-filesystem]
source = "https://github.com/WebAssembly/WASI"
source_type = "monorepo"
path = "proposals/filesystem/wit"
version = "0.2.10"
hash = "sha256:abc..."
upstream_files = ["types.wit", "preopens.wit"]
file = "filesystem.wit"
```

`sync` checks the monorepo's GitHub Releases for newer stable versions.
`pull` fetches from the release tag.

### Standalone toys — pin to commit SHA

```toml
[wasi-keyvalue]
source = "https://github.com/WebAssembly/wasi-keyvalue"
source_type = "standalone"
path = "wit"
version = "0.2.0"
commit = "a1b2c3d4"
hash = "sha256:def..."
upstream_files = ["store.wit"]
file = "keyvalue.wit"
```

`sync` compares pinned commit against HEAD of default branch. Reports
how many commits behind. `pull` fetches from the pinned commit.

To bump: update `commit` in registry, run `toys pull`.

### Commands

- **`toys check`** — local file hash vs registry hash. Offline.
- **`toys sync`** — monorepo toys: check for newer releases. Standalone
  toys: check if pinned commit is behind HEAD. Reports per toy. Does
  not modify files.
- **`toys pull <name>`** — monorepo: fetch from release tag. Standalone:
  fetch from pinned commit. Replace local, update hash, verify SDK
  compiles, revert on failure.
- **`toys pull --all`** — pulls all WASI toys at their pinned versions.

To upgrade: edit `version` (monorepo) or `commit` (standalone) in
registry, then `toys pull <name>`.

## Full Toy Registry

### Monorepo toys (WASI stable — `WebAssembly/WASI/proposals/`)

```toml
[wasi-cli]
source = "https://github.com/WebAssembly/WASI"
source_type = "monorepo"
path = "proposals/cli/wit"
version = "0.2.10"
upstream_files = ["command.wit", "environment.wit", "exit.wit", "run.wit", "stdio.wit", "terminal.wit"]
file = "cli.wit"

[wasi-clocks]
source = "https://github.com/WebAssembly/WASI"
source_type = "monorepo"
path = "proposals/clocks/wit"
version = "0.2.10"
upstream_files = ["monotonic-clock.wit", "wall-clock.wit", "timezone.wit"]
file = "clocks.wit"

[wasi-filesystem]
source = "https://github.com/WebAssembly/WASI"
source_type = "monorepo"
path = "proposals/filesystem/wit"
version = "0.2.10"
upstream_files = ["types.wit", "preopens.wit"]
file = "filesystem.wit"

[wasi-http]
source = "https://github.com/WebAssembly/WASI"
source_type = "monorepo"
path = "proposals/http/wit"
version = "0.2.10"
upstream_files = ["types.wit", "handler.wit"]
file = "http.wit"

[wasi-io]
source = "https://github.com/WebAssembly/WASI"
source_type = "monorepo"
path = "proposals/io/wit"
version = "0.2.10"
upstream_files = ["streams.wit", "poll.wit", "error.wit"]
file = "io.wit"

[wasi-random]
source = "https://github.com/WebAssembly/WASI"
source_type = "monorepo"
path = "proposals/random/wit"
version = "0.2.10"
upstream_files = ["random.wit", "insecure.wit", "insecure-seed.wit"]
file = "random.wit"

[wasi-sockets]
source = "https://github.com/WebAssembly/WASI"
source_type = "monorepo"
path = "proposals/sockets/wit"
version = "0.2.10"
upstream_files = ["tcp.wit", "udp.wit", "network.wit", "ip-name-lookup.wit"]
file = "sockets.wit"
```

### Standalone toys (WASI proposals — individual repos)

```toml
[wasi-keyvalue]
source = "https://github.com/WebAssembly/wasi-keyvalue"
source_type = "standalone"
path = "wit"
version = "0.2.0"
upstream_files = ["store.wit"]
file = "keyvalue.wit"

[wasi-logging]
source = "https://github.com/WebAssembly/wasi-logging"
source_type = "standalone"
path = "wit"
version = "0.1.0"
upstream_files = ["logging.wit"]
file = "logging.wit"

[wasi-messaging]
source = "https://github.com/WebAssembly/wasi-messaging"
source_type = "standalone"
path = "wit"
version = "0.2.0"
upstream_files = ["messaging.wit"]
file = "messaging.wit"

[wasi-sql]
source = "https://github.com/WebAssembly/wasi-sql"
source_type = "standalone"
path = "wit"
version = "0.1.0"
upstream_files = ["readwrite.wit"]
file = "sql.wit"
```

### Patina delta toys (we own these)

```toml
[patina-git]
source = "patina"
version = "0.1.0"
wasi_overlap = "none"
file = "patina-git.wit"

[patina-events-stream]
source = "patina"
version = "0.1.0"
wasi_overlap = "wasi-messaging covers producing; consumption is our delta"
file = "patina-events-stream.wit"

[patina-measure]
source = "patina"
version = "0.1.0"
wasi_overlap = "none"
file = "patina-measure.wit"

[patina-connect]
source = "patina"
version = "0.2.0"
wasi_overlap = "extends wasi-http with credential injection"
file = "patina-connect.wit"

[patina-task]
source = "patina"
version = "0.1.0"
wasi_overlap = "none"
file = "patina-task.wit"

[patina-peer]
source = "patina"
version = "0.1.0"
wasi_overlap = "none"
file = "patina-peer.wit"
```

For single-upstream-file toys: direct replacement.

For multi-file toys, composition algorithm:
1. Fetch each file in `upstream_files` order
2. Strip duplicate `package` declarations — keep only the first
3. Reorder `use` statements to appear before the types/functions that
   reference them (topological sort within the merged output)
4. If composition produces invalid WIT (parse error), pull fails for
   that toy with the parse error message — no file replacement

If a toy's upstream layout changes (files added/removed/renamed), the
`upstream_files` list in the registry must be updated manually before
pull will succeed. `sync` does not detect layout changes — only version
changes.

## Pull Atomicity

`toys pull <name>` is transactional per toy:

1. Fetch upstream file(s) from pinned tag to temp
2. Compose into local file (in temp)
3. Replace local file
4. Update registry hash
5. Run `cargo check -q -p patina-sdk --features child`
6. If check passes: done
7. If check fails: revert local file to pre-pull content, restore
   registry hash, print error

`pull --all` pulls all toys first, then runs one compile check at the
end (not per-toy — avoids N compile cycles). If the final compile
fails, all toys are reverted and the user is told to pull individually
to isolate which toy broke.

Summary reports per-toy: `pulled` (file changed), `unchanged` (hash
matched, no fetch needed), `failed` (fetch or compose error), or
`reverted` (compile check failed, all rolled back).

## Implementation Notes

Three execution risks for the builder:

1. **Tag vs release fallback.** Some WASI repos have git tags but no
   GitHub Release objects. Sync should try Releases API first, fall back
   to tag listing (`git/refs/tags`), and report which method was used.
   If neither produces results, hard-fail for that toy — don't silently
   report "no updates."

2. **Rollback snapshot strategy.** Before `pull --all` modifies any
   files, copy all target WIT files + `toys-registry.toml` to temp.
   Revert reads from temp, not from git. This guarantees rollback even
   if the process is interrupted mid-write. Temp is cleaned up on
   success.

3. **Comment/doc preservation in composition.** Multi-file compose must
   preserve upstream comments and doc blocks verbatim. No normalization
   of whitespace or comment style. The composed output should be
   byte-stable on repeated pulls of the same version — same input
   produces same hash every time. If upstream reformats comments between
   releases, that shows as a legitimate diff in `sync`.

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
