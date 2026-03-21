# Design: Patina Pre-v1 — Full Architecture Conversion

> The SPEC says what and why. This document says how, exactly. Each phase has a commit plan with file paths, changes, and verification. A build agent should be able to execute these commits mechanically. Per [[spec-driven-design]]: code executes what specs decide.

> **Scalpel, not shotgun.** One logical change per commit. Read before write. Move first, adapt second, add third. `cargo test` after every commit — with one documented exception: move+fix commit pairs (e.g., `git mv` then fix imports) where the first commit may break compile. The pair must be consecutive and the second commit must restore green. No drive-by refactors.

## Why This Design

Patina's architecture needs to match its beliefs. The beliefs say: Mother is infrastructure, agents are guests, children are composable, toys are WIT components. The code says: everything is one binary, MCP is the interface, worlds are monolithic, toys are permission flags. This design closes the gap in 10 phases, each building on the last, each independently verifiable.

The ordering is deliberate: SDK first (because it's the external developer map and unblocks everything), plumbing second (WIT worlds and linker), new capabilities third (github and session toys), proof fourth (session-writer and ducklake), architecture fifth (Mother extraction, CLI thin client, MCP retirement), polish last (relationships).

---

## Phase 1: SDK Restructure — The Map

### Why
The SDK is what external developers see first. Today it's one flat crate with all toys always compiled. You can't build a child without pulling in lake, connector, graph, belief, and 10 other things you don't need. The tiered SDK makes the onramp obvious: start with `core`, add `data` or `agent` as needed.

### Approach
Extract, don't rewrite. The existing `patina-sdk` code is well-structured — `toys.rs` has clean trait-based backends, `knowledge_child.rs` has the plugin trait. We split these into crates by concern and add feature gates.

### Commits

1. `sdk: scaffold patina-sdk-core crate` — Create `sdk/patina-sdk-core/Cargo.toml` and `src/lib.rs`. Move `KnowledgeChildPlugin` trait, `WasmCell`, `register_knowledge_child!` macro, and the `LogBackend`/`StateBackend` traits + ZST wrappers from `sdk/patina-sdk/src/`. These are the minimum to build any child. Dependencies: `wit-bindgen` only. No `serde`/`serde_json` yet.

2. `sdk: scaffold patina-sdk-data crate` — Create `sdk/patina-sdk-data/Cargo.toml` and `src/lib.rs`. Move `LakeBackend`, `CheckpointBackend`, `MeasureBackend`, `ConnectorBackend`, `IngressBackend` traits + ZST wrappers. Feature-gated: `toy-lake`, `toy-checkpoint`, `toy-measure`, `toy-github` (empty stub for now), `toy-connector` (legacy, to be replaced). Dependencies: `patina-sdk-core`, `serde`, `serde_json`.

3. `sdk: scaffold patina-sdk-agent crate` — Create `sdk/patina-sdk-agent/Cargo.toml` and `src/lib.rs`. Move `QueryBackend`, `EmitBackend` traits + ZST wrappers. Feature-gated: `toy-query`, `toy-session` (empty stub), `toy-emit`. Dependencies: `patina-sdk-core`.

4. `sdk: refactor patina-sdk as re-export umbrella` — Rewrite `sdk/patina-sdk/src/lib.rs` to re-export all three tiers. Existing children (`ducklake`, `belief-verifier`) continue to depend on `patina-sdk` — no change to their Cargo.toml yet. All existing tests pass.

5. `sdk: feature-gate granted module` — In each tier crate, wrap toy factory functions in `#[cfg(feature = "toy-*")]` gates. The `granted::log()` function only exists when `toy-log` is enabled. The `granted::lake()` function only exists when `toy-lake` is enabled. Default features in `patina-sdk` enable everything (backward compat).

6. `sdk: add per-toy features to child Cargo.tomls` — Update `children/ducklake/Cargo.toml` to use `patina-sdk = { features = ["knowledge-child", "toy-log", "toy-state", "toy-checkpoint", "toy-lake", "toy-connector", "toy-measure"] }`. Update `children/belief-verifier/Cargo.toml` similarly. Verify both compile and all tests pass.

7. `sdk: create cargo-generate template` — Add `children/template/` with `cargo-generate.toml`, templated `Cargo.toml`, `src/lib.rs` (minimal handle-only child), `plugin.toml`, and `README.md`. Template asks for child name and toy selection.

8. `sdk: write SDK README` — Write `sdk/README.md` covering: what Patina is (infrastructure for agents), what a child is (composable WASM worker), what a toy is (WIT capability), the three tiers, how to scaffold, build, install, and test a child. 5-minute onramp.

### Direct Code Targets
- `sdk/patina-sdk/src/lib.rs` — becomes re-export umbrella
- `sdk/patina-sdk/src/toys.rs` — traits split across tier crates
- `sdk/patina-sdk/src/knowledge_child.rs` — `granted` module feature-gated
- `sdk/patina-sdk-core/src/lib.rs` — NEW: core trait + log + state
- `sdk/patina-sdk-data/src/lib.rs` — NEW: data toys
- `sdk/patina-sdk-agent/src/lib.rs` — NEW: agent toys
- `children/ducklake/Cargo.toml` — explicit feature selection
- `children/belief-verifier/Cargo.toml` — explicit feature selection
- `children/template/` — NEW: cargo-generate template
- `sdk/README.md` — NEW: onramp guide
- `Cargo.toml` (workspace) — add new members

### Verification
- `cargo build -p patina-sdk-core` succeeds with zero dependencies beyond wit-bindgen
- `cargo build -p patina-sdk-data --features toy-lake` succeeds
- `cargo build -p patina-sdk-agent --features toy-query` succeeds
- `cargo build -p patina-sdk` (umbrella) succeeds with all features
- `cargo component build -p patina-plugin-ducklake` succeeds (existing child, explicit features)
- `cargo component build -p patina-plugin-belief-verifier` succeeds
- All existing tests pass (`cargo test`)

---

## Phase 2: Per-Child WIT Worlds

### Why
The monolithic `knowledge-child` WIT world imports all 14 host interfaces. A session child needing 3 toys gets bindings for 14. The binary is bloated and the compile-time contract is meaningless. Per-child worlds make the WIT definition match the child's actual needs — the world IS the sandbox.

### Approach
Split the monolithic `host.wit` into individual toy WIT files. Create per-child world files that import only needed toys. Update the build system to compile each child against its own world.

### WIT Governance: Source of Truth Rule

**`wit/toys/*.wit` files are the canonical source.** They are hand-edited. The old monolithic `wit/deps/patina-host/host.wit` is retired after extraction — it is NOT kept as a parallel re-export. One source of truth, not two that drift.

**`wit/worlds/*.wit` files are hand-edited per-child.** Each world file is owned by the child it serves. Adding a toy to a child means editing its world file and its `plugin.toml` manifest — both must agree.

**Rule: if you edit a toy interface, you edit `wit/toys/<toy>.wit`. There is no other copy to update.** World files only import — they never redefine toy interfaces. SDK tier crates symlink or copy toy WIT files from `wit/toys/` at build time (build script, not manual copy).

### Commits

9. `wit: split host.wit into individual toy interfaces` — Create `wit/toys/` directory. Extract each `interface` block from `wit/deps/patina-host/host.wit` into its own file: `log.wit`, `state.wit`, `lake.wit`, `checkpoint.wit`, `measure.wit`, `query.wit`, `http.wit`, `emit.wit`, `ingress.wit`, `connector.wit`, `events.wit`, `task.wit`, `graph.wit`, `belief.wit`, `types.wit`. Each file is a standalone WIT package under `patina:host`. The original monolithic `host.wit` is deleted — `wit/toys/` is now the single source of truth (see WIT Governance rule above).

10. `wit: create per-child world files` — Create `wit/worlds/` directory. Write `ducklake.wit` importing only `log`, `state`, `checkpoint`, `lake`, `connector`, `measure`, `types`. Write `belief-verifier.wit` importing only `log`, `state`, `checkpoint`, `events`, `belief`, `types`. Each world exports the same `handle`/`tick`/`drain`/`health` functions as the monolithic world.

11. `wit: update ducklake build to use ducklake.wit world` — Change `children/ducklake/Cargo.toml` and its WIT config to compile against `wit/worlds/ducklake.wit` instead of the monolithic `knowledge-child.wit`. Update `wit-bindgen` configuration. Verify compilation succeeds.

12. `wit: update belief-verifier build to use belief-verifier.wit world` — Same as above for belief-verifier against `wit/worlds/belief-verifier.wit`.

13. `wit: copy per-child wit into SDK tier crates` — Each SDK tier crate gets the toy WIT files it needs in its `wit/` directory. `patina-sdk-core` gets `log.wit`, `state.wit`, `types.wit`. `patina-sdk-data` gets `lake.wit`, `checkpoint.wit`, `measure.wit`, `connector.wit`. Build system composes the world from enabled features.

14. `wit: measure binary size reduction` — Build ducklake and belief-verifier in release mode. Compare binary sizes against baseline (pre-Phase-2). Document the delta. Expected: measurable reduction from eliminated unused interface stubs.

### Direct Code Targets
- `wit/deps/patina-host/host.wit` — source for extraction, then DELETED (wit/toys/ becomes canonical)
- `wit/toys/*.wit` — NEW: 15 individual toy interface files
- `wit/worlds/ducklake.wit` — NEW: composed world for ducklake
- `wit/worlds/belief-verifier.wit` — NEW: composed world for belief-verifier
- `children/ducklake/Cargo.toml` — world reference change
- `children/belief-verifier/Cargo.toml` — world reference change
- `sdk/patina-sdk-core/wit/` — toy WIT files for core tier
- `sdk/patina-sdk-data/wit/` — toy WIT files for data tier

### Verification
- `cargo component build -p patina-plugin-ducklake --release` succeeds with new world
- `cargo component build -p patina-plugin-belief-verifier --release` succeeds with new world
- Release binary sizes for both children are smaller than Phase 1 baseline
- All existing tests pass

---

## Phase 3: Per-Child Linker

### Why
Today Mother uses one shared `Linker<HostState>` that links all 14 host interfaces for every child. The child manifest says which toys it needs, but the linker ignores this — every child gets everything linked. This means a child that doesn't declare `lake` can still call lake functions (they'll fail at runtime grant checks, but they shouldn't even be linkable). Per-child linking makes the sandbox compile-time real.

### Manifest Schema Migration

Current manifests use `[capabilities]` (boolean flags, scoped sub-tables) and `[toys]` (flat key-value) as two separate sections. The new model consolidates into a single `[needs]` section with a `toys` list, since the WIT world is now the capability boundary.

**Current schema** (ducklake example):
```toml
[capabilities]
host_log = true
host_measure = true
[capabilities.state]
enabled = true
[capabilities.checkpoint]
streams = ["ducklake.sync"]
[toys]
lake = ["default"]
connector = true
```

**Target schema:**
```toml
[needs]
toys = ["log", "state", "checkpoint", "lake", "github", "measure"]

[needs.scopes]
checkpoint.streams = ["ducklake.sync"]
lake.names = ["default"]
```

`[needs].toys` is the flat list the linker reads. `[needs.scopes]` carries per-toy runtime parameters (which streams, which lake names, etc.) — these feed into `GrantedToys` for defense-in-depth scoping within a granted toy.

**Migration order:** Commit 16 adds `[needs]` parsing alongside existing `[capabilities]`/`[toys]` (reads both, prefers `[needs]` if present). Commit 6 updates child `plugin.toml` files to the new schema. Old schema support removed in Phase 9 cleanup.

### Approach
Refactor `KnowledgeChildEngine` to build a linker per-child based on the manifest's `[needs].toys` list. Split the monolithic `add_to_linker` into per-interface linking functions. Keep `GrantedToys` runtime checks as defense-in-depth.

### Commits

15. `host: split add_to_linker into per-interface functions` — In `src/plugin/internal/knowledge_child.rs`, extract each `impl patina::host::*::Host for HostState` block into a function that can be called independently: `link_log()`, `link_state()`, `link_lake()`, etc. The existing shared linker calls all of them (no behavior change yet).

16. `host: read manifest toys in KnowledgeChildEngine` — Extend `PluginManifest` (from `plugin.toml`) to parse `[needs].toys` as a `Vec<String>`. The `instantiate_child` path reads this list but doesn't use it yet.

17. `host: build per-child linker from manifest` — In `instantiate_child`, build a fresh `Linker<HostState>` per-child. Iterate `manifest.toys()` and call only the matching `link_*()` function for each declared toy. If a child's WASM binary imports an interface not in its manifest, wasmtime will fail at instantiation with a clear "missing import" error.

18. `host: verify sandbox enforcement` — Add test: compile a minimal test child that imports `lake`, but give it a manifest that doesn't declare `lake`. Verify instantiation fails with "missing import" error. Add test: same child with `lake` in manifest — verify instantiation succeeds.

### Direct Code Targets
- `src/plugin/internal/knowledge_child.rs` — split `add_to_linker`, per-child linker build
- `src/plugin/internal/tests.rs` — sandbox enforcement tests
- Plugin manifest parsing (wherever `plugin.toml` is read)

### Verification
- Existing children still load and function (manifest includes their actual toys)
- Test child without `lake` in manifest fails to instantiate
- Test child with `lake` in manifest succeeds
- All existing tests pass

---

## Phase 4: New Toy Interfaces

### Why
Two capabilities don't exist as toys yet: GitHub data access (currently a separate native child process) and session artifact management (currently done by shell scripts and LLM skills). Both need to become WIT interfaces that Mother implements host-side.

### Approach
Define WIT interfaces. Implement host-side. Absorb existing code — github from the native connector, session from `src/session/`. Add to SDK tiers.

### Commits

19. `wit: define patina:host/github@0.1.0 interface` — Create `wit/toys/github.wit`. Typed records for issue, pull-request, comment, review, event. Paginated list functions for all 8 entity types. `list-params` record with `since`, `state`, `page`, `per-page`. `page<T>` record with `items`, `has-next`, `next-page`, `rate-remaining`.

20. `host: implement github toy` — Create `src/toys/github.rs`. Implement the WIT interface against GitHub's REST API. Credential injection from `HostState` grants — child never sees tokens. Rate-limit tracking per endpoint. Pagination follows `Link` headers. Absorb HTTP client logic from the native github-connector where applicable. Add `link_github()` function for the per-child linker.

21. `host: add github toy tests` — Integration tests with fixture data (recorded API responses). Verify pagination, rate-limit backoff, credential injection, all 8 entity types.

22. `wit: define patina:host/session@0.1.0 interface` — Create `wit/toys/session.wit`. Functions: `write-artifact(content: string)`, `create-tag(name: string)`, `set-status(status: string)`, `write-handoff(handoff: string)`, `get-previous-session() -> option<string>`, `get-session-id() -> string`.

23. `host: implement session toy` — Create `src/toys/session.rs`. Implement against `src/session/internal/live.rs` — calls `begin_session`, `archive_session`, git tag creation. Scoped to `layer/sessions/` only. Add `link_session()` function.

24. `host: add session toy tests` — Tests: write artifact, create real git tag, set status, write crash handoff. Verify git tags exist in repo.

25. `sdk: add toy-github and toy-session to SDK tiers` — Add `GithubBackend` trait to `patina-sdk-data` with `toy-github` feature. Add `SessionBackend` trait to `patina-sdk-agent` with `toy-session` feature. Wire `granted::github()` and `granted::session()` factory functions.

### Direct Code Targets
- `wit/toys/github.wit` — NEW
- `wit/toys/session.wit` — NEW
- `src/toys/github.rs` — NEW (absorb from native connector)
- `src/toys/session.rs` — NEW (absorb from `src/session/`)
- `src/plugin/internal/knowledge_child.rs` — add `link_github()`, `link_session()`
- `sdk/patina-sdk-data/src/lib.rs` — `GithubBackend` trait
- `sdk/patina-sdk-agent/src/lib.rs` — `SessionBackend` trait

### Verification
- `cargo test` for github toy with fixture data
- `cargo test` for session toy — git tags created, artifact written
- SDK compiles with new features enabled
- All existing tests pass

---

## Phase 5: Session-Writer Child

### Why
First child born on the new model. Proves three things: (1) a minimal child can be tiny (<150KB), (2) per-child worlds work end-to-end, (3) crash recovery via Mother-managed child is viable. Every future session — regardless of which agent connects — gets proper artifact lifecycle.

### Approach
Minimal child. Three toys: log, state, session. Handle-only (no tick, no drain). Mother spawns it at `check_in()`. Mother calls it on agent death for crash recovery.

### Commits

26. `wit: create session-writer world` — Create `wit/worlds/session-writer.wit` importing only `log`, `state`, `session`, `types`. Exports: `handle`, `health`, `init`, `name`. No `tick`, no `drain` (default stubs from SDK).

27. `child: scaffold session-writer` — Create `children/session-writer/Cargo.toml` depending on `patina-sdk-core` + `patina-sdk-agent = { features = ["toy-session"] }`. Create `src/lib.rs` with `SessionWriter` struct implementing `KnowledgeChildPlugin`. Actions: `"note"`, `"update"`, `"spec-link"`, `"close"`, `"crash-handoff"`.

28. `child: implement session-writer handle actions` — `"note"`: writes note to artifact via `granted::session().write_artifact()`. `"update"`: appends activity log entry. `"spec-link"`: adds spec reference. `"close"`: writes outcome, creates real end tag. `"crash-handoff"`: writes modified-files from git, structured handoff, creates `-crashed` end tag.

29. `host: wire session-writer spawn at check_in` — In `src/interface/internal/checkin.rs`, after `begin_session()`, load and instantiate the session-writer child. Pass session ID and artifact path via `handle("init-session", ...)`. Store child handle in Mother's state for the interface.

30. `host: wire crash recovery` — When Mother detects interface death (pipe EOF, heartbeat timeout, tmux lane poll), call session-writer's `handle("crash-handoff", ...)` with git diff since session start. Session-writer writes handoff to artifact, creates real `-crashed` git tag, archives session.

31. `child: populate parent_session and raw handoff at auto-start` — Session-writer's `"init-session"` handler reads `last-session.md`, populates `parent_session` field in frontmatter, and copies the raw `## Handoff` section from the previous session artifact into `## Previous Session Context`. This is a deterministic file read + copy — no synthesis or summarization. The agent fills in the human-readable summary when it engages (that's the agent's job, not the child's). Per [[session-capture]]: scripts handle mechanics, humans (and agents) handle meaning.

32. `child: fix display_name in auto-start sessions` — Session-writer sets `display_name` to the OS user (from environment), not the interface name. Fixes the ghost session fingerprint.

33. `session: backfill historical fake end tags` — Script: scan all `layer/sessions/*.md` files. For each session with `end_tag` in frontmatter but no corresponding real git tag, create the tag at the session's archive commit (find via `git log --all --oneline -- <artifact_path>` for the last commit touching that file). Log results. This is a one-time historical cleanup per [[git-tags-must-be-real-or-not-claimed]]. The 16 known fake tags from the March audit are the target.

34. `child: measure session-writer binary size` — Build `cargo component build --release`. Measure `.wasm` binary. Target: <150KB. If over, profile with `wasm-opt` and identify what's pulling in weight.

### Direct Code Targets
- `wit/worlds/session-writer.wit` — NEW
- `children/session-writer/Cargo.toml` — NEW
- `children/session-writer/src/lib.rs` — NEW
- `children/session-writer/plugin.toml` — NEW
- `src/interface/internal/checkin.rs` — spawn session-writer
- `src/mother/state.rs` — store session-writer handle per interface
- `Cargo.toml` (workspace) — add member

### Verification
- Session-writer compiles with composed world
- Release binary <150KB
- `handle("note", ...)` writes to artifact
- `handle("crash-handoff", ...)` creates real git tag
- `handle("close", ...)` creates real end tag
- Parent session populated at auto-start
- Display name is OS user, not interface name
- All existing tests pass

---

## Phase 6: DuckLake Enterprise

### Why
Proves the composable model handles real enterprise workloads. DuckLake composes 6 toys, implements production-grade ingestion with watermarks, idempotent upserts, encrypted parquet, bronze/silver/gold outputs, reconciliation, and operational telemetry.

### Approach
Migrate ducklake to its composed world (from Phase 2). Then build the enterprise pipeline in slices: scope/planner first, materialization second, quality/operations third. Data contract is locked from the superseded spec.

### Commits

35. `child: migrate ducklake to composed world with toy-github` — Replace `connector` toy usage with `github` toy in ducklake's `lib.rs`. Update `Cargo.toml` features. Update `plugin.toml` manifest. Remove connector dependency. Verify existing fetch-and-store still works against toy-github.

36. `child: implement endpoint planner` — Add `Planner` struct to ducklake that knows all 8 entity types (issues, issue-comments, issue-events, pulls, pull-comments, reviews, review-comments, pull-commits). Planner reads `plugin.toml` config for which entities are enabled. Pull-commits feature-flagged off by default.

37. `child: implement two-phase ingestion pipeline` — Phase A: list pagination for top-level entities (issues, pulls) via `granted::github().list_issues()` / `list_pulls()`. Phase B: bounded fanout for child entities (comments, events, reviews) with concurrency limit. Adaptive rate-limit backoff using `rate-remaining` from page responses.

38. `child: implement watermark cursor system` — Stable watermark tuple: `(updated_at, provider_id)` per repo/entity stream. Monotonic lexicographic progression. Resume queries are time-inclusive, local filtering drops `<=` last committed tuple. Store watermarks via `granted::state()`.

39. `child: implement idempotent upserts` — Entity identity key: `repo_id + entity_type + provider_id`. On insert, check for existing entity by key. If exists and `updated_at` is newer, update. If exists and same or older, skip. No silent duplicates.

40. `child: implement bronze parquet partitions` — Write raw API responses as encrypted parquet files partitioned by `org/repo/entity/date/`. Use DuckDB's `COPY ... TO ... (FORMAT PARQUET, ENCRYPTION_CONFIG ...)`. Store partition manifests in DuckDB metadata via `granted::lake()`.

41. `child: implement silver normalized views` — DuckDB views over bronze with normalization: stable column names, typed fields, deduplication by identity key. Soft-delete support: `is_deleted`, `deleted_at` columns populated by reconciliation.

42. `child: implement gold analytics views` — Stable query views: `SELECT * FROM issues`, `SELECT * FROM prs`, `SELECT * FROM comments`. Exclude tombstoned rows by default. These are the surfaces downstream agents/apps consume.

43. `child: implement reconciliation` — Compare entity counts from GitHub API list endpoints against bronze record counts. Bounded tolerance: `max(2% of source count, 25 records)`. Log threshold breaches. Flag missing entities for re-fetch on next run.

44. `child: implement late-arrival handling` — Replay window: 24h trailing for child entities. On each run, re-read entities with `updated_at` within replay window. Deduplicate by identity key + tuple ordering. Captures edits to comments, state changes, etc.

45. `child: implement dead-letter flow` — Entities that fail parsing or storage after 3 retries go to `_dead_letter` table with error details. Dead letters don't block cursor advancement. Reportable via `handle("status", ...)`.

46. `child: implement operational telemetry` — Emit via `granted::measure()`: per-run metrics (duration, total calls, bytes fetched, retries, entities processed), per-endpoint metrics (calls, rate-limit hits, errors), lag metric (time between newest entity and now).

47. `child: end-to-end litmus — anthropics/claude-code` — Full ingestion run against anthropics/claude-code repo. Verify bronze/silver/gold outputs. Verify reconciliation against GitHub API totals. Verify queryable via standalone `duckdb <lake>/lake.ducklake`. Measure binary size.

### Direct Code Targets
- `children/ducklake/src/lib.rs` — major rewrite (planner, pipeline, watermarks, upserts)
- `children/ducklake/src/planner.rs` — NEW: endpoint planner
- `children/ducklake/src/pipeline.rs` — NEW: two-phase ingestion
- `children/ducklake/src/watermark.rs` — NEW: cursor system
- `children/ducklake/src/materialization.rs` — NEW: bronze/silver/gold
- `children/ducklake/src/quality.rs` — NEW: reconciliation, late-arrival, dead-letter
- `children/ducklake/src/telemetry.rs` — NEW: measure emission
- `children/ducklake/Cargo.toml` — updated features and dependencies
- `children/ducklake/plugin.toml` — updated manifest

### Verification
- Ducklake compiles with composed world (6 toys, not 14)
- Two-phase ingestion works with fixture data
- Watermark progression is monotonic, no duplicates on replay
- Bronze parquet files written with correct partitioning
- Silver views normalize correctly
- Gold views exclude tombstoned rows
- Reconciliation detects count mismatches within tolerance
- Late-arrival replay captures recent edits
- Dead letters captured without blocking cursors
- Telemetry emitted for all metric dimensions
- End-to-end litmus against anthropics/claude-code passes
- All existing tests pass

---

## Phase 7: Mother Extraction

### Why
Mother is buried in the CLI binary. She can't accept connections from agents independently. Extracting her into a standalone daemon is what makes "agents are guests" real — any agent can connect to Mother without going through the CLI.

### Approach
Pure move first, then adapt, then add. Commit 1: `git mv` modules. Commit 2: fix imports. Commit 3: add daemon listener. This keeps diffs reviewable and preserves git blame.

### Commits

48. `mother: create crate and move modules` — Create `mother/Cargo.toml`. `git mv src/mother/* mother/src/`. `git mv src/toys/* mother/src/host/`. `git mv src/child/* mother/src/child/`. `git mv src/plugin/internal/knowledge_child.rs mother/src/wasm/knowledge_child.rs`. `git mv src/plugin/internal/host_support.rs mother/src/wasm/host_support.rs`. Pure move — no logic changes. Tests will break (import paths). That's expected and fixed next commit.

49. `mother: fix import paths` — Update all `use crate::mother::`, `use crate::toys::`, `use crate::child::`, `use crate::plugin::` paths throughout the codebase to point to the new `mother` crate. The main binary (`src/`) depends on `mother` crate. Children and SDK don't change (they talk through WIT, not Rust imports).

50. `mother: add daemon listener` — Implement `mother::daemon::listen()`. Accepts connections on Unix socket (`~/.patina/mother.sock`). Each connection is an agent session. Connection protocol: JSON lines over Unix socket. Agent sends `{"action": "...", "payload": ...}`, Mother routes to appropriate child, returns `{"result": ...}` or `{"error": ...}`. Same `handle(action, payload) → response` pattern as children.

51. `mother: implement agent connection lifecycle` — On agent connect: register agent, spawn session-writer child (Phase 5), return session ID. On agent disconnect: call session-writer crash-handoff if unclean, archive session. On agent message: route to appropriate child or Mother-level handler.

52. `mother: implement daemon startup` — `patina mother start` launches daemon, writes PID to `~/.patina/mother.pid`, listens on socket. `patina mother stop` sends shutdown. `patina mother status` reports running children, connected agents, active sessions. Daemon starts on-demand if not running when CLI tries to connect.

### Direct Code Targets
- `mother/Cargo.toml` — NEW
- `mother/src/lib.rs` — NEW: crate root
- `mother/src/daemon.rs` — NEW: listener
- `mother/src/broker.rs` — moved from `src/mother/broker/`
- `mother/src/state.rs` — moved from `src/mother/state.rs`
- `mother/src/host/*.rs` — moved from `src/toys/`
- `mother/src/wasm/*.rs` — moved from `src/plugin/internal/`
- `mother/src/child/` — moved from `src/child/`
- `src/main.rs` — depends on `mother` crate
- `Cargo.toml` (workspace) — add `mother` member

### Verification
- `cargo build -p mother` succeeds
- `cargo build` (main binary) succeeds with `mother` dependency
- `patina mother start` launches daemon, socket created
- Agent connects via socket, session starts
- Agent sends message, receives response routed through child
- Agent disconnects, session archived
- All existing tests pass (via main binary path still working)

---

## Phase 8: CLI Thin Client

### Why
The CLI embeds Mother today — 2,132 LOC in `main.rs` plus all the command implementations. In the new model, CLI is a thin client that connects to Mother's daemon socket and forwards requests. This completes the separation: Mother is infrastructure, CLI is one of many agents.

### Approach
Incremental migration. Each command gets a "daemon path" that connects to Mother's socket. If daemon is running, use it. If not, start it on-demand. Eventually all commands use the daemon path and the embedded path is removed.

### Commits

53. `cli: add daemon client module` — Create `src/commands/client.rs` (or `cli/src/client.rs` if separate crate). Connects to `~/.patina/mother.sock`. Sends JSON request, receives JSON response. Handles daemon-not-running (auto-start).

54. `cli: migrate patina context to daemon path` — `patina context` sends `{"action": "context", "payload": {"question": "..."}}` to Mother. Mother routes to query child (or handles directly for now). Returns response. CLI displays it.

55. `cli: migrate patina measure to daemon path` — Same pattern for measure.

56. `cli: migrate patina spec to daemon path` — Spec operations route through Mother.

57. `cli: migrate patina lake to daemon path` — Lake operations route to ducklake child via Mother.

58. `cli: migrate remaining commands` — All commands that need Mother infrastructure use the daemon path. Pure CLI commands (help, version) stay local.

59. `cli: remove embedded Mother code from main binary` — The main binary no longer includes Mother's broker, state, WASM engine, etc. It only includes the daemon client. Binary size should drop significantly.

### Direct Code Targets
- `src/commands/client.rs` — NEW: daemon client
- `src/commands/*.rs` — each command gets daemon path
- `src/main.rs` — slimmed down, just CLI parsing + daemon client
- `Cargo.toml` — remove heavy dependencies (wasmtime, duckdb) from main binary

### Verification
- `patina context "what changed?"` returns response via daemon round-trip
- `patina measure` works via daemon
- `patina spec list` works via daemon
- Main binary size decreases (no wasmtime, no duckdb)
- All commands work with daemon running
- Daemon auto-starts if not running
- All tests pass

---

## Phase 9: MCP Retirement + Interface Decoupling

### Why
MCP was a bridge that became permanent. The agent connection protocol (Phase 7) replaces it. Interface runtimes (Claude/OpenCode/Gemini launch code) are unnecessary when agents bring themselves. This phase removes ~6,700 LOC.

### Approach
Delete in order: MCP server first (replaced by daemon protocol), then interface runtimes (agents connect directly), then pipe infrastructure (native child protocol retired). Each deletion is a separate commit for reviewability.

**Phase gate: compatibility smoke suite MUST pass before ANY deletion.** This phase removes ~6,700 LOC in quick succession. Before commit 60, run the full smoke suite below. If any check fails, the daemon path has a gap — fix it BEFORE deleting the old path. Never delete the old path while something still needs it — that's how [[bridges-become-permanent]] happens in reverse (premature retirement).

**Concrete smoke suite (run all, all must pass):**
```bash
# 1. Mother daemon is running and healthy
patina mother status                    # expect: running, children loaded

# 2. Children load and respond via daemon
patina mother status | grep ducklake    # expect: loaded, healthy
patina mother status | grep session-writer  # expect: loaded, healthy

# 3. Core CLI commands work via daemon round-trip (not embedded Mother)
patina context "what changed today?"    # expect: response, not "daemon not found"
patina measure                          # expect: output
patina spec list                        # expect: patina-pre-v1 listed
patina lake list                        # expect: lake listing or empty

# 4. Session lifecycle works via daemon
# (start a test session, write a note, close it — verify artifact and git tags)
patina mother status | grep "active sessions"  # expect: current session visible

# 5. DuckLake ingests via daemon-hosted toy-github (if configured)
# (skip if no github credential configured — but verify child responds)
# patina lake sync github-data          # expect: ingestion run or auth error (not "command not found")

# 6. No code path still imports from src/mcp/
grep -r "use crate::mcp" src/          # expect: zero matches
grep -r "mcp::" src/commands/          # expect: zero matches

# 7. No code path still imports from src/interface/runtime/
grep -r "use crate::interface::runtime" src/  # expect: zero matches

# 8. Full test suite
cargo test                              # expect: all pass
```

**Gate rule:** ALL 8 checks must pass. If check 6 or 7 finds lingering imports, those code paths depend on MCP/interface runtimes and must be migrated to the daemon path first.

### Commits

60. `retire: remove MCP server` — Delete `src/mcp/` (2,228 LOC). Remove MCP-related dependencies from `Cargo.toml`. Remove MCP server startup from daemon/CLI. Update AGENTS.md to describe agent connection protocol instead of MCP tools. **Pre-condition: smoke suite confirms no command depends on MCP path.**

61. `retire: remove interface runtime launchers` — Delete `src/interface/runtime/claude/`, `src/interface/runtime/gemini/`, `src/interface/runtime/opencode/` and their parent module. (~3,500 LOC). Keep `src/interface/internal/checkin.rs` (session check-in is now done by the daemon connection handler, but the logic may be useful). Update CLAUDE.md, OPENCODE.md.

62. `retire: remove tmux infrastructure` — Delete `src/interface/internal/tmux.rs` and related tmux lane management. Agents manage their own terminal sessions.

63. `retire: remove patina-pipe and patina-pipe-types crates` — Delete `crates/patina-pipe/` and `crates/patina-pipe-types/`. Remove from workspace. These were the native child communication protocol, replaced by WASM WIT interfaces.

64. `retire: remove native github-connector` — Delete any remaining native github-connector child code. GitHub access is now toy-github (Phase 4).

65. `retire: update AGENTS.md and CLAUDE.md` — Rewrite agent instructions. Describe: how to connect to Mother, what toys are available, how to use the SDK to build children. Remove all MCP tool documentation.

### Direct Code Targets
- `src/mcp/` — DELETE (2,228 LOC)
- `src/interface/runtime/` — DELETE (~3,500 LOC)
- `src/interface/internal/tmux.rs` — DELETE
- `crates/patina-pipe/` — DELETE
- `crates/patina-pipe-types/` — DELETE
- `AGENTS.md` — REWRITE
- `CLAUDE.md` — REWRITE

### Verification
- `cargo build` succeeds without MCP, interface, pipe code
- All remaining tests pass
- `patina mother start` still works
- Agents connect via Unix socket protocol
- No references to MCP remain in codebase (grep confirms)

---

## Phase 10: Child Relationships + Polish

### Why
Completes the composable vision. Children can emit events and listen for events from other children, with Mother mediating. DuckLake ingests data → emits `data-ingested` → session-writer captures as activity. This is the last piece of the "Mother orchestrates, children compose" model.

### Approach
Extend the manifest format, build the routing table in Mother, add a peer communication WIT interface.

### Commits

66. `manifest: add relationships to plugin.toml` — Extend `plugin.toml` schema with `[relationships]` section: `emits = ["event-name", ...]` and `listens = ["event-name", ...]`. Parse in manifest loader.

67. `mother: build event routing table at child load` — When Mother loads children, read their `[relationships]` declarations. Build a routing table: event name → list of listening children. Store in Mother's state.

68. `wit: define patina:host/peer@0.1.0 interface` — Create `wit/toys/peer.wit`. Functions: `emit-event(name: string, payload: string)` (child emits, Mother routes), `on-event(name: string, payload: string)` (Mother delivers to listener). Add `link_peer()` to linker.

69. `sdk: add toy-peer to SDK` — Add `PeerBackend` trait to `patina-sdk-core` (peer communication is fundamental, not tier-specific). Feature-gated as `toy-peer`.

70. `child: ducklake emits data-ingested events` — After successful ingestion run, ducklake calls `granted::peer().emit_event("data-ingested", summary_json)`. Add `peer` to ducklake's world and manifest.

71. `child: session-writer listens for data-ingested` — Session-writer's manifest declares `listens = ["data-ingested"]`. Mother routes ducklake's event to session-writer via `handle("on-event", ...)`. Session-writer writes activity log entry to session artifact.

72. `polish: final binary size audit` — Build all children in release mode. Document sizes. Session-writer target: <150KB. DuckLake target: <2MB. Template child target: <50KB.

73. `polish: external developer template end-to-end test` — Run `cargo generate` with template. Build child. Install to `~/.patina/plugins/children/`. Start Mother. Verify child loads and responds to `handle("ping", "")`. Total time: under 5 minutes.

74. `polish: update SDK README with relationship documentation` — Document how to declare relationships, emit events, listen for events. Add example of cross-child communication.

### Direct Code Targets
- `plugin.toml` schema — extended with `[relationships]`
- `mother/src/broker.rs` — event routing table
- `wit/toys/peer.wit` — NEW
- `sdk/patina-sdk-core/src/lib.rs` — `PeerBackend` trait
- `children/ducklake/src/lib.rs` — emit events
- `children/ducklake/plugin.toml` — add emits
- `children/session-writer/src/lib.rs` — listen for events
- `children/session-writer/plugin.toml` — add listens
- `children/template/` — updated with relationship example
- `sdk/README.md` — updated with relationship docs

### Verification
- DuckLake emits `data-ingested` after run
- Mother routes event to session-writer
- Session-writer writes activity entry to artifact
- Binary sizes meet targets
- Template child builds and installs in <5 minutes
- All tests pass

---

## Open Questions

1. **Daemon auto-start mechanism.** Should Mother daemon start via launchd/systemd, or on-demand when CLI first connects? On-demand is simpler but adds latency to first command.

2. **Agent connection protocol details.** JSON lines over Unix socket is the baseline. Should we also support TCP for remote agents? HTTP/SSE for browser-based agents? Start with Unix socket only, extend later.

3. **WIT world composition tooling.** Can `cargo-component` build against a custom world file, or do we need WAC (WebAssembly Compositions) tooling? Needs prototyping in Phase 2.

4. **DuckDB in WASM.** DuckLake currently embeds DuckDB. Does the `toy-lake` host-side implementation mean DuckDB moves to Mother? If so, the WASM child doesn't need DuckDB — it calls `granted::lake()` and Mother handles DuckDB. This could dramatically reduce ducklake binary size.

5. **Session-writer crash detection.** How does Mother detect agent death? Options: Unix socket EOF (immediate), heartbeat timeout (delayed but reliable), tmux lane poll (legacy). Socket EOF is the natural choice with the new daemon model.

6. **SDK crate publishing.** When do we publish the SDK crates to crates.io? After Phase 1? After Phase 6? Needs a versioning strategy.
