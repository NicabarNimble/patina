---
type: fix
id: sdk-upstream-toy-sync
status: complete
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
    text: "All 6 WASI toy WIT files pulled at their pinned release versions. `mother toys check` all green. Local files match upstream release content."
    checked: true

  - id: uts8-trait-divergences-fixed
    text: "If pulled upstream WIT changed error types or interface shapes (e.g., keyvalue `error` from `type error = string` to `variant error`), SDK traits and host implementations are updated to match. This is separate work from the pull command itself."
    checked: true

  - id: uts9-command-tests
    text: "Tests exist for: pull rejects patina delta toys, pull updates registry hash on success, pull reverts on compile failure, check passes after pull, `--all` reports per-toy status and reverts all on compile failure. Sync tests: prerelease/draft tags filtered, `v` prefix normalized, API failure reports error and continues to next toy, rate limit handled gracefully."
    checked: true

  - id: uts10-compile-proof
    text: "SDK, 6 canon children (`patina-ai-child-*`), `patina-ai`, and `mother` all pass `cargo check -q`. `cargo test -q --lib` passes."
    checked: true
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
