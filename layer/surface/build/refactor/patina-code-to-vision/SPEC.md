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
    text: Mother is a standalone daemon in the mother/ crate — Mother-owned runtime infrastructure (state, broker infrastructure, registry, events, tasks, lifecycle, socket, protocol) is centralized there, not split across three locations
    checked: false
  - id: CV2
    text: CLI binary has zero Mother infrastructure runtime code — it talks to Mother over Unix socket or runs core verbs standalone; thin adapter bridges to core product domains are explicit and allowed
    checked: false
  - id: CV3
    text: Core verbs (scrape, scry, assay, context, belief, measure, oxidize) have an explicit, command-by-command Mother-unavailable policy documented in this spec and verified by command tests (no implicit placeholder-filter fallback behavior)
    checked: false
  - id: CV4
    text: Pre-v1 extracted-daemon probe routing is removed from canonical core command paths (`context`, `measure`, `spec`, `lake`, `scry`) — no `try_daemon_*` probes or `contains("not yet implemented")` filtering in those paths
    checked: false
  - id: CV5
    text: '"cargo check -q" produces zero warnings'
    checked: false
  - id: CV6
    text: Vocabulary migration completes with 1:1 parity and bridge removal — runtime code uses child vocabulary (`ChildManifest`, `ChildKind`, `ChildEngine`), `src/plugin/` is removed, and any temporary compatibility bridge is deleted only after parity proof (unless user explicitly approves exception)
    checked: false
  - id: CV7
    text: Mother startup guarantees bundled runtime children are loaded and visible in health/status output when daemon boots successfully (`secrets` compiled-in + `session-writer` first-party WASM inventory)
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
    text: Scrape strategy boundary is explicit and enforceable — layer/beliefs remain core, and non-core scrape strategy lanes are extraction-ready and independently pluggable without breaking current core scrape behavior. Child extraction happens only after 1:1 parity proof
    checked: false
  - id: CV12
    text: '"patina spec list" without Mother returns clear "spec-manager not available" error'
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
- Spec, lake, session, and doctor are woven into core as if they're fundamental
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
- Core CLI: belief, scrape, scry, assay, context, measure, oxidize — standalone, no daemon
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
- `patina measure` — core measurement and reporting

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

Mother has two explicit bundled runtime-child loader modes (no hidden third mode):
- Compiled-in native child registration (always available): `secrets`
- First-party WASM inventory under `~/.patina/children/` (installed by Patina): `session-writer`

Mother resolves **project children** on connect:
- Project connects with a manifest of needs
- Mother checks local inventory
- Missing children resolved from registry (future: other Mothers via P2P)
- Children are WASM files — portable, sandboxed

### Children (opt-in WASM extensions)

**Workflow children:**
- `spec-manager` — spec lifecycle (create, promote, complete, rename, reopen)
- `doctor` — project diagnostics and health checks

**Strategy children (examples):**
- scrape strategy children (domain/lane-specific)
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
- No net-new user-facing features outside migration parity and required safety UX for moved child surfaces (rename/reopen + human-confirmed complete/abandon)

## Current State (verified)

- Mother runtime is split across `mother/`, `src/mother/`, and `src/commands/mother/`.
- Core verb commands still run extracted daemon-first probes (`try_daemon_*`) with fallback filtering on placeholder responses.
- `spec`, `lake`, `doctor`, `session`, and `measure` remain core CLI command surfaces.
- Vocabulary bridge is partial: canonical child manifests are in place, but `src/plugin/` and `src/child/` coexist.
- `cargo check` reports 40 warnings.
- Scrape code path is already grammar-driven and multi-language-capable; extraction work must preserve grammar abstraction.

### CV Truth Map (Phase 0 baseline)

Refreshed: 2026-03-22

Status keys:

- `verified-false` = code disproves criterion today
- `verified-partial` = some pieces exist, criterion not yet satisfied
- `verified-true` = criterion appears satisfied
- `unverified` = explicit proof still needed

Evidence format rules for this table:

- Structure claims use `path:line` anchors.
- Absence claims use explicit command evidence with observed key output.

| CV | Status | Evidence |
|---|---|---|
| CV1 | verified-partial | Mother runtime infrastructure is centralized in `mother/src/` with shell cleanup complete (`state/events/tasks/broker data/registry/lifecycle/socket`), while adapter-backed orchestrators remain explicitly listed in DESIGN Phase 3 verification (`src/mother/broker/mod.rs`, `src/commands/mother/graph.rs`, `src/commands/mother/daemon.rs`). |
| CV2 | verified-partial | CLI start/stop/status is thin lifecycle transport (`src/commands/mother/mod.rs` calling `mother_crate::lifecycle`/`mother_crate::socket`); explicit adapter bridges remain for core-domain orchestration boundaries. |
| CV3 | verified-true | Runtime policy is explicitly documented in this spec and command-tested with `resources/scripts/check-core-verb-policy.sh --mode off --isolated` (covers `scrape`, `scry`, `assay`, `context`, `belief`, `measure`, `oxidize`). |
| CV4 | verified-true | Command proof: `grep "try_daemon_|not yet implemented" src/commands/{context.rs,measure/mod.rs,spec/mod.rs,lake.rs,scry/internal/routing.rs}` => no matches (2026-03-22). |
| CV5 | verified-false | Command proof: `cargo check` output contains `patina-ai (bin "patina") generated 40 warnings` (2026-03-22 baseline). |
| CV6 | verified-true | Runtime code is child-vocabulary canonical (`src/child/`); command proof: `rg "PluginManifest|PluginWorld|PluginEngine|PluginRole|PluginProvides" src/` => no matches; `test -d src/plugin && echo exists || echo missing` => `missing`; `rg "plugin\.toml|\[plugin\]" src/child src/main.rs src/lib.rs src/commands/setup/grammars.rs sdk/patina-sdk/src` => no matches. |
| CV7 | verified-false | `patina mother status` shows loaded children `ducklake` and `secrets` only; `session-writer` is not loaded/visible in daemon status output (2026-03-22 baseline). |
| CV8 | verified-false | Command proof: `test -e .patina/manifest.toml || echo missing` => `missing`; no project child-needs manifest contract is present in-tree (2026-03-22 baseline). |
| CV9 | verified-true | `patina spec` is a Mother-routed child surface (`spec-manager`) via `commands::spec::execute` + daemon child dispatch; CLI no longer executes spec lifecycle operations directly. |
| CV10 | verified-true | Command proof: `test -e wit/toys/layer-fs.wit && test -e wit/toys/git.wit` => success; host implementations exist at `mother/src/toys/layer_fs.rs:1` and `mother/src/toys/git.rs:1`; proof command `cargo test -q -p mother` passes. |
| CV11 | verified-partial | Scrape is in-core strategy-structured (`src/commands/scrape/mod.rs:1`, `src/commands/scrape/code/extract_v2.rs:1`), but child seam/final extraction contract is not yet implemented. |
| CV12 | verified-true | `target/debug/patina spec list --json` succeeds with Mother running and `target/debug/patina spec list` fails clearly when Mother is stopped (`spec-manager unavailable via Mother ...`). |
| CV13 | verified-true | `spec rename` and `spec reopen` are implemented in spec mutations (`src/commands/spec/internal/mutations.rs`) and wired into `SpecCommands` child dispatch surface. |
| CV14 | verified-true | HITL confirmation is enforced in `commands::spec::execute` for `complete` and `abandon` (interactive `[y/N]` prompt unless `--json`). |
| CV15 | verified-true | `patina doctor` is a Mother child route (`doctor`) and fails clearly without Mother (`doctor child unavailable via Mother ...`). |
| CV16 | verified-true | Version command no longer queries spec state; `src/commands/version/internal.rs` outputs version/strategy/components only, and `cargo run -q -- version` works without Mother and without `patina.db` lookups. |
| CV17 | verified-true | Core CLI no longer exposes `session` verb (`src/main.rs` has no `Commands::Session`); session artifacts are handled by agent/session systems rather than a core user command. |
| CV18 | verified-true | `patina lake` is a Mother child route (`lake-manager`) and fails clearly without Mother (`lake-manager unavailable via Mother ...`). |

## Target State

- Mother is one standalone crate with all runtime logic
- CLI binary is core verbs + thin Mother client
- Children are WASM, installed per-project, resolved by Mother
- Core verbs follow explicit runtime policy when Mother is unavailable (snapshot/degraded vs hard-fail paths are defined per command)
- No daemon stubs, no fallback dance, no "not yet implemented"
- Zero warnings

### Command Runtime Policy (locked for this spec)

Baseline protocol execution means only core verbs (`scrape`, `scry`, `assay`, `context`, `belief`, `measure`, `oxidize`).
Mother-required policy applies only to child-provided command surfaces after childization.

| Command surface | Mother unavailable policy | Notes |
|---|---|---|
| `scrape` | `snapshot/degraded` | Core layer+belief strategies must still run locally. |
| `scry` | `snapshot/degraded` | Local search works; cross-project Mother path is additive when available. |
| `assay` | `snapshot/degraded` | Structural local analysis remains available. |
| `context` | `snapshot/degraded` | Local context synthesis remains available. |
| `belief` | `snapshot/degraded` | Belief lifecycle remains core/local. |
| `measure` | `snapshot/degraded` | Measure remains a core primitive; local metrics/reporting path remains available without Mother. |
| `oxidize` | `snapshot/degraded` | Local index build remains available. |
| `spec` (post-childization) | `mother-required` | Routes to `spec-manager` child. |
| `lake` (post-childization) | `mother-required` | Routes to `lake-manager` child. |
| `doctor` (post-childization) | `mother-required` | Routes to doctor child. |
| `session` (post-childization) | `mother-required` | Session lifecycle owned by session-writer child. |

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
- 3a (structural relocation): move src/mother/ runtime infrastructure into mother/ crate (state, events, broker, tasks, registry, transport shell)
- 3b (functional extraction): for graph/query/toy host orchestration, extract adapter contracts first, then switch callers and ownership
- Do not force-move orchestrators that still depend on patina-internal domains; patch through adapters before physical relocation
- Adapter-backed transitional orchestrator ownership is allowed at Phase 3 completion only when explicitly listed in DESIGN progress logs with parity proof; hidden split ownership is not allowed
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
- Rewrite CLI commands as thin Mother→child routes

### Phase 6: Separable scrape strategies
- Harden scrape strategy seam and preserve current behavior.
- Make non-core scrape strategy lanes extraction-ready behind explicit interfaces.
- Childize non-core scrape strategies only after 1:1 parity proof.
- Scrape orchestrator can discover installed strategy children via Mother once parity gates are satisfied.
- Phase completion does not require immediate childization of scrape strategy lanes; seam hardening + parity evidence is sufficient.

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
2. Mother = standalone daemon. One crate, one process. Bundled runtime-child loading uses explicit modes.
3. Children = opt-in WASM extensions. Per-project, resolved by Mother.
4. Spec is a child. Lake is a child. Doctor is a child. Session is a child. Measure remains core.
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
- `patina measure` remains available without Mother (core behavior)
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
