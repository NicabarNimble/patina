---
type: refactor
id: patina-pre-v1
status: active
created: 2026-03-21
sessions:
  origin: 20260320-212325-011658000
supersedes:
- composable-toy-sdk
- ducklake-enterprise
- interface-session-model
- session-handoff-enrichment
- mother-maturation
- measure-process-owned
- grammar-markdown
- runtime-hardening
- scrape-simplification
- spec-knowledge-evolution
- spec-prompt-handoff
- patina-v1
beliefs:
- agents-are-guests-mother-is-infrastructure
- children-have-agency-toys-are-capabilities
- initialize-is-capability-grant
- connector-toy-is-indivisible-authority
- mother-is-the-daemon
- durability-lives-outside-interface-process
- universal-artifact-interface-specific-enrichment
- git-tags-must-be-real-or-not-claimed
- beliefs-live-at-two-levels
exit_criteria:
- id: EC1
  text: Tiered SDK ships — patina-sdk-core, patina-sdk-data, patina-sdk-agent each build independently with feature-gated toys
  checked: false
- id: EC2
  text: Per-child WIT worlds — each child declares its own world importing only needed toy interfaces, no monolithic knowledge-child world
  checked: false
- id: EC3
  text: Per-child linker — Mother builds a linker per child from its manifest, linking only declared toy interfaces
  checked: false
- id: EC4
  text: toy-github WIT interface — Mother implements github issues/PRs/comments/reviews/events with pagination, rate-limit backoff, credential injection
  checked: true
- id: EC5
  text: toy-session WIT interface — Mother implements session artifact writes, git tag creation, crash recovery handoff
  checked: true
- id: EC6
  text: session-writer child — minimal WASM child (log + state + session toys), spawned at agent connection, handles artifact lifecycle and crash recovery
  checked: false
- id: EC7
  text: DuckLake on new model — composed world with toy-github replacing connector, basic fetch-and-store works, queryable via standalone DuckDB CLI
  checked: false
- id: EC8
  text: Mother extracted — standalone daemon crate, accepts agent connections, manages children and toys, separate from CLI binary
  checked: false
- id: EC9
  text: Agent connection protocol — JSON lines over Unix socket, any agent can connect, no MCP required
  checked: true
- id: EC10
  text: CLI is thin client — patina commands delegate to Mother daemon, no embedded Mother logic
  checked: false
- id: EC11
  text: MCP retired — MCP server removed from main binary, replaced by agent connection protocol
  checked: true
- id: EC12
  text: Interface runtimes decoupled — Claude/OpenCode/Gemini launch code removed from main binary, agents bring themselves
  checked: false
- id: EC13
  text: Child relationships — Mother mediates event routing between children based on manifest emits/listens declarations
  checked: false
- id: EC14
  text: Git tag integrity — every session gets real start and end tags, no frontmatter-only claims, historical backfill complete
  checked: true
- id: EC15
  text: External developer onramp — cargo generate template, README, working example child that builds and installs in under 5 minutes
  checked: false
---
# refactor: Patina Pre-v1 — Full Architecture Conversion

> Patina is infrastructure for agents, not an agent itself. Mother is the daemon that manages children and toys. Agents are guests that connect directly. Children are composable workers with bounded agency. Toys are composable WASM components children play with. The belief system is the core value. Per [[agents-are-guests-mother-is-infrastructure]] and [[children-have-agency-toys-are-capabilities]].

## Problem

Patina's current architecture grew organically from a CLI tool with MCP bolted on. The result:

1. **Mother is buried in the main binary.** She's 2,688 LOC in `src/mother/` but not a standalone daemon. Agents can't connect to Mother without going through CLI or MCP.

2. **Plugin loader is 47% of the codebase.** `src/plugin/` at 7,338 LOC is bigger than Mother, toys, children, and session combined. WASM loading, host state, and all 14 WIT trait implementations are crammed into one module.

3. **Monolithic WIT world.** Every knowledge-child imports all 14 host interfaces. A session child needing 3 toys gets bindings for 14. The compile-time contract is meaningless.

4. **Toys are permission flags, not components.** Runtime grant checks gate capabilities that should be enforced at compile time by the WIT world. Two layers doing the same job.

5. **MCP is 14% of the codebase and should be 0%.** Agents should connect to Mother directly, not through a protocol bridge that became permanent.

6. **Interface runtimes are hardcoded.** Claude/OpenCode/Gemini launch code (4,518 LOC) is in the main binary. In the "agents are guests" model, agents bring themselves.

7. **The SDK is flat.** One crate, 5 world features, all toys always compiled. No path for external developers to build children without understanding the entire system.

8. **GitHub is a child when it should be a toy.** Data source access is a capability, not an actor.

9. **Sessions don't survive crashes.** 16 sessions have fake end tags (frontmatter only, no git tag).

10. **DuckLake ingestion is migration-era.** JSON-in-DuckDB rows, no enterprise pipeline.

## Goal

Ship Patina pre-v1: the foundation for a local-first WASM P2P agentic knowledge system. Mother as standalone machine-node daemon, composable WASM children with per-child worlds, tiered SDK for external developers, DuckLake on the new model, agents connect directly with persona context, MCP retired. This architecture must support — without blocking — the post-v1 direction: persona crypto namespaces, Mother-to-Mother P2P federation, belief provenance signing, and ZK-provable computation from WASM execution traces.

## Status

Draft. All prior specs abandoned and subsumed. Phases 1-10 ship today. Phase 11 is follow-on.

## Non-Goals

- Multi-provider connector abstraction (GitHub-specific)
- S3 storage backend for DuckLake (follow-on)
- Real-time / streaming ingestion (poll mode only)
- Cross-machine Mother-to-Mother P2P federation (follow-on — architecture must not block it)
- Persona implementation (crypto namespaces, keypairs, persona-scoped children — follow-on, architecture must not block it)
- Belief signing and provenance (follow-on — requires persona keypairs)
- Backward compatibility with pre-v1 child binaries
- Enterprise DuckLake pipeline in this pass (Phase 11 follow-on)

## Current State

```
patina/
├── src/                          ONE BINARY (everything embedded)
│   ├── mother/      2,688 LOC    Broker, state — buried in CLI binary
│   ├── plugin/      7,338 LOC    WASM loader — 47%, too big, wrong place
│   ├── interface/   4,518 LOC    Claude/OpenCode/Gemini — hardcoded
│   ├── mcp/         2,228 LOC    MCP server — should be 0
│   ├── toys/          583 LOC    Host-side toys — right idea, wrong location
│   ├── session/       996 LOC    Session lifecycle — no crash recovery
│   ├── child/         227 LOC    Child traits — minimal, correct
│   └── main.rs      2,132 LOC    CLI — too much logic
├── sdk/              2,160 LOC    Flat, one crate, all toys always compiled
├── children/           401 LOC    ducklake + belief-verifier (tiny)
├── crates/           3,939 LOC    patina-pipe + patina-pipe-types
└── wit/                           5 fixed worlds, monolithic imports
```

## Target State

```
patina/
├── mother/                        STANDALONE DAEMON
│   ├── daemon.rs                  Agent connection listener (Unix socket)
│   ├── broker.rs                  Child lifecycle, toy grants, event routing
│   ├── state.rs                   Persistent state
│   ├── linker.rs                  Per-child WIT linker (from manifest)
│   ├── wasm.rs                    WASM engine + component loader
│   └── host/                      Host-side toy implementations
│       ├── log.rs, state.rs, lake.rs, github.rs, session.rs, measure.rs, ...
│
├── sdk/                           TIERED SDK (the external dev map)
│   ├── patina-sdk-core/           KnowledgeChild trait + log + state
│   ├── patina-sdk-data/           lake, checkpoint, github, measure
│   ├── patina-sdk-agent/          session, query
│   └── patina-sdk/                Full re-export (convenience)
│
├── children/                      WHERE THE LOGIC LIVES
│   ├── ducklake/                  Data ingestion (github+lake+state+checkpoint+measure)
│   ├── belief-verifier/           Belief verification (state+checkpoint+events+belief)
│   ├── session-writer/            Session artifact lifecycle (log+state+session)
│   └── template/                  cargo-generate template for external devs
│
├── wit/
│   ├── toys/                      Individual toy interfaces (canonical source)
│   └── worlds/                    Per-child composed worlds
│
├── cli/                           THIN CLIENT
│   └── main.rs                    patina commands → Mother daemon calls
│
└── layer/                         THE CORE VALUE
    └── surface/epistemic/beliefs/ Belief system persists across everything
```

## Solution

### Phase 1: SDK Restructure — The Map (6 commits)

1. Create `patina-sdk-core` crate — `KnowledgeChildPlugin` trait, `handle`/`tick`/`drain`, `toy-log`, `toy-state`
2. Create `patina-sdk-data` crate — `toy-lake`, `toy-checkpoint`, `toy-github`, `toy-measure` (feature-gated)
3. Create `patina-sdk-agent` crate — `toy-query`, `toy-emit` (feature-gated), `toy-session` (stub only — filled in Phase 4)
4. Refactor `patina-sdk` to re-export tiers
5. Feature-gate the `granted` module — only enabled toys compile bindings
6. Add per-toy features to child Cargo.tomls, verify both children compile

### Phase 2: Per-Child WIT Worlds (6 commits)

7. Split `wit/deps/patina-host/host.wit` into individual toy WIT files under `wit/toys/`
8. Create per-child world files under `wit/worlds/`
9. Update ducklake build to use `ducklake.wit` world
10. Update belief-verifier build to use `belief-verifier.wit` world
11. Copy per-child WIT into SDK tier crates via build scripts
12. Measure binary size reduction

### Phase 3: Per-Child Linker (4 commits)

13. Split `add_to_linker` into per-interface functions
14. Read manifest `[needs].toys` in `KnowledgeChildEngine`
15. Build per-child linker from manifest — only link declared toys
16. Verify sandbox enforcement — child without `lake` in manifest fails to instantiate

### Phase 4: New Toy Interfaces (7 commits)

17. Define `patina:host/github@0.1.0` WIT interface
18. Implement host-side `github.rs` — absorb from native connector, credential injection
19. Add github toy integration tests with fixture data
20. Define `patina:host/session@0.1.0` WIT interface
21. Implement host-side `session.rs` — absorb from `src/session/`
22. Add session toy tests — artifact writes, real git tags, crash handoff
23. Add `toy-github` and `toy-session` features to SDK tiers

### Phase 5: Session-Writer Child (9 commits)

24. Create session-writer world (`log` + `state` + `session`)
25. Scaffold `children/session-writer/` with `KnowledgeChildPlugin` impl
26. Implement handle actions: `note`, `update`, `spec-link`, `close`, `crash-handoff`
27. Wire into `check_in()` — Mother spawns session-writer when agent connects
28. Wire crash recovery — Mother detects agent death via socket EOF, calls session-writer
29. Populate `parent_session` link and copy raw handoff at auto-start (deterministic — synthesis is the agent's job)
30. Fix `display_name` in auto-start sessions — OS user, not interface name
31. Backfill historical fake end tags — create real git tags for 16 sessions with frontmatter-only claims
32. Measure session-writer binary size — target <150KB release

### Phase 6: DuckLake New Model (3 commits)

Migrate DuckLake to the composable model. Enterprise pipeline (watermarks, parquet, bronze/silver/gold) is Phase 11.

33. Migrate ducklake to composed world with toy-github replacing connector
34. Verify basic fetch-and-store cycle works via toy-github
35. End-to-end litmus: anthropics/claude-code issues queryable via standalone DuckDB

### Phase 7: Mother Extraction (5 commits)

36. Create `mother/` crate, `git mv` modules
37. Fix import paths — main binary depends on `mother` crate
38. Implement daemon listener — Unix socket, JSON lines protocol
39. Implement agent connection lifecycle — spawn session-writer on connect, crash-handoff on disconnect
40. Implement daemon startup — `patina mother start/stop/status`, on-demand auto-start

### Phase 8: CLI Thin Client (7 commits)

41. Add daemon client module — connects to Mother socket, handles auto-start
42. `patina context` → Mother call → child handles
43. `patina measure` → Mother call → child handles
44. `patina spec` → Mother call → appropriate child
45. `patina lake` → Mother call → ducklake child
46. Migrate remaining commands to daemon path
47. Remove embedded Mother code from main binary

### Phase 9: MCP Retirement + Interface Decoupling (6 commits)

48. Remove `src/mcp/` (2,228 LOC)
49. Remove `src/interface/runtime/` launch code (~3,500 LOC) — keep `src/interface/internal/` temporarily for session check-in reuse
50. Remove tmux infrastructure
51. Remove `patina-pipe`, `patina-pipe-types` crates, and legacy `[capabilities]`/`[toys]` manifest parsing
52. Remove native github-connector child
53. Update AGENTS.md / CLAUDE.md

### Phase 10: Child Relationships + Polish (11 commits)

54. Extend `plugin.toml` manifest with `[relationships]` — `emits` and `listens`
55. Mother reads relationship declarations, builds event routing table
56. Define `patina:host/peer@0.1.0` for mediated child-to-child events
57. Add `toy-peer` to SDK — `PeerBackend` trait in `patina-sdk-core`
58. DuckLake emits `data-ingested` events
59. Session-writer listens for `data-ingested`
60. Create cargo-generate template for new children
61. Write SDK README — 5-minute onramp guide
62. External developer template tested end-to-end
63. Final binary size audit
64. Update SDK README with relationship documentation

### Phase 11: DuckLake Enterprise Pipeline (follow-on, 12 commits)

**STOP. DO NOT BUILD.** Phase 11 is documented here for continuity only. Commits 65-76 are NOT authorized. A build agent MUST stop after commit 64 and report completion. Phase 11 requires separate authorization via `patina spec promote` or a new spec.

65. Implement endpoint planner for 8 GitHub entity types
66. Implement two-phase ingestion: list pagination → bounded fanout with adaptive backoff
67. Implement watermark cursor system — stable tuples, monotonic progression
68. Implement idempotent upserts — no silent duplicates on replay
69. Write bronze encrypted parquet partitions (`org/repo/entity/date/`)
70. Build silver normalized views with soft-delete support
71. Build gold analytics views for downstream agents/apps
72. Implement reconciliation with bounded tolerance
73. Implement late-arrival handling with replay window
74. Implement dead-letter flow
75. Emit operational telemetry via toy-measure
76. Enterprise end-to-end litmus with full validation

## Implementation Order

```
TODAY (Phases 1-10, 64 commits):
  Phase 1:  SDK Restructure (the map)
  Phase 2:  Per-Child WIT Worlds
  Phase 3:  Per-Child Linker
  Phase 4:  New Toy Interfaces (github, session)
  Phase 5:  Session-Writer Child (minimal proof)
  Phase 6:  DuckLake New Model (migrate, verify)
  Phase 7:  Mother Extraction (standalone daemon)
  Phase 8:  CLI Thin Client
  Phase 9:  MCP Retirement + Interface Decoupling
  Phase 10: Child Relationships + Polish

FOLLOW-ON (Phase 11, 12 commits):
  Phase 11: DuckLake Enterprise Pipeline
```

Phases 1-3 unblock child development. Phases 4-6 prove the model. Phases 7-10 complete the architecture. Phase 11 adds enterprise data engineering.

## Core Values Anchor

Every line of code in this conversion must hold to Patina's core values and the Rust house style. This is not optional guidance — it is the quality gate.

### From `layer/core/values/`

- **[[unix-philosophy]]** — One tool, one job, done well. Children are single-purpose. Toys are single-capability. Mother orchestrates, doesn't do the work.
- **[[dependable-rust]]** — Small public interfaces, hidden internals. The `internal/` module pattern is mandatory.
- **[[adapter-pattern]]** — Trait-based adapters, never concrete types in core logic. Toys are WIT interfaces, not concrete structs.
- **[[patina-identity]]** — The binary is the pipeline, the layer is the product. If it's not protocol operation/tooling/infrastructure, it's a plugin.
- **[[safety-boundaries]]** — Project-scoped only. Children are sandboxed. Toys are capability-gated.
- **[[oxidized-knowledge]]** + **[[beliefs-live-at-two-levels]]** — Project beliefs live in `layer/` (git-tracked, travel with code). Persona beliefs live in Mother's state (crypto-scoped, span projects, sync via P2P). Different lifetimes, different storage, same signing key.
- **[[session-capture]]** — Scripts handle mechanics, humans handle meaning. Session-writer embodies this.
- **[[spec-driven-design]]** — This spec authorizes 64 commits (Phases 1-10). Phase 11 is a separate authorization. When the build agent encounters an edge case, stop and ask.

### Gjengset-Lens Rust Quality (from `rust-house-style.md`)

1. **Types encode invariants.** Newtypes for IDs. Closed enums with explicit match arms.
2. **Errors are first-class API.** `Result<T, E>`, `anyhow::Context` at boundaries. No `.ok()`, no `unwrap()`.
3. **Separate concerns.** Data access, core logic, presentation never in one function.
4. **O(delta) not O(n).** Work proportional to change size.
5. **Parse at boundaries, type the interior.** WIT accepts strings → parse into typed structs immediately.
6. **No unchecked indexing or arithmetic.** `slice.get(i)`, `checked_sub`.
7. **Sync-first.** Async only for measured I/O concurrency.

### Build Agent Instructions: Scalpel, Not Shotgun

- **Git commits are surgical.** One logical change per commit. Never bundle unrelated changes.
- **Read before write.** Understand existing code before changing it.
- **Move code, don't rewrite.** Pure move → fix imports → add functionality (separate commits).
- **Test at every step.** `cargo test` after every commit. Exception: move+fix commit pairs where the first may break compile — pair must be consecutive, second restores green.
- **No drive-by refactors.** This spec authorizes 64 specific commits. Unauthorized changes are scope creep.
- **Preserve git history.** `git mv` for moves. Never delete-and-recreate.
- **Small diffs, clear messages.** Reviewable in under 2 minutes.

## Resolved Decisions

1. **Patina is infrastructure for agents, not an agent itself.** Mother manages children and toys. Agents are guests. The belief system is the core value.
2. **Children have bounded agency.** Agency within the sandbox Mother grants. `handle()` mandatory, `tick()`/`drain()` optional.
3. **Toys are composable WIT components, not permission flags.** Compile-time enforcement via per-child worlds.
4. **Per-child WIT worlds.** Monolithic `knowledge-child` world is retired.
5. **GitHub is a toy, not a child.** Native github-connector is retired.
6. **No MCP.** Agents connect via JSON lines over Unix socket. MCP removed entirely.
7. **No backward compatibility.** Single user, full convert.
8. **The SDK is the external developer map.** Tiered crates, feature-gated toys, cargo-generate template.
9. **Mother is a standalone daemon.** Extracted from CLI. Accepts agent connections on Unix socket.
10. **DuckDB stays host-side in Mother.** `toy-lake` is a host-implemented WIT interface. Children call `granted::lake()`, Mother handles DuckDB. Children never embed DuckDB. This is already how it works — formalizing, not changing.
11. **Daemon starts on-demand.** CLI auto-starts Mother if not running. No launchd/systemd needed.
12. **Agent connection protocol is JSON lines over Unix socket.** `{action, payload}` → `{result}` or `{error}`. Same pattern as child `handle()`. Version field in first message for future compatibility. No streaming, cancellation, or auth in v1 — single local user.
13. **WIT world composition via `cargo-component`.** Each child's `Cargo.toml` points to its own world file in `wit/worlds/`. If `cargo-component` can't handle custom worlds, use WAC as build-time compositor. Decision locked at start of Phase 2.
14. **Manifest schema is `[needs].toys` + `[needs.scopes]`.** Migrated from current `[capabilities]`/`[toys]` split. Old schema supported during migration, removed in Phase 9.
15. **Crash detection via Unix socket EOF.** When agent's socket closes, Mother knows immediately. No heartbeat needed.

### Architecture Must Not Block (post-v1 direction)

These are NOT in scope, but every decision above must be compatible with:

16. **Mother = machine node.** One Mother per machine. Personas are crypto namespaces within Mother, not separate instances. Per [[persona-is-a-patina-instance]] (scoped) and [[session-20260320-212325-011658000]].
17. **Personas span Mothers.** Same persona keypair on multiple machines. Beliefs sync via P2P. Mother-to-Mother federation is machine-level, not persona-level.
18. **Beliefs live at two levels.** Project beliefs in `layer/` (git-tracked). Persona beliefs in Mother state (crypto-scoped, P2P-synced). Per [[beliefs-live-at-two-levels]].
19. **Agent connection carries persona context.** The `connect` handshake must support a `persona` field so Mother can scope children, toys, and beliefs to the active persona.
20. **WASM determinism enables ZK proofs.** Per-child worlds, bounded execution, and measured I/O are the substrate for future STARK proof generation from WASM execution traces.
21. **Tmux is agent-launcher, not architectural.** Session liveness is socket connection, not tmux lane. Agents without tmux have full sessions. Per [[tmux-lane-defines-active-session]] (scoped).

## Verification

| Phase | Gate |
|-------|------|
| 1 | `cargo build` succeeds for all 3 SDK tiers independently |
| 2 | Each child compiles with its own world, binary sizes decrease |
| 3 | Child without `lake` declaration fails to instantiate |
| 4 | `cargo test` for toy-github and toy-session host implementations |
| 5 | Session-writer handles notes, survives crash, creates real git tags, <150KB |
| 6 | DuckLake fetches via toy-github, queryable via standalone DuckDB |
| 7 | Mother starts as daemon, accepts connection, routes to child |
| 8 | `patina context` works via CLI → Mother → child round-trip |
| 9 | MCP and interface code deleted, all tests pass |
| 10 | DuckLake emits event → Mother routes → session-writer captures; template builds in <5min |
| 11 | Enterprise pipeline: watermarks, parquet, reconciliation, telemetry all pass |

## Exit Criteria

See frontmatter — 15 exit criteria for Phases 1-10. Phase 11 has its own criteria defined at build time.

## Build Readiness

- [x] All prior specs abandoned and subsumed
- [x] Architectural beliefs aligned and updated (2026-03-21)
- [x] Data contract locked (from [[ducklake-github-lakehouse-ingestion]])
- [x] SDK trait abstractions validated (generic backend pattern is correct)
- [x] WIT interfaces already separate packages (composable foundation exists)
- [x] Pi-mono reference repo studied (MCP-free agent model validated)
- [x] Single user, no backward compatibility constraints
- [x] Design doc complete — 64 commits (Phases 1-10) + 12 follow-on (Phase 11)
- [x] All open questions resolved as decisions
