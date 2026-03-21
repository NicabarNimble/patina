# Design: Patina Pre-v1 — Full Architecture Conversion

> The SPEC says what and why. This document says how, exactly. Each phase has a commit plan with file paths, changes, and verification. A build agent executes these commits mechanically. Per [[spec-driven-design]]: code executes what specs decide.

> **Scalpel, not shotgun.** One logical change per commit. Read before write. Move first, adapt second, add third. `cargo test` after every commit — with one documented exception: move+fix commit pairs (e.g., `git mv` then fix imports) where the first commit may break compile. The pair must be consecutive and the second commit must restore green. No drive-by refactors.

## Why This Design

Patina's architecture needs to match its beliefs. This design closes the gap in 11 phases. Phases 1-10 ship today (64 commits). Phase 11 is follow-on (12 commits). The ordering: SDK first (onramp), plumbing second (worlds, linker), capabilities third (toys), proof fourth (session-writer, ducklake), architecture fifth (Mother, CLI, MCP retirement), polish sixth (relationships, template).

---

## Phase 1: SDK Restructure — The Map (6 commits)

### Why
The SDK is what external developers see first. Today it's one flat crate. The tiered SDK makes the onramp obvious: start with `core`, add `data` or `agent` as needed.

### Approach
Extract, don't rewrite. Split existing code into crates by concern, add feature gates.

### Commits

1. `sdk: scaffold patina-sdk-core crate` — Create `sdk/patina-sdk-core/Cargo.toml` and `src/lib.rs`. Move `KnowledgeChildPlugin` trait, `WasmCell`, `register_knowledge_child!` macro, `LogBackend`/`StateBackend` traits + ZST wrappers. Dependencies: `wit-bindgen` only.

2. `sdk: scaffold patina-sdk-data crate` — Create `sdk/patina-sdk-data/Cargo.toml` and `src/lib.rs`. Move `LakeBackend`, `CheckpointBackend`, `MeasureBackend`, `ConnectorBackend`, `IngressBackend` traits + ZST wrappers. Feature-gated: `toy-lake`, `toy-checkpoint`, `toy-measure`, `toy-github` (stub), `toy-connector` (legacy). Dependencies: `patina-sdk-core`, `serde`, `serde_json`.

3. `sdk: scaffold patina-sdk-agent crate` — Create `sdk/patina-sdk-agent/Cargo.toml` and `src/lib.rs`. Move `QueryBackend`, `EmitBackend` traits + ZST wrappers. Feature-gated: `toy-query`, `toy-session` (stub), `toy-emit`. Dependencies: `patina-sdk-core`.

4. `sdk: refactor patina-sdk as re-export umbrella` — Rewrite `sdk/patina-sdk/src/lib.rs` to re-export all three tiers. Existing children depend on `patina-sdk` unchanged. All tests pass.

5. `sdk: feature-gate granted module` — Wrap toy factory functions in `#[cfg(feature = "toy-*")]` gates. Default features in `patina-sdk` enable everything (backward compat).

6. `sdk: add per-toy features to child Cargo.tomls` — Update ducklake and belief-verifier `Cargo.toml` with explicit features. Verify both compile. All tests pass.

### Verification
- `cargo build -p patina-sdk-core` succeeds
- `cargo build -p patina-sdk-data --features toy-lake` succeeds
- `cargo build -p patina-sdk-agent --features toy-query` succeeds
- Both children compile with explicit features
- All existing tests pass

---

## Phase 2: Per-Child WIT Worlds (6 commits)

### Why
The monolithic world imports all 14 interfaces for every child. Per-child worlds make the WIT definition match actual needs — the world IS the sandbox.

### WIT Governance: Source of Truth Rule

**`wit/toys/*.wit` files are the canonical source.** Hand-edited. The old monolithic `host.wit` is deleted after extraction.

**`wit/worlds/*.wit` files are hand-edited per-child.** World files only import — they never redefine toy interfaces. SDK tier crates copy toy WIT files from `wit/toys/` via build scripts.

### Toolchain Decision

`cargo-component` supports custom world paths via `Cargo.toml` configuration (`[package.metadata.component]` section). If this doesn't work, WAC composes worlds as a build-time pre-step. Decision confirmed at commit 9 — if it fails, pivot to WAC before commit 10.

### Commits

7. `wit: split host.wit into individual toy interfaces` — Extract each interface into `wit/toys/`: `log.wit`, `state.wit`, `lake.wit`, `checkpoint.wit`, `measure.wit`, `query.wit`, `http.wit`, `emit.wit`, `ingress.wit`, `connector.wit`, `events.wit`, `task.wit`, `graph.wit`, `belief.wit`, `types.wit`. Delete monolithic `host.wit`.

8. `wit: create per-child world files` — Create `wit/worlds/ducklake.wit` (7 imports), `wit/worlds/belief-verifier.wit` (6 imports). Each exports `handle`/`tick`/`drain`/`health`.

9. `wit: update ducklake build to use ducklake.wit` — Change `cargo-component` config to point at `wit/worlds/ducklake.wit`. Verify compilation succeeds. **This is the toolchain proof — if `cargo-component` can't handle it, pivot to WAC here.**

10. `wit: update belief-verifier build to use belief-verifier.wit` — Same for belief-verifier.

11. `wit: wire SDK tier crates to use toy WIT files` — Build scripts in each SDK tier crate copy needed WIT files from `wit/toys/` at compile time.

12. `wit: measure binary size reduction` — Release builds. Compare against Phase 1 baseline. Document delta.

### Verification
- Both children compile with per-child worlds
- Release binary sizes decrease
- All existing tests pass

---

## Phase 3: Per-Child Linker (4 commits)

### Why
Mother's shared linker links all 14 interfaces for every child. Per-child linking makes the sandbox compile-time real.

### Manifest Schema

Current `[capabilities]`/`[toys]` → new `[needs].toys` + `[needs.scopes]`. Migration order: commit 14 adds `[needs]` parsing alongside old schema (reads both, prefers `[needs]`). Commit 15 rewrites child `plugin.toml` files to the new `[needs]` schema. Old `[capabilities]`/`[toys]` parsing code removed in Phase 9 commit 52 (alongside pipe crate removal — both are legacy cleanup).

### Commits

13. `host: split add_to_linker into per-interface functions` — Extract each `impl Host for HostState` block into `link_log()`, `link_state()`, `link_lake()`, etc. Shared linker calls all (no behavior change).

14. `host: read manifest [needs].toys` — Parse `[needs].toys` as `Vec<String>` alongside existing schema. Prefer `[needs]` if present.

15. `host: build per-child linker from manifest + migrate plugin.tomls` — Fresh `Linker<HostState>` per-child. Only link declared toys. Missing import → wasmtime instantiation error. Also rewrite ducklake and belief-verifier `plugin.toml` files from old `[capabilities]`/`[toys]` schema to new `[needs].toys` + `[needs.scopes]`.

16. `host: verify sandbox enforcement` — Test: child imports `lake` but manifest omits it → fails. Child with `lake` in manifest → succeeds.

### Verification
- Existing children load (manifests include actual toys)
- Sandbox test passes
- All tests pass

---

## Phase 4: New Toy Interfaces (7 commits)

### Why
GitHub data access and session management don't exist as toys yet. Both need WIT interfaces.

### Commits

17. `wit: define patina:host/github@0.1.0` — Typed records for issue, PR, comment, review, event. Paginated list functions. `list-params`, `page<T>` with `rate-remaining`.

18. `host: implement github toy` — `src/toys/github.rs`. GitHub REST API, credential injection from `HostState`, rate-limit tracking, pagination. Absorb from native connector. Add `link_github()`.

19. `host: add github toy tests` — Record API responses to `tests/fixtures/github-api/` and test against fixtures (deterministic, no network). Tests cover pagination, rate-limit backoff, credential injection, all 8 entity types. Live API tests gated behind `#[ignore]` + `GITHUB_TOKEN` env var.

20. `wit: define patina:host/session@0.1.0` — `write-artifact`, `create-tag`, `set-status`, `write-handoff`, `get-previous-session`, `get-session-id`.

21. `host: implement session toy` — `src/toys/session.rs`. Absorb from `src/session/internal/live.rs`. Scoped to `layer/sessions/`. Add `link_session()`.

22. `host: add session toy tests` — Artifact writes, real git tags, crash handoff.

23. `sdk: add toy-github and toy-session to SDK tiers` — `GithubBackend` in `patina-sdk-data`. `SessionBackend` in `patina-sdk-agent`. Wire `granted::` factories.

### Verification
- GitHub toy tests pass with fixtures
- Session toy tests pass with real git tags
- SDK compiles with new features
- All tests pass

---

## Phase 5: Session-Writer Child (9 commits)

### Why
First child on the new model. Proves minimal child (<150KB), per-child worlds end-to-end, crash recovery.

### Commits

24. `wit: create session-writer world` — `wit/worlds/session-writer.wit`: `log`, `state`, `session`, `types`. Exports: `handle`, `health`.

25. `child: scaffold session-writer` — `children/session-writer/Cargo.toml` → `patina-sdk-core` + `patina-sdk-agent[toy-session]`. Minimal `KnowledgeChildPlugin` impl.

26. `child: implement handle actions` — `"note"`, `"update"`, `"spec-link"`, `"close"`, `"crash-handoff"`. Each calls `granted::session()`.

27. `host: wire spawn at check_in` — After `begin_session()`, load session-writer, call `handle("init-session", ...)`. Store handle in Mother state.

28. `host: wire crash recovery` — Socket EOF → call `handle("crash-handoff", ...)` → write handoff, create `-crashed` tag, archive.

29. `child: populate parent_session at auto-start` — Read `last-session.md`, set `parent_session` field, copy raw `## Handoff` section. Deterministic file read only — synthesis is the agent's job.

30. `child: fix display_name` — OS user from environment, not interface name.

31. `session: backfill historical fake end tags` — Scan sessions, create real git tags for 16 frontmatter-only claims.

32. `child: measure binary size` — `cargo component build --release`. Target: <150KB.

### Verification
- Compiles with composed world. <150KB release.
- Handle actions write to artifact, create git tags.
- Crash handoff works. Parent session populated.
- All tests pass.

---

## Phase 6: DuckLake New Model (3 commits)

### Why
Proves DuckLake works on the composable model. Enterprise pipeline (watermarks, parquet, bronze/silver/gold) is Phase 11.

### Commits

33. `child: migrate ducklake to composed world with toy-github` — Replace `connector` with `github` in `lib.rs`, `Cargo.toml`, `plugin.toml`. Remove connector dependency.

34. `child: verify fetch-and-store via toy-github` — Basic fetch issues/PRs, store in DuckDB via `granted::lake()`. Verify data arrives. **Precondition:** `GITHUB_TOKEN` env var set with repo read access. If no credential, test against recorded fixture responses (add `tests/fixtures/github-api/` with captured JSON).

35. `child: end-to-end litmus` — anthropics/claude-code issues queryable via `duckdb <lake>/lake.ducklake`. **If live API unavailable (no credential or rate-limited):** replay from fixtures. Litmus passes if data arrives in DuckDB regardless of source (live or fixture).

### Verification
- Ducklake compiles with composed world (6 toys, not 14)
- Basic ingestion works via toy-github
- Queryable via standalone DuckDB
- All tests pass

---

## Phase 7: Mother Extraction (5 commits)

### Why
Makes "agents are guests" real. Mother as standalone daemon on Unix socket.

### Protocol Contract

Agent connection protocol is JSON lines over Unix socket (`~/.patina/mother.sock`):

```
→ {"v":1, "action":"connect", "payload":{"agent":"claude-code"}}
← {"v":1, "result":{"session_id":"...", "children":["ducklake","session-writer"]}}

→ {"v":1, "action":"context", "payload":{"question":"what changed?"}}
← {"v":1, "result":{"response":"..."}}

→ {"v":1, "action":"lake.sync", "payload":{"lake":"github-data"}}
← {"v":1, "result":{"issues":847, "prs":312}}

(socket close → Mother detects EOF → crash-handoff)
```

**Envelope rule:** Every message (request and response) carries `v` field. This is not negotiated — it's a fixed field on every line. V1 has no streaming, cancellation, or auth — single local user. If `v` is missing or unrecognized, Mother returns `{"v":1, "error":"unsupported protocol version"}`.

### Commits

36. `mother: create crate and move modules` — `mother/Cargo.toml`. `git mv src/mother/*`, `src/toys/*`, `src/child/*`, `src/plugin/internal/{knowledge_child,host_support}.rs`. Pure move — tests break (fixed next commit).

37. `mother: fix import paths` — Update all `use crate::` paths. Main binary depends on `mother` crate.

38. `mother: add daemon listener` — `mother::daemon::listen()` on Unix socket. JSON lines protocol per contract above.

39. `mother: implement agent connection lifecycle` — Connect → register agent, spawn session-writer, return session ID. Disconnect → crash-handoff if unclean, archive session.

40. `mother: implement daemon startup` — `patina mother start/stop/status`. PID file. On-demand auto-start.

### Verification
- `cargo build -p mother` succeeds
- Daemon starts, socket created
- Agent connects, gets session
- Agent message routed to child
- Agent disconnect triggers crash-handoff
- All tests pass

---

## Phase 8: CLI Thin Client (7 commits)

### Why
CLI becomes one agent among many. No embedded Mother.

### Commits

41. `cli: add daemon client module` — Connects to socket, auto-starts daemon if needed.

42. `cli: migrate patina context` → daemon call
43. `cli: migrate patina measure` → daemon call
44. `cli: migrate patina spec` → daemon call
45. `cli: migrate patina lake` → daemon call
46. `cli: migrate remaining commands` → daemon path
47. `cli: remove embedded Mother code` — Binary drops wasmtime/duckdb deps. Size decreases.

### Verification
- All commands work via daemon round-trip
- Main binary size decreases
- Daemon auto-starts if not running
- All tests pass

---

## Phase 9: MCP Retirement + Interface Decoupling (6 commits)

### Why
Remove the bridge. Agents bring themselves. ~6,700 LOC deleted.

### Phase Gate: Pre-Delete + Post-Delete Checks

**PRE-DELETE checks (run before commit 48, all must pass):**
```bash
# Daemon path works — everything routes through Mother, nothing depends on old paths
patina mother status                              # running, children loaded
patina mother status | grep ducklake              # loaded, healthy
patina mother status | grep session-writer        # loaded, healthy
patina context "what changed today?"              # response via daemon (not embedded Mother)
patina measure                                    # output via daemon
patina spec list                                  # patina-pre-v1 listed via daemon
cargo test                                        # all pass
```

If any pre-delete check fails, the daemon path has a gap. Fix it BEFORE deleting the old path.

**POST-DELETE checks (run after commit 53, all must pass):**
```bash
# Old code is gone and nothing references it
grep -r "use crate::mcp" src/                    # zero matches
grep -r "crate::interface::runtime" src/          # zero matches
grep -r "patina.pipe" Cargo.toml                  # zero matches
cargo build                                       # compiles without deleted code
cargo test                                        # all pass
```

### Commits

48. `retire: remove MCP server` — Delete `src/mcp/` (2,228 LOC). **Pre-condition: smoke suite passes.**
49. `retire: remove interface runtime launchers` — Delete `src/interface/runtime/` (~3,500 LOC).
50. `retire: remove tmux infrastructure` — Delete tmux lane management.
51. `retire: remove patina-pipe, patina-pipe-types, and legacy manifest parsing` — Delete pipe crates. Remove old `[capabilities]`/`[toys]` parsing code and tests from `PluginManifest` — only `[needs]` schema remains.
52. `retire: remove native github-connector` — Absorbed into toy-github.
53. `retire: update AGENTS.md and CLAUDE.md` — Describe daemon connection, toys, SDK.

### Verification
- `cargo build` succeeds without deleted code
- No references to MCP in codebase
- All tests pass

---

## Phase 10: Child Relationships + Polish (11 commits)

### Why
Completes the composable vision. Template + README deliver the external developer onramp.

### Commits

54. `manifest: add [relationships] to plugin.toml` — `emits`/`listens` declarations.
55. `mother: build event routing table` — Read declarations at child load, build routing map.
56. `wit: define patina:host/peer@0.1.0` — `emit-event`, `on-event`. Add `link_peer()`.
57. `sdk: add toy-peer` — `PeerBackend` in `patina-sdk-core`. Feature-gated.
58. `child: ducklake emits data-ingested` — After ingestion, `granted::peer().emit_event(...)`.
59. `child: session-writer listens for data-ingested` — Writes activity entry to session artifact.
60. `sdk: create cargo-generate template` — `children/template/`: `cargo-generate.toml`, templated `Cargo.toml`/`src/lib.rs`/`plugin.toml`.
61. `sdk: write README` — What Patina is, what children/toys are, 3 tiers, scaffold/build/install guide.
62. `polish: template end-to-end test` — Generate, build, install, Mother loads, `handle("ping")` responds. Under 5 minutes.
63. `polish: final binary size audit` — Session-writer <150KB, DuckLake <2MB, template <50KB.
64. `polish: update README with relationship docs` — Cross-child examples.

### Verification
- Event routing works (ducklake → session-writer)
- Template builds and installs in <5 minutes
- Binary sizes meet targets
- All tests pass

---

## Phase 11: DuckLake Enterprise Pipeline (follow-on, 12 commits)

### Why
Enterprise data engineering on top of the proven composable model. This is polish, not architecture.

### Commits

65. `child: implement endpoint planner` — 8 entity types, feature-flagged commits.
66. `child: implement two-phase ingestion` — List pagination → bounded fanout, adaptive backoff.
67. `child: implement watermark cursor system` — Stable `(updated_at, provider_id)` tuples, monotonic progression.
68. `child: implement idempotent upserts` — Entity identity keys, no silent duplicates.
69. `child: implement bronze parquet partitions` — Encrypted, partitioned by `org/repo/entity/date/`.
70. `child: implement silver normalized views` — Stable columns, typed fields, soft-delete.
71. `child: implement gold analytics views` — `SELECT * FROM issues`, excludes tombstoned.
72. `child: implement reconciliation` — Bounded tolerance (max 2% or 25 records).
73. `child: implement late-arrival handling` — 24h replay window.
74. `child: implement dead-letter flow` — Failed entities to `_dead_letter` table.
75. `child: implement operational telemetry` — Per-run and per-endpoint metrics via toy-measure.
76. `child: enterprise litmus` — Full validation against anthropics/claude-code.

### Verification
- Watermark progression monotonic, no duplicates on replay
- Parquet partitions written correctly
- Reconciliation detects count mismatches
- Telemetry emitted for all dimensions
- All tests pass
