---
type: refactor
id: patina-code-to-vision
status: draft
created: 2026-03-22
sessions:
  origin: 20260321-164003-365905000
beliefs:
- core-primitives-are-not-children
- core-verbs-standalone-mother-additive
- core-baseline-child-strategy-extensions
- agents-are-guests-mother-is-infrastructure
- mother-is-the-daemon
exit_criteria:
- id: CV1
  text: Mother is a standalone daemon in the mother/ crate — all runtime logic (state, broker, registry, graph, events, tasks) lives there, not split across three locations
  checked: false
- id: CV2
  text: CLI binary has zero Mother runtime code — it talks to Mother over Unix socket or runs core verbs standalone
  checked: false
- id: CV3
  text: Core verbs (scrape, scry, assay, context, belief, oxidize) have an explicit, command-by-command Mother-unavailable policy documented in this spec and verified by command tests (no implicit placeholder-filter fallback behavior)
  checked: false
- id: CV4
  text: Pre-v1 extracted-daemon probe routing is removed from canonical core command paths (`context`, `measure`, `spec`, `lake`, `scry`) — no `try_daemon_*` probes or `contains("not yet implemented")` filtering in those paths
  checked: false
- id: CV5
  text: "cargo check -q" produces zero warnings
  checked: false
- id: CV6
  text: Vocabulary migration completes with 1:1 parity and bridge removal — runtime code uses child vocabulary (`ChildManifest`, `ChildKind`, `ChildEngine`), `src/plugin/` is removed, and any temporary compatibility bridge is deleted only after parity proof (unless user explicitly approves exception)
  checked: false
- id: CV7
  text: Mother startup guarantees bundled children (`measure-health`, `session-writer`) are loaded and visible in health/status output when daemon boots successfully
  checked: false
- id: CV8
  text: Project manifest exists — a project declares what children it needs and Mother resolves them on connect
  checked: false
- id: CV9
  text: spec-manager is a child — all spec operations route through Mother to this child, not through core CLI code
  checked: false
- id: CV10
  text: toy-layer-fs and toy-git WIT interfaces exist with Mother host implementations
  checked: false
- id: CV11
  text: Scrape strategy boundary is explicit and enforceable — layer/beliefs remain core, and code/git lanes are extraction-ready and independently pluggable without breaking current core scrape behavior. Child extraction happens only after 1:1 parity proof (or explicit user override)
  checked: false
- id: CV12
  text: "patina spec list" without Mother returns clear "spec-manager not available" error
  checked: false
- id: CV13
  text: spec lifecycle supports rename and reopen (in spec-manager child)
  checked: false
- id: CV14
  text: spec complete and spec abandon require human confirmation
  checked: false
- id: CV15
  text: doctor is a child, not core CLI
  checked: false
- id: CV16
  text: version command decoupled from spec workflow — shows version without querying spec system
  checked: false
- id: CV17
  text: session is not a core command — session artifacts are written by agents/children (session-writer), not by a core CLI verb
  checked: false
- id: CV18
  text: lake is a child, not core CLI
  checked: false
---
# refactor: Make the codebase reflect the architecture vision

> The code must match the vision. Not docs about the vision. Not governance about the vision. The code.

## Problem

Patina has a clear architecture: Patina is the knowledge protocol (beliefs at the core), Mother is standalone infrastructure (daemon, children, toys), children are opt-in extensions. But the codebase doesn't match:

- Mother is three tangled modules across the binary, not one standalone daemon
- Daemon stubs sit in front of working core verbs returning "not yet implemented"
- Spec, lake, session, doctor, measure are woven into core as if they're fundamental
- Plugin vocabulary persists half-migrated (src/plugin/ and src/child/ coexist, plugins/ output dir at root)
- 40 warnings remain from partially severed command paths (warning cleanup must be proof-driven)
- src/toys/ has toy host implementations in the CLI binary — should be in mother/ crate

### Claim Discipline (required)

Every state claim in this spec must be backed by code evidence before implementation:

- `file:line` references for structure claims,
- command output for behavior claims,
- and updates to the CV truth map in this spec when evidence changes.

No assumption-only planning.

## Goal

Make the code match the vision. When a new contributor reads the code, they see:
- Core CLI: belief, scrape, scry, assay, context, oxidize — standalone, no daemon
- Mother: standalone daemon, hosts children, grants toys, connects agents
- Children: opt-in WASM extensions installed per-project

## Execution Contract (anti-drift)

This spec is execution-constrained. Any agent implementing it must follow these rules:

Core-values anchor is mandatory at phase start:

- `layer/core/spec-driven-design.md`
- `layer/core/dependable-rust.md`
- `layer/core/safety-boundaries.md`
- `layer/core/unix-philosophy.md`

1. No silent scope changes.
   - If a phase reveals missing prerequisite work, update SPEC/DESIGN first, then implement.
   - Do not "just continue" with hidden assumptions.

2. No deferral language in implementation commits.
   - Forbidden closure phrases: "later", "future", "follow-on", "placeholder", "stub for now" for CV-scoped paths.
   - If a CV requires behavior, ship behavior or mark CV false.

3. Claim discipline is mandatory.
   - Every state claim must have evidence (file:line or command output) in this spec's truth map/logs.
   - Unverified claims must be labeled `unverified`.
   - Read code before write code.

4. Criteria integrity.
   - CV text cannot be weakened to fit current code.
   - If wording is wrong, record amendment rationale under `Resolved Decisions` and preserve intent strength.

5. One-phase-at-a-time gate.
   - A phase starts only when prerequisite claims are verified.
   - A phase ends only when phase verification commands pass and CV truth map is updated.
   - Git updates with scalpel, not shotgun: phase-scoped edits, no broad opportunistic rewrites.

6. No ghost completion.
   - A CV may be checked only when proof is reproducible by another agent from SPEC+DESIGN alone.

## Phase Gate Policy

- Each phase must have:
  - entry conditions (what must be true first),
  - implementation commits,
  - exit proofs (commands + expected key lines),
  - CV truth-map updates.
- If proofs fail, phase remains open; do not start next phase.

## Runtime Policy Lock

Per-command behavior with Mother unavailable must be explicit and stable during this spec:

- `mother-required` (hard fail), or
- `snapshot/degraded` (defined local behavior).

No implicit fallback filtering on placeholder daemon responses is allowed as a final state.

## The Architecture (frozen)

### Patina Core (the protocol — snapshot-capable, living mode policy explicit)

The knowledge system. Beliefs are the core product. Everything serves the belief loop.

**Core commands in the CLI binary:**
- `patina scrape` — index knowledge (orchestrator; strategies can be children)
- `patina oxidize` — build semantic indices
- `patina scry` — search knowledge
- `patina assay` — structural analysis
- `patina context` — guidance synthesis from beliefs + knowledge
- `patina belief` — belief CRUD, audit, grounding

**Core scrape strategies that stay built-in:**
- layer/ scraping (patterns, sessions) — every Patina project has this
- beliefs/ scraping (belief extraction, verification) — beliefs are the product

**Core scrape strategies that become children:**
- code scraping (AST, imports, call graphs) — not every project has code
- git scraping — most projects have this but it's separable

### Mother (standalone daemon — her own process, her own crate)

All Mother runtime logic lives in `mother/`:
- State store (SQLite: children, tasks, events, cursors, beliefs, sessions)
- Child registry (load, lifecycle, heartbeat, knowledge cycles)
- Broker (source orchestration, capability grants)
- Graph (cross-project knowledge, belief sharing)
- Event streams (named streams, offset tracking, ack)
- Daemon server (Unix socket, JSON-lines protocol)
- Task queue (enqueue, lease, complete, dead-letter)

Mother ships with **bundled children** (always available when she runs):
- `measure-health` — system health reporting across the 5 verb categories
- `session-writer` — session artifact lifecycle, crash recovery

Mother resolves **project children** on connect:
- Project connects with a manifest of needs
- Mother checks local inventory
- Missing children resolved from registry (future: other Mothers via P2P)
- Children are WASM files — portable, sandboxed

### Children (opt-in WASM extensions)

**Workflow children:**
- `spec-manager` — spec lifecycle (create, promote, complete, rename, reopen)
- `doctor` — project diagnostics and health checks

**Strategy children:**
- `scrape-code` — code AST/imports/call-graphs via tree-sitter grammars
- `scrape-git` — commit history, co-change, conventional commits
- `github-scraper` (ducklake) — GitHub issues/PRs/reviews into lake

**Data children:**
- `lake-manager` — lake CRUD and storage management

**Verification children:**
- `belief-verifier` — multi-phase belief verification

## Non-Goals

- No P2P Mother-to-Mother protocol implementation (architecture must not block it)
- No persona crypto implementation (field stays in protocol)
- No enterprise DuckLake pipeline
- No belief loop redesign (belief command stays core, loop evolves later)
- No new user-facing features

## Current State (verified)

- Mother runtime is split across `mother/`, `src/mother/`, and `src/commands/mother/`.
- Core verb commands still run extracted daemon-first probes (`try_daemon_*`) with fallback filtering on placeholder responses.
- `spec`, `lake`, `doctor`, `session`, and `measure` remain core CLI command surfaces.
- Vocabulary bridge is partial: canonical child manifests are in place, but `src/plugin/` and `src/child/` coexist.
- `cargo check` reports 40 warnings.
- Scrape code path is already grammar-driven and multi-language-capable; extraction work must preserve grammar abstraction.

### CV Truth Map (Phase 0 refresh)

Refreshed: 2026-03-22

Status keys:

- `verified-false` = code disproves criterion today
- `verified-partial` = some pieces exist, criterion not yet satisfied
- `verified-true` = criterion appears satisfied
- `unverified` = explicit proof still needed

| CV | Status | Evidence |
|---|---|---|
| CV1 | verified-false | Runtime remains split (`mother/src/*.rs`, `src/mother/*.rs`, `src/commands/mother/*.rs`). |
| CV2 | verified-false | CLI still contains Mother runtime command/server code (`src/commands/mother/daemon.rs`, `src/commands/mother/mod.rs`). |
| CV3 | verified-false | Core command paths still include extracted-daemon probe routing and implicit placeholder fallback behavior. |
| CV4 | verified-false | `try_daemon_*` + `contains("not yet implemented")` filtering still present in `context`, `measure`, `spec`, `lake`, `scry`. |
| CV5 | verified-false | `cargo check` still reports 40 warnings (refresh run 2026-03-22). |
| CV6 | verified-partial | Child vocabulary bridge exists (`child.toml` + `kind`), but `src/plugin/*` still coexists with `src/child/*`. |
| CV7 | verified-false | `children/measure-health/` is absent; bundled-load guarantee for measure-health/session-writer is not implemented. |
| CV8 | verified-false | No project child-needs manifest + connect-time resolution flow (only unrelated bootstrap `manifest.toml` snapshot path exists). |
| CV9 | verified-false | Spec system remains in core (`src/commands/spec/internal/*`); `children/spec-manager/` absent. |
| CV10 | verified-false | `wit/toys/layer-fs.wit` and `wit/toys/git.wit` absent; host impls absent. |
| CV11 | verified-partial | Scrape is grammar-driven/strategy-structured in-core, but no code/git child-pluggable lane shipped yet. |
| CV12 | verified-false | `patina spec list` remains core path; no "spec-manager not available" child gating path. |
| CV13 | verified-false | No `rename` or `reopen` spec subcommands present in core spec CLI. |
| CV14 | verified-false | No mandatory human confirmation gate on `spec complete` / `spec abandon`. |
| CV15 | verified-false | Core doctor command exists (`src/commands/doctor.rs`). |
| CV16 | verified-false | Version command still queries spec readiness (`src/commands/version/internal.rs`). |
| CV17 | verified-false | Core session command module exists (`src/commands/session/*`). |
| CV18 | verified-false | Core lake command exists (`src/commands/lake.rs`). |

## Target State

- Mother is one standalone crate with all runtime logic
- CLI binary is core verbs + thin Mother client
- Children are WASM, installed per-project, resolved by Mother
- Core verbs follow explicit runtime policy when Mother is unavailable (snapshot/degraded vs hard-fail paths are defined per command)
- No daemon stubs, no fallback dance, no "not yet implemented"
- Zero warnings

### Command Runtime Policy (locked for this spec)

| Command surface | Mother unavailable policy | Notes |
|---|---|---|
| `scrape` | `snapshot/degraded` | Core layer+belief strategies must still run locally. |
| `scry` | `snapshot/degraded` | Local search works; cross-project Mother path is additive when available. |
| `assay` | `snapshot/degraded` | Structural local analysis remains available. |
| `context` | `snapshot/degraded` | Local context synthesis remains available. |
| `belief` | `snapshot/degraded` | Belief lifecycle remains core/local. |
| `oxidize` | `snapshot/degraded` | Local index build remains available. |
| `spec` (post-childization) | `mother-required` | Routes to `spec-manager` child. |
| `lake` (post-childization) | `mother-required` | Routes to `lake-manager` child. |
| `doctor` (post-childization) | `mother-required` | Routes to doctor child. |
| `session` (post-childization) | `mother-required` | Session lifecycle owned by session-writer child. |
| `measure` (post-childization) | `mother-required` | Routed through bundled `measure-health` child. |

## Implementation Order

### Phase 0: Reality audit (required before Phase 1)
- Validate and update the CV truth map in this spec with fresh evidence.
- Amend any inaccurate CV text before code changes begin.
- Keep the truth map updated at each phase boundary.
- Lock runtime policy per command (`mother-required living`, `snapshot read-only`, or `hard-fail`) and capture it in DESIGN phase verification logs.
- Record explicit entry/exit checklist for each subsequent phase in DESIGN before implementation.

### Phase 1: Clean core paths
- Remove NEW daemon stub routing (the pre-v1 JSON-lines stubs that return "not yet implemented") from core verb commands: context, measure, spec, lake
- Preserve scry's existing Mother HTTP path (`mother::scry()`) — this is the real cross-project search path that talks to the running Mother HTTP daemon, not a stub
- Delete warning-producing code only after call-site verification (grep + compile proof). Do not pre-assume deadness.
- After cleanup: core verbs call their internal modules directly, no daemon round-trip for local operations

### Phase 2: Finish vocabulary
- Merge src/plugin/ internals into existing src/child/ (src/child/ already exists with runtime re-exports — plugin engine/manifest/linker code moves there)
- Rename Plugin types to Child types

### Phase 3: Consolidate Mother
- Move src/mother/ runtime into mother/ crate (state, events, broker, tasks, graph)
- Move src/commands/mother/ daemon into mother/ crate (server, registry, heartbeat)
- Move src/toys/ host implementations into mother/ crate (these are Mother's toy implementations, not CLI code)
- CLI `patina mother` becomes thin: start/stop/status over the socket
- Mother is one crate, one process, one thing
- Note: mother/src/daemon.rs already has real protocol routing (327 lines) — the actions return placeholder text but the infrastructure is solid. Phase 3 adds real runtime behind the existing routing.

### Phase 4: New toys
- Define toy-layer-fs WIT interface + Mother host implementation
- Define toy-git WIT interface + Mother host implementation
- Add to SDK tiered surface

### Phase 5: Move children out of core
- spec-manager child (with rename, reopen, HITL confirmation)
- doctor child
- lake-manager child
- session becomes session-writer child responsibility (remove core session command)
- measure-health as bundled Mother child
- Rewrite CLI commands as thin Mother→child routes

### Phase 6: Separable scrape strategies
- Harden scrape strategy seam and preserve current behavior.
- Make code/git strategy lanes extraction-ready behind explicit interfaces.
- Childize `scrape-code`/`scrape-git` only after 1:1 parity proof (or explicit user override).
- Scrape orchestrator can discover installed strategy children via Mother once parity gates are satisfied.
- Phase completion does not require immediate childization of code/git lanes; seam hardening + parity evidence is sufficient.

### Phase 7: Project manifests
- Define project manifest format (what children does this project need)
- Mother reads manifest on project connect
- Mother resolves missing children from local inventory
- Future: registry and P2P resolution (architecture allows, not implemented)

### Phase 8: Version cleanup
- Decouple version command from spec system
- Version shows version, period

### Phase 9: Verify
- Full test suite — zero failures, zero warnings
- Core verbs work with Mother stopped
- Child-provided commands work through Mother
- Child-provided commands fail gracefully without Mother
- Project connect resolves children from manifest

## Resolved Decisions

1. Patina core = belief system + knowledge verbs. Runtime policy is explicit: living mode with Mother, plus defined behavior when Mother is unavailable.
2. Mother = standalone daemon. One crate, one process. Ships with bundled children.
3. Children = opt-in WASM extensions. Per-project, resolved by Mother.
4. Spec is a child. Lake is a child. Doctor is a child. Session is a child. Measure is a bundled Mother child.
5. Core scrape strategies (layer, beliefs) stay built-in. Domain strategies (code, git) become children.
6. Belief command stays core — the loop will evolve later but belief is the product.
7. No daemon stubs. Command behavior with/without Mother follows explicit policy rather than implicit fallback filtering.
8. Pre-v1 JSON-lines daemon stubs are removed (they return "not yet implemented" — pure waste). Scry's Mother HTTP path (`mother::scry()`) is preserved — it's real cross-project search functionality, not a stub.
9. Dead code from MCP/interface/pipe removal is deleted. These are formatters and functions that lost all callers. If children need similar formatting later, rebuild from child context — don't carry dead code.
10. Plugin vocabulary fully replaced with child vocabulary.
11. Project manifests declare child needs. Mother resolves.
12. Architecture must not block P2P Mother federation (but doesn't implement it).
13. Child resolution is behind a pluggable interface — local inventory is the first implementation, registry and P2P are future implementations of the same interface.

## Verification

- Commands obey explicit with/without-Mother policy and do not rely on placeholder-filter fallback flows
- `patina spec list` requires Mother + spec-manager child
- `patina lake list` requires Mother + lake-manager child
- `patina measure` requires Mother + measure-health child
- `cargo check -q` produces zero warnings
- All tests pass

Verification quality bar:

- Every passed item above must include runnable command proof in DESIGN phase logs.
- "It should" statements are not accepted evidence.
- Any unresolved contradiction between code and spec keeps related CV unchecked.

## Exit Criteria

See frontmatter (CV1–CV18).

## Build Readiness

Ready when promoted to active. Phases are sequential. Each phase is independently verifiable. Any agent can pick this up, read the beliefs, and execute.
