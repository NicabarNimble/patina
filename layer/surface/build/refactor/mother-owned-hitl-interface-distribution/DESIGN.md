# Design: refactor: Mother-Owned HITL Interface Distribution

## Why This Design

Preserve launcher and `patina ai <interface>` behavior.
Move interface/skill/session ownership to Mother-managed registries/packages.

## Build Target

Deliver a no-behavior-change ownership migration for three surfaces:

1. **Interfaces** — Mother owns HITL interface registry, metadata, and package distribution.
2. **Skills** — Mother owns skill package roots consumed during projection.
3. **Sessions** — Mother remains explicit lifecycle authority while wrapper workflow stays intact.

User-visible invariants:
- `patina` no-subcommand launcher flow remains identical.
- `patina ai <interface>` remains identical from user perspective.
- prompt label changes to `Available HITL interfaces`.
- PI is available as the 4th HITL interface.

Execution model: one umbrella refactor with three gated phases (registry,
skills, session/path), completed in order.

## Core Value Anchors

- Adapter Pattern: read current code fully before adding/removing boundaries.
- Dependable Rust: preserve public command contracts; isolate internal rewrites.
- Safety Boundaries: path mutations stay within declared managed paths.
- Implementation discipline: read code before write/remove code.
- Git discipline: update with scalpel, not shotgun.

## Resolved Decisions

- **Behavior lock:** this spec changes authority/distribution only.
- **HITL taxonomy:** launcher selection is explicitly HITL-scoped.
- **HITL scope now:** use `hitl` classification only in this phase.
- **Registry authority:** runtime interface lists come from Mother registry, not static arrays.
- **PI onboarding model:** PI arrives via manifest/package path, not compile-time hardcoding.
- **PI proof role:** PI is required proof that registry representation/launch is
  general and not hardcoded to legacy interfaces.
- **Skills authority:** Mother package roots are authoritative for projected skills.
- **Detection contract:** keep current behavior (ordered probe commands, picker filters detected, explicit launch errors if not detected).
- **Bootstrap contract:** no detect-fail auto-bootstrap/retry in this phase.
- **Vendor bootstrap boundary:** keep interface-specific vendor bootstrap metadata.
- **Managed paths boundary:** path mutations must be declared and logged.
- **Tmux boundary:** keep current tmux behavior; represent policy in metadata.
- **Project predicate:** one canonical project test shared by launcher/session flows.
- **Vocabulary stance:** HITL interface distribution is Mother client
  infrastructure, distinct from MCT child/toy composition.
- **Mother scope stance:** Big Mother is accepted for this phase; decomposition
  is a separate follow-up concern.
- **No freshness eviction:** active sessions are not cleared by age.
- **Liveness phase boundary:** no new heartbeat subsystem required in this phase.
- **`src/main.rs` sizing:** acknowledged and deferred to a follow-up refactor spec.

## Architecture Sketch

### A) HITL Manifest Metadata

Mother-managed interface manifests carry launch-time metadata, including a
classification field enabling launcher filtering to HITL interfaces.

Expected minimal metadata contract:
- `name`
- `display`
- `classification` (`hitl` for Claude/Gemini/OpenCode/PI)
- `detect_commands` (ordered)
- `vendor_bootstrap` (optional)
- `managed_paths`
- `tmux_policy`
- `skills.include`

Validation behavior:
- invalid manifest (missing required field/type) is excluded from picker discovery,
- explicit launch to invalid manifest returns field-level validation error,
- invalid manifests do not block loading other valid interfaces.

PI definition for this spec: a HITL interface that uses the same launcher and
`patina ai <interface>` contracts as existing HITL interfaces.

### B) Launcher Discovery Path

`patina` launcher and `patina ai` resolve interfaces from Mother registry.
Temporary static-list fallback is allowed only during migration commits and must
be removed before this spec is marked complete.

Selection text updates from `Available interfaces` to `Available HITL interfaces`.

Detection behavior is unchanged:
- picker path uses detected interfaces only,
- explicit `patina ai <interface>` errors when detect fails.

Bootstrap decision tree:
1. resolve interface metadata from registry,
2. run ordered detect probes,
3. if picker path and detect=false: do not show as selectable,
4. if explicit path and detect=false: hard fail with install guidance,
5. if detect=true: continue launch and project vendor/bootstrap files.

Probe execution contract:
- no shell interpolation,
- inherit process environment and `PATH`,
- per-command timeout: 3000ms,
- success: exit code 0,
- failure classes: timeout / non-zero / executable missing,
- stop on first successful probe.

PI must satisfy this same contract as proof of registry generality.

### B1) Managed Paths + Operation Log

Managed paths are centralized runtime data. Path creation/update/deletion during
projection and cleanup must:
- be constrained to declared `managed_paths`,
- pass through centralized path handling,
- append an operation log entry (interface, runtime_id, operation, path).

Operation log contract:
- location: `.patina/local/interface-ops.jsonl`
- format: one JSON object per mutation with
  `{ts, interface, runtime_id, op, path, result}`
- retention: append-only for this phase
- idempotency: cleanup of missing path records `result="skipped"`
- concurrency: writes serialized with exclusive file lock
- lock behavior: wait up to 1000ms, then fail closed with actionable error

This establishes deterministic cleanup and auditability for future lifecycle work.

### C) Skill Distribution Authority

Projection behavior remains wrapper-compatible, and source roots are Mother-owned
packages (e.g., under `~/.patina/skills/` and Mother-managed interface package
paths). Existing interface-local projection shape remains unchanged for users.

Registry stance:
- small scope,
- interface-specific projection adapters are first-class.

Refined stance:
- small and opinionated for Patina client-infrastructure and MCT-adjacent
  workflows.

### D) Session Lifecycle Authority

No workflow changes to `session-start/update/note/end` wrappers. Session runtime
state remains canonical in Mother store; wrappers keep writing/updating durable
artifacts through current command surfaces.

Canonical project predicate in this phase:
- `.patina/config.toml` exists
- `layer/` directory exists

This predicate must be shared by launcher and session flows.

### E) Session Lifetime Policy (No Freshness Eviction)

Do not evict/clear HITL session state due to age. An active interface may remain
open for days.

This phase keeps cleanup boundaries explicit:
- `session-end` wrapper flow
- `patina ai end`

Stale-pointer recovery in this phase:
- active-runtime predicate is Mother `mother_sessions.status=active` for
  project+interface+runtime_id,
- on launch/start, clear interface pointer when referenced runtime is not active,
- `patina ai end` is idempotent and clears pointers even when process is already dead.

Orphan/liveness automation is out of scope for this spec.

## Commits
1. `refactor(interface): add HITL classification metadata and PI package seed` — introduces taxonomy and PI in registry-owned form.
2. `refactor(launch): read selectable interfaces from Mother registry` — preserves launcher behavior while changing authority.
3. `refactor(ai): preserve patina ai <interface> semantics with registry-backed resolution` — keeps user-facing behavior stable.
4. `refactor(skills): source projection inputs from Mother-owned skill packages` — ownership/distribution cutover with wrapper-compatible projection output.
5. `refactor(paths): centralize managed-path handling and add projection/cleanup operation log` — constrains mutations and enables deterministic cleanup.
6. `refactor(session): harden Mother lifecycle authority wiring and unify project predicate` — clarifies canonical lifecycle ownership without workflow changes.
7. `refactor(session): enforce no-age-eviction policy, explicit cleanup, and stale-pointer reconciliation` — keep long-running sessions valid while authority migrates.
8. `test(registry): PI proof with registry-fixture integration test` — loads PI from registry fixture and confirms no compile-time interface-name additions required.

Phase gates:
- Phase A: commits 1,2,3 complete + PI registry-only launch proof.
- Phase B: commit 4 complete.
- Phase C: commits 5,6,7 complete.

## Direct Code Targets
- `src/commands/launch/internal.rs` — launcher flow, lost-project prompt text, interface selection source.
- `src/commands/ai/surface.rs` — `patina ai <interface>` resolution path and launch orchestration invariants.
- `src/interface/launch.rs` — remove static-list authority for runtime selection/discovery.
- `src/interface/mod.rs` — remove CLI-side static interface factory/catalog wiring in favor of registry-backed adapter resolution.
- `src/interface/internal/bundle.rs` — remove hardcoded bundle authority in favor of registry reads.
- `src/interface/runtime/templates.rs` — package-root ownership for template/skill projection inputs.
- `src/paths.rs` — centralized managed-path definitions/helpers used by projection/cleanup.
- `src/session/internal/live.rs` — explicit Mother lifecycle authority checkpoints remain canonical.
- `src/session/internal/projection.rs` — keep active/last session pointers stable through authority migration.
- `src/main.rs` — wiring updates only; no structural split in this spec.
- `layer/surface/build/refactor/mother-owned-hitl-interface-distribution/SPEC.md` — criteria tracking.

## Verification Plan

1. **Compile**
   - `cargo check --workspace -q`
2. **Launcher in non-project dir**
   - run `patina`
   - verify `Are you lost?` flow is unchanged
   - verify prompt says `Available HITL interfaces`
   - choose interface and confirm direct launch behavior remains
3. **Launcher in existing Patina project**
   - run `patina`
   - verify default interface resolution and launch semantics unchanged
4. **Direct AI command invariants**
   - run `patina ai claude`, `patina ai gemini`, `patina ai opencode`, `patina ai pi`
   - verify expected session/bootstrap/tmux semantics are unchanged
5. **Session workflow invariants**
   - run wrappers `session-start`, `session-update`, `session-note`, `session-end`
   - verify tags/artifacts and active-session pointers still behave the same
6. **Registry authority check**
   - verify runtime interface listing/selection reads Mother registry, not static list
7. **Managed path governance check**
   - verify projection/cleanup touches only declared `managed_paths`
   - verify operation log entries exist for path mutations
8. **Skills authority check**
   - verify projected skills come from Mother-owned package roots and include wrapper-compatible command surfaces
9. **No-age-eviction policy check**
   - keep a HITL interface session open across long duration (or simulated delay)
   - verify no time-based cleanup occurs
10. **Project predicate parity check**
   - assert launcher and session commands use same predicate (`.patina/config.toml` + `layer/`)
11. **PI hard proof check**
   - run required integration test with registry fixtures containing PI metadata
   - verify PI registration/launch works from fixture-loaded registry data
   - verify no compile-time interface-name additions are required for PI landing

## Build Readiness

Medium-High. Most required primitives already exist (Mother session store,
launch orchestration, projection mechanisms). Main effort is replacing static
interface authority with registry authority while preserving existing UX.
