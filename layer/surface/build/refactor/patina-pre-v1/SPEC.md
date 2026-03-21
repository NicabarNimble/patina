---
type: refactor
id: patina-pre-v1
status: draft
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
  checked: false
- id: EC5
  text: toy-session WIT interface — Mother implements session artifact writes, git tag creation, crash recovery handoff
  checked: false
- id: EC6
  text: session-writer child — minimal WASM child (log + state + session toys), spawned at agent connection, handles artifact lifecycle and crash recovery
  checked: false
- id: EC7
  text: DuckLake enterprise — WASM child with composed world (github + lake + state + checkpoint + measure + log), two-phase ingestion, watermarks, idempotent upserts, encrypted parquet, bronze/silver/gold outputs
  checked: false
- id: EC8
  text: Mother extracted — standalone daemon crate, accepts agent connections, manages children and toys, separate from CLI binary
  checked: false
- id: EC9
  text: Agent connection protocol — simple JSON protocol any agent can speak to connect to Mother, no MCP required
  checked: false
- id: EC10
  text: CLI is thin client — patina commands delegate to Mother daemon, no embedded Mother logic
  checked: false
- id: EC11
  text: MCP retired — MCP server removed from main binary, replaced by agent connection protocol
  checked: false
- id: EC12
  text: Interface runtimes decoupled — Claude/OpenCode/Gemini launch code removed from main binary, agents bring themselves
  checked: false
- id: EC13
  text: Child relationships — Mother mediates event routing between children based on manifest emits/listens declarations
  checked: false
- id: EC14
  text: Quality gates — DuckLake reconciliation, late-arrival, dead-letter, parity samples against GitHub API totals all pass
  checked: false
- id: EC15
  text: Telemetry — all children emit operational metrics via toy-measure, Mother aggregates
  checked: false
- id: EC16
  text: Git tag integrity — every session gets real start and end tags, no frontmatter-only claims
  checked: false
- id: EC17
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

8. **GitHub is a child when it should be a toy.** Data source access is a capability, not an actor. The native github-connector adds process overhead and pipe protocol complexity for what should be a WIT function call.

9. **Sessions don't survive crashes.** 16 sessions have fake end tags (frontmatter only, no git tag). No crash recovery mechanism exists.

10. **DuckLake ingestion is migration-era.** JSON-in-DuckDB rows, no watermarks, no idempotent upserts, no encrypted parquet, no bronze/silver/gold outputs.

## Goal

Ship Patina pre-v1: a complete system where Mother is a standalone daemon, children are composable WASM workers with bounded agency, toys are WIT interfaces children compose, agents connect directly, the SDK is the onramp for external developers, and DuckLake is enterprise-grade.

## Status

Draft. All prior specs abandoned and subsumed. This is the single spec governing the complete conversion.

## Non-Goals

- Multi-provider connector abstraction (GitHub-specific in pre-v1)
- S3 storage backend for DuckLake (follow-on)
- Real-time / streaming ingestion (poll mode only)
- Cross-machine Mother federation (local-first only)
- Custom Parquet writer (DuckLake handles this)
- Backward compatibility with pre-v1 child binaries

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
│   ├── daemon.rs                  Agent connection listener
│   ├── broker.rs                  Child lifecycle, toy grants, event routing
│   ├── state.rs                   Persistent state
│   ├── linker.rs                  Per-child WIT linker (from manifest)
│   ├── wasm.rs                    WASM engine + component loader
│   └── host/                      Host-side toy implementations
│       ├── log.rs
│       ├── state.rs
│       ├── lake.rs
│       ├── github.rs              NEW — toy-github
│       ├── session.rs             NEW — toy-session
│       ├── measure.rs
│       ├── checkpoint.rs
│       └── ...
│
├── sdk/                           TIERED SDK (the external dev map)
│   ├── patina-sdk-core/           KnowledgeChild trait + log + state
│   ├── patina-sdk-data/           lake, checkpoint, github, measure
│   ├── patina-sdk-agent/          llm, conversation, session, query
│   └── patina-sdk/                Full re-export (convenience)
│
├── children/                      WHERE THE LOGIC LIVES
│   ├── ducklake/                  Enterprise ingestion (github+lake+state+checkpoint+measure)
│   ├── belief-verifier/           Belief verification (state+checkpoint+events+belief)
│   ├── session-writer/            Session artifact lifecycle (log+state+session)
│   └── template/                  cargo-generate template for external devs
│
├── wit/
│   ├── toys/                      Individual toy interfaces (composable)
│   │   ├── log.wit
│   │   ├── state.wit
│   │   ├── lake.wit
│   │   ├── github.wit
│   │   ├── session.wit
│   │   ├── measure.wit
│   │   ├── checkpoint.wit
│   │   ├── belief.wit
│   │   ├── graph.wit
│   │   ├── query.wit
│   │   └── events.wit
│   └── worlds/                    Per-child composed worlds
│       ├── ducklake.wit
│       ├── session-writer.wit
│       ├── belief-verifier.wit
│       └── ...
│
├── cli/                           THIN CLIENT
│   └── main.rs                    patina commands → Mother daemon calls
│
└── layer/                         THE CORE VALUE
    └── surface/epistemic/beliefs/ Belief system persists across everything
```

## Solution

### Phase 1: SDK Restructure — The Map

The SDK is the onramp. External developers see this first. Ship it first.

1. Create `patina-sdk-core` crate — `KnowledgeChildPlugin` trait, `handle`/`tick`/`drain`, `toy-log`, `toy-state`
2. Create `patina-sdk-data` crate — `toy-lake`, `toy-checkpoint`, `toy-github`, `toy-measure` (feature-gated)
3. Create `patina-sdk-agent` crate — `toy-session`, `toy-query` (feature-gated, future: `toy-llm`, `toy-conversation`)
4. Refactor `patina-sdk` to re-export tiers
5. Add per-toy feature flags — `toy-log`, `toy-state`, `toy-lake`, etc.
6. Feature-gate the `granted` module — only enabled toys compile bindings
7. Write `cargo-generate` template for new children
8. Write README with 5-minute onramp guide

### Phase 2: Per-Child WIT Worlds

Break the monolithic world into composable pieces.

9. Split `wit/deps/patina-host/host.wit` into individual toy WIT files under `wit/toys/`
10. Create per-child world files under `wit/worlds/` — each imports only needed toys
11. Update `wit-bindgen` build to use per-child worlds
12. Migrate ducklake to `ducklake.wit` world (7 imports, not 15)
13. Migrate belief-verifier to `belief-verifier.wit` world
14. Verify binary sizes decrease

### Phase 3: Per-Child Linker

Mother links only what the child declares.

15. Refactor `KnowledgeChildEngine` — build `Linker<HostState>` per-child from manifest `[needs].toys`
16. Split `add_to_linker` into per-interface functions
17. Keep `GrantedToys` runtime checks as defense-in-depth
18. Verify: child that doesn't declare `lake` fails to instantiate if binary imports lake

### Phase 4: New Toy Interfaces

Build the toys that don't exist yet.

19. Define `patina:host/github@0.1.0` — issues, PRs, comments, reviews, events with pagination and rate-limit backoff
20. Implement host-side `github.rs` — absorb from native github-connector, credential injection via `HostState`
21. Add github toy integration tests with fixture data
22. Define `patina:host/session@0.1.0` — artifact write, git tag creation, status update, crash handoff
23. Implement host-side `session.rs` — absorb from `src/session/`
24. Add session toy tests — artifact writes, real git tags, crash handoff
25. Add `toy-github` and `toy-session` features to SDK tiers

### Phase 5: Session-Writer Child

First child born on the new model. Proves minimal child thesis.

26. Create session-writer world (`log` + `state` + `session`)
27. Scaffold `children/session-writer/` with `KnowledgeChildPlugin` impl
28. Implement handle actions: `note`, `update`, `spec-link`, `close`, `crash-handoff`
29. Wire into `check_in()` — Mother spawns session-writer when agent connects
30. Wire crash recovery — Mother detects agent death, calls session-writer for handoff and real end tag
31. Populate `parent_session` link and copy raw previous session handoff at auto-start (deterministic file read — synthesis is the agent's job)
32. Fix `display_name` in auto-start sessions — OS user, not interface name
33. Backfill historical fake end tags — create real git tags for 16 sessions with frontmatter-only claims
34. Measure session-writer binary size — target <150KB release

### Phase 6: DuckLake Enterprise

Full enterprise ingestion on the composable model.

35. Migrate ducklake to composed world with toy-github replacing connector
36. Implement endpoint planner for 8 GitHub entity types
37. Implement two-phase pipeline: list pagination → bounded fanout with adaptive backoff
38. Implement watermarks with stable entity keys and monotonic cursor progression
39. Implement idempotent upserts — no silent duplicates on replay
40. Write bronze encrypted parquet partitions (`org/repo/entity/date/`)
41. Build silver normalized views, gold analytics views
42. Implement gold stable query views for downstream agents/apps
43. Implement reconciliation with bounded tolerance (max 2% or 25 records)
44. Implement late-arrival handling with 24h replay window
45. Implement dead-letter flow for unprocessable entities
46. Emit run-level + endpoint-level telemetry via toy-measure
47. End-to-end litmus: anthropics/claude-code full ingestion, queryable via standalone DuckDB

### Phase 7: Mother Extraction

Pull Mother out of the CLI binary into a standalone daemon.

48. Create `mother/` crate, `git mv` modules from `src/mother/`, `src/toys/`, `src/child/`, `src/plugin/internal/`
49. Fix import paths — main binary depends on `mother` crate
50. Implement daemon listener — Unix socket, JSON lines, `handle(action, payload)` pattern
51. Implement agent connection lifecycle — spawn session-writer on connect, crash-handoff on disconnect
52. Implement daemon startup — `patina mother start/stop/status`, on-demand auto-start

### Phase 8: CLI Thin Client

CLI becomes a thin client that talks to Mother.

53. Add daemon client module — connects to Mother socket, handles auto-start
54. `patina context` → Mother call → child handles
55. `patina measure` → Mother call → child handles
56. `patina spec` → Mother call → appropriate child
57. `patina lake` → Mother call → ducklake child
58. Migrate remaining commands to daemon path
59. Remove embedded Mother code from main binary — binary size drops significantly

### Phase 9: MCP Retirement + Interface Decoupling

Remove the bridge. Agents bring themselves.

60. Remove `src/mcp/` (2,228 LOC)
61. Remove `src/interface/` runtime launch code (~3,500 LOC) — Claude/OpenCode/Gemini connect as agents via the connection protocol
62. Remove tmux infrastructure — agents manage their own terminal sessions
63. Remove `patina-pipe` and `patina-pipe-types` crates (native child pipe protocol retired)
64. Remove native github-connector child (absorbed into toy-github)
65. Update AGENTS.md / CLAUDE.md to describe agent connection instead of MCP tools

### Phase 10: Child Relationships + Polish

Complete the composable vision.

66. Extend `plugin.toml` manifest with `[relationships]` — `emits` and `listens` declarations
67. Mother reads relationship declarations, builds event routing table at child load
68. Define `patina:host/peer@0.1.0` for mediated child-to-child events
69. Add `toy-peer` to SDK — `PeerBackend` trait in `patina-sdk-core`, feature-gated
70. DuckLake emits `data-ingested` events after successful ingestion runs
71. Session-writer listens for `data-ingested` — Mother routes, session-writer captures as activity
72. Final binary size audit — all children measured, release targets confirmed
73. External developer template tested end-to-end — new child in 5 minutes
74. Update SDK README with relationship documentation and cross-child examples

## Implementation Order

Phases are sequential. Each builds on the last.

```
Phase 1:  SDK Restructure (the map)
Phase 2:  Per-Child WIT Worlds
Phase 3:  Per-Child Linker
Phase 4:  New Toy Interfaces (github, session)
Phase 5:  Session-Writer Child (minimal proof)
Phase 6:  DuckLake Enterprise (real proof)
Phase 7:  Mother Extraction (standalone daemon)
Phase 8:  CLI Thin Client
Phase 9:  MCP Retirement + Interface Decoupling
Phase 10: Child Relationships + Polish
```

Phases 1-3 unblock child development. Phases 4-6 prove the model. Phases 7-10 complete the architecture.

## Core Values Anchor

Every line of code in this conversion must hold to Patina's core values and the Rust house style. This is not optional guidance — it is the quality gate.

### From `layer/core/values/`

- **[[unix-philosophy]]** — One tool, one job, done well. Children are single-purpose. Toys are single-capability. Mother orchestrates, doesn't do the work. If a component is a "system" (coordinates multiple operations), decompose it into tools.

- **[[dependable-rust]]** — Small public interfaces, hidden internals. Every module states what it does in one sentence. Implementation changes never break callers. The `internal/` module pattern is mandatory.

- **[[adapter-pattern]]** — Trait-based adapters, never concrete types in core logic. You can swap the implementation without changing callers. This is why toys are WIT interfaces, not concrete structs.

- **[[patina-identity]]** — The binary is the pipeline, the layer is the product, the protocol is the contract. Before adding a module: is it protocol operation, protocol tooling, or protocol infrastructure? If none, it's a plugin — don't add it to the binary.

- **[[safety-boundaries]]** — Project-scoped only. User consent before major operations. No surprise side effects. Children are sandboxed. Toys are capability-gated. Mother never reaches outside her boundaries.

- **[[oxidized-knowledge]]** — Project knowledge is git-tracked and shared. Persona knowledge is local and private. Never mix them.

- **[[session-capture]]** — Capture with minimal friction. Scripts handle mechanics, humans handle meaning. The session-writer child embodies this — it captures automatically, the agent enriches with meaning.

- **[[spec-driven-design]]** — Every non-trivial change is authorized by a spec. This spec authorizes 74 commits across 10 phases. When the build agent encounters an edge case, the correct action is to stop and ask — not to make a judgment call.

### Gjengset-Lens Rust Quality (from `rust-house-style.md`)

All code in this conversion MUST pass a Jon Gjengset audit. This means:

1. **Types encode invariants.** No stringly-typed identifiers. `ToyId`, `ChildId`, `SessionId` are newtypes, not `String`. Enums model finite sets (toy kinds, child states, session status). Closed enums with explicit match arms — no `_ =>` catch-all that silently swallows new variants.

2. **Errors are first-class API.** `Result<T, E>` for all fallible work. `anyhow::Context` at every boundary. No `.ok()` to erase errors. No `unwrap()` outside tests. Failures carry context: what you were doing, what went wrong, what the inputs were.

3. **Separate concerns.** Data access, core logic, and presentation are never mixed in one function. A toy implementation fetches data. The child logic decides what to do with it. The session artifact formats it for output.

4. **O(delta) not O(n).** Work proportional to change size. Incremental compilation, incremental linking, incremental ingestion. DuckLake's watermark system is O(delta) by design — fetch only what changed.

5. **Parse at boundaries, type the interior.** WIT boundaries accept strings. The first thing inside the boundary is parsing into typed structs. Interior code never touches raw strings for structured data.

6. **No unchecked indexing or arithmetic.** `slice.get(i)` not `slice[i]`. `checked_sub` not `-`. Validate early, return descriptive errors.

7. **Sync-first.** Async only when it buys measurable I/O concurrency. Mother's daemon listener may be async. Child `handle()` calls are sync. Don't async-ify pure logic.

### Build Agent Instructions: Scalpel, Not Shotgun

When building this spec, the agent MUST:

- **Git commits are surgical.** One logical change per commit. "Add toy-github WIT interface" is one commit. "Implement host-side github.rs" is another. Never bundle unrelated changes. Never commit generated code alongside hand-written code.

- **Read before write.** Always read the file you're about to modify. Understand the existing code before changing it. Never write to a file you haven't read in this conversation.

- **Move code, don't rewrite.** When extracting Mother from `src/mother/` to `mother/`, the first commit is a pure move (no logic changes). The second commit adapts imports. The third commit adds new functionality. Reviewers must be able to verify "nothing changed except location" in the move commit.

- **Test at every step.** `cargo test` must pass after every commit, not just at the end of a phase. One documented exception: move+fix commit pairs (e.g., `git mv` then fix imports) where the first may break compile — the pair must be consecutive and the second must restore green.

- **No drive-by refactors.** If you notice something unrelated that should be improved, note it — don't fix it. This spec authorizes 74 specific commits. Unauthorized changes are scope creep per [[spec-driven-design]].

- **Preserve git history.** Use `git mv` for renames and moves so history follows the file. Never delete-and-recreate when a move would preserve blame.

- **Small diffs, clear messages.** Each commit message says what changed and why. The diff should be reviewable in under 2 minutes. If a diff is too big to review, split it.

## Resolved Decisions

1. **Patina is infrastructure for agents, not an agent itself.** Mother manages children and toys. Agents are guests. The belief system is the core value.

2. **Children have bounded agency.** Agency within the sandbox Mother grants. Not fully autonomous, not fully directed. `handle()` mandatory, `tick()`/`drain()` optional.

3. **Toys are composable WIT components, not permission flags.** The WIT interface IS the capability. Compile-time enforcement via per-child worlds. Runtime grants are defense-in-depth only.

4. **Per-child WIT worlds.** Each child declares exactly the toys it needs. The monolithic `knowledge-child` world is retired.

5. **GitHub is a toy, not a child.** Data source access is a capability interface. The native github-connector is retired.

6. **No MCP.** Agents connect to Mother via a simple protocol. MCP is removed entirely, not kept as a shim.

7. **No backward compatibility.** Single user, full convert. Pre-v1 child binaries are not supported.

8. **The SDK is the external developer map.** Tiered crates, feature-gated toys, cargo-generate template, 5-minute onramp.

9. **Mother is a standalone daemon.** Extracted from the CLI binary. Accepts agent connections. Manages all children and toys.

10. **Two-phase ingestion is mandatory for DuckLake.** List pagination first, bounded fanout second. No single-pass deep crawl.

11. **Bronze/silver/gold outputs.** Bronze append-only parquet, silver normalized with soft-deletes, gold stable analytics views.

## Verification

Each phase has a natural verification gate:

| Phase | Gate |
|-------|------|
| 1 | `cargo build` succeeds for all 3 SDK tiers independently |
| 2 | Each child compiles with its own world, binary sizes decrease |
| 3 | Child without `lake` declaration fails to instantiate if binary imports lake |
| 4 | `cargo test` for toy-github and toy-session host implementations |
| 5 | Session-writer handles notes, survives simulated crash, creates real git tags, <150KB |
| 6 | DuckLake ingests anthropics/claude-code, queryable via DuckDB, reconciliation passes |
| 7 | Mother starts as daemon, accepts connection, routes to child, returns response |
| 8 | `patina context` works via CLI → Mother → child round-trip |
| 9 | MCP and interface code deleted, all tests pass without them |
| 10 | DuckLake emits event → Mother routes → session-writer captures |

## Exit Criteria

See frontmatter — 17 discrete checkpoints covering every phase.

## Build Readiness

- [x] All prior specs abandoned and subsumed
- [x] Architectural beliefs aligned and updated (2026-03-21)
- [x] Data contract locked (from [[ducklake-github-lakehouse-ingestion]])
- [x] SDK trait abstractions validated (generic backend pattern is correct)
- [x] WIT interfaces already separate packages (composable foundation exists)
- [x] Pi-mono reference repo studied (MCP-free agent model validated)
- [x] Single user, no backward compatibility constraints
- [x] Design doc complete — 74 commits across 10 phases with file paths, verification gates, and open questions
