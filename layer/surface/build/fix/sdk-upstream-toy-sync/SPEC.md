---
type: fix
id: sdk-upstream-toy-sync
status: draft
created: 2026-04-06
sessions:
  origin: 20260405-133644-511306000
beliefs:
  - "[[wasi-is-foundation-not-option]]"
  - "[[pandos-are-products-children-are-compute]]"
related:
  - sdk/patina-sdk/src/toys.rs
  - sdk/patina-sdk/src/child.rs
  - wit/toys/deps/
  - wit/toys/toybox.wit
  - src/commands/mother/toys.rs
exit_criteria:

  - id: uts1-preview2-pinned
    text: "Registry pins to WASI Preview 2 as a unit. `toys-registry.toml` has `[preview2]` section with version `0.2.8` and source `https://github.com/WebAssembly/WASI`. All 7 Preview 2 proposals listed: io, clocks, random, filesystem, sockets, cli, http."
    checked: false

  - id: uts2-preview2-wit-pulled
    text: "All 7 Preview 2 WIT packages pulled from the WASI monorepo at tag `v0.2.8` (or matching release tag). Local files in `wit/toys/deps/` match upstream content. `mother toys check` all green."
    checked: false

  - id: uts3-patina-toys-reclassified
    text: "keyvalue, logging, messaging, and sql reclassified from 'WASI proposal' to 'Patina toy' in the registry. They are Patina-owned interfaces inspired by stalled WASI proposals. Source is `patina`, not upstream repos."
    checked: false

  - id: uts4-toys-status-updated
    text: "`mother toys status` shows two tiers: WASI Preview 2 (7 toys at `0.2.8`) and Patina (all others). No 'WASI proposal' tier."
    checked: false

  - id: uts5-pull-preview2
    text: "`mother toys pull --preview2` pulls all 7 Preview 2 WIT packages as a unit from the pinned release tag. Atomic: all succeed or all revert. Verifies SDK compiles after pull."
    checked: false

  - id: uts6-sync-preview2
    text: "`mother toys sync` checks the WASI monorepo for releases newer than the pinned Preview 2 version. Reports: current pin, latest stable release, latest RC. Single check for the whole Preview, not per-proposal."
    checked: false

  - id: uts7-wasi-testsuite-tracked
    text: "`wasi-testsuite` repo (github.com/WebAssembly/wasi-testsuite) cloned or referenced. Test runner can execute Preview 2 conformance tests against Mother's wasmtime configuration. At minimum: test suite runs, results reported, failures documented."
    checked: false

  - id: uts8-testsuite-passes
    text: "Mother passes the WASI Preview 2 test suite for the interfaces we implement (filesystem, http, cli at minimum). Failures for interfaces we don't yet implement (sockets, random) are documented, not hidden."
    checked: false

  - id: uts9-patina-toy-tests
    text: "Each Patina toy (keyvalue, logging, messaging, sql, git, events-stream, measure, connect, task, peer) has conformance tests written in the same style as the WASI test suite. Tests validate the toy contract against Mother's host implementation."
    checked: false

  - id: uts10-sdk-traits-match-pulled-wit
    text: "After pulling Preview 2 `0.2.8`, any SDK trait divergences from the pulled WIT are fixed. SDK traits match the upstream WIT shapes exactly."
    checked: false

  - id: uts11-compile-proof
    text: "SDK, 6 canon children (`patina-ai-child-*`), `patina-ai`, and `mother` all pass `cargo check -q`. `cargo test -q --lib` passes. WASI test suite results documented."
    checked: false
---
# fix: SDK Upstream Toy Sync

## Problem

Our toy registry pointed to scattered standalone WASI repos, four of
which are stalled proposals with no active champions. Our sync tooling
compared file hashes against inconsistent upstream sources. We called
interfaces "WASI toys" when they aren't part of any WASI standard.

The real source of truth is WASI Preview 2 — 7 proposals at `0.2.8`,
housed in the WASI monorepo, with a conformance test suite. Everything
outside Preview 2 is ours.

## Two Tiers of Toys

### WASI Preview 2 (standard — we don't own these)

7 proposals that passed phase 3, met portability criteria, and were
voted for inclusion. All at version `0.2.8`.

| Proposal | Version | We use it for |
|---|---|---|
| wasi-io | 0.2.8 | Streams, polling (underlying infra) |
| wasi-clocks | 0.2.8 | Wall clock, monotonic clock |
| wasi-random | 0.2.8 | Random number generation |
| wasi-filesystem | 0.2.8 | File I/O via std::fs + preopens |
| wasi-sockets | 0.2.8 | TCP/UDP networking |
| wasi-cli | 0.2.8 | CLI environment, stdin/stdout |
| wasi-http | 0.2.8 | HTTP outgoing requests |

These are pulled from the WASI monorepo as a unit. We match them
exactly. We prove conformance via the WASI test suite.

### Patina toys (ours — we own these)

Everything else. Some were inspired by stalled WASI proposals
(keyvalue, logging, messaging, sql). Some are purely Patina
(git, events-stream, measure, connect, task, peer). All are ours
to maintain, test, and evolve.

| Toy | Inspired by | Status of inspiration |
|---|---|---|
| keyvalue | wasi-keyvalue | Champion left (Jul 2025) |
| logging | wasi-logging | No activity 18+ months |
| messaging | wasi-messaging | Champion left (Jul 2025) |
| sql | wasi-sql | No activity 2+ years |
| git | — | Patina original |
| events-stream | — | Patina original |
| measure | — | Patina original |
| connect | — | Patina original (extends http) |
| task | — | Patina original |
| peer | — | Patina original |

If a stalled proposal becomes active again or joins a future WASI
Preview, we evaluate adopting the standard and retiring our version.
Until then, these are ours.

## Registry Format

```toml
[preview2]
source = "https://github.com/WebAssembly/WASI"
version = "0.2.8"
proposals = ["io", "clocks", "random", "filesystem", "sockets", "cli", "http"]

[preview2.io]
path = "proposals/io/wit"
upstream_files = ["streams.wit", "poll.wit", "error.wit"]
file = "io.wit"

[preview2.clocks]
path = "proposals/clocks/wit"
upstream_files = ["monotonic-clock.wit", "wall-clock.wit", "timezone.wit"]
file = "clocks.wit"

[preview2.random]
path = "proposals/random/wit"
upstream_files = ["random.wit", "insecure.wit", "insecure-seed.wit"]
file = "random.wit"

[preview2.filesystem]
path = "proposals/filesystem/wit"
upstream_files = ["types.wit", "preopens.wit"]
file = "filesystem.wit"

[preview2.sockets]
path = "proposals/sockets/wit"
upstream_files = ["tcp.wit", "udp.wit", "network.wit", "ip-name-lookup.wit"]
file = "sockets.wit"

[preview2.cli]
path = "proposals/cli/wit"
upstream_files = ["command.wit", "environment.wit", "exit.wit", "run.wit", "stdio.wit", "terminal.wit"]
file = "cli.wit"

[preview2.http]
path = "proposals/http/wit"
upstream_files = ["types.wit", "handler.wit"]
file = "http.wit"

# Patina toys
[patina-keyvalue]
source = "patina"
version = "0.2.0"
file = "keyvalue.wit"
inspired_by = "https://github.com/WebAssembly/wasi-keyvalue (stalled)"

[patina-logging]
source = "patina"
version = "0.1.0"
file = "logging.wit"
inspired_by = "https://github.com/WebAssembly/wasi-logging (stalled)"

[patina-messaging]
source = "patina"
version = "0.2.0"
file = "messaging.wit"
inspired_by = "https://github.com/WebAssembly/wasi-messaging (stalled)"

[patina-sql]
source = "patina"
version = "0.1.0"
file = "sql.wit"
inspired_by = "https://github.com/WebAssembly/wasi-sql (stalled)"

[patina-git]
source = "patina"
version = "0.1.0"
file = "patina-git.wit"

[patina-events-stream]
source = "patina"
version = "0.1.0"
file = "patina-events-stream.wit"

[patina-measure]
source = "patina"
version = "0.1.0"
file = "patina-measure.wit"

[patina-connect]
source = "patina"
version = "0.2.0"
file = "patina-connect.wit"

[patina-task]
source = "patina"
version = "0.1.0"
file = "patina-task.wit"

[patina-peer]
source = "patina"
version = "0.1.0"
file = "patina-peer.wit"
```

## Mother Commands

- `mother toys status` — shows two tiers: Preview 2 (version, 7 proposals)
  and Patina (version, inspired-by if applicable).
- `mother toys check` — verifies local WIT files match pinned hashes.
- `mother toys sync` — checks WASI monorepo for newer Preview 2 releases.
  Reports current pin vs latest stable vs latest RC.
- `mother toys pull --preview2` — pulls all 7 Preview 2 WIT packages from
  the pinned release tag as a unit. Atomic: all succeed or all revert.
  Verifies SDK compiles after pull.
- `mother toys test` — runs WASI test suite against Mother. Reports pass/fail
  per interface.

## WASI Test Suite Integration

The WASI test suite (github.com/WebAssembly/wasi-testsuite) is the
conformance proof. Mother is a WASI host — her wasmtime configuration
must pass the Preview 2 tests for interfaces we implement.

Integration approach:
1. Reference or clone wasi-testsuite
2. Run tests against Mother's wasmtime setup
3. Report results per interface
4. Failures for unimplemented interfaces (sockets, random initially)
   are documented, not hidden
5. Patina toys get their own test suite following the same patterns

## Verification

```bash
# Registry and status
patina mother toys status
patina mother toys check

# Pull Preview 2
patina mother toys pull --preview2

# SDK and children compile
cargo check -q -p patina-sdk --features child
for child in file-system-monitor content-extractor schema-enforcer \
             dedup-filter record-writer lakehouse-catalog; do
  cargo check -q -p "patina-ai-child-$child"
done
cargo check -q -p patina-ai
cargo check -q -p mother
cargo test -q --lib

# WASI conformance
patina mother toys test
```
