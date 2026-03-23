# Design: Greenfield Mother + Patina Rebuild

## Why This Design

The active refactor proves we can migrate safely, but migration-safe shape is not always
the same as ideal shape. This design captures the ideal shape explicitly so future work has
a clear north star rather than inheriting accidental boundaries.

## Build Target

Define the architecture we would implement if starting from empty source tree today:

- Patina core as protocol product,
- Mother as standalone runtime daemon,
- children as opt-in extensions,
- toys as least-privilege host capabilities,
- interface runtimes as external guests.

Execution intent for this phase: finish the extraction that recent refactor work began.
This lane is about removing transitional coupling cleanly (with parity gates), not
starting another broad rewrite cycle.

Greenfield done-state for this design:

- Patina and Mother are fully separated by explicit, durable contracts.
- SDK surfaces are coherent for third-party builders of Mother/child/toy systems.
- Extension patterns (custom lakes, custom data blocks/apps, multi-Mother topologies,
  persona-centric workflows) are enabled through contracts, not internal forks.

## Design Principles

1. Beliefs are the product; infrastructure serves belief loops.
2. Mother runtime ownership is singular and explicit.
3. Core verbs are local-first and standalone-capable by policy.
4. Child seams are contracts, not convenience wrappers.
5. Session artifacts are runtime-owned outputs, not ad-hoc CLI state.
6. Verification evidence is required before ownership moves.
7. Third-party buildability is a first-class architecture constraint.
8. Experimentation lanes must compose around Patina without violating core contracts.

## Crate Naming and Publish Policy

This lane treats crate identity as architecture contract, not branding detail.

- `patina-ai` remains the product-facing crate (CLI + core protocol ergonomics).
- `patina-sdk` remains the single public SDK entrypoint for child/toy builders.
- `patina-sdk-core`, `patina-sdk-data`, and `patina-sdk-agent` are tier crates that support
  the SDK surface; they are not additional public SDK brands.
- Mother runtime crate naming should align with Patina namespace when published externally;
  preferred crates.io identity is `patina-mother` (not bare `mother`) for clarity.

Publishing policy for Mother runtime:

1. Keep Mother crate internal-only until runtime boundaries are stable and M1 seam extraction
   parity is proven.
2. Publish as `patina-mother` only when the external runtime contract is intentionally minimal,
   documented, and covered by compatibility tests.
3. Do not publish transitional/internal-only APIs that are expected to churn during migration.

## Work Plan

### Slice 0: Current-state evidence pass (required)

- Build a path-anchored ownership snapshot before adding any new target claims.
- Capture evidence for child boundary, toy grant model, Mother runtime ownership, and remaining CLI seams.
- Record contradictions explicitly; do not hide them behind "future cleanup" language.

Required anchors:

- `src/child/mod.rs:1`
- `src/child/toy_host/mod.rs:1`
- `mother/src/state.rs:70`
- `mother/src/toys.rs:13`
- `src/commands/mother/daemon.rs:670`
- `src/commands/spec/mod.rs:374`

### Slice A: Architecture map (GF1)

- Define ownership matrix for `core`, `mother`, `children`, `sdk`, `wit`.
- Mark each boundary as `permanent contract` or `migration scaffold`.
- Add explicit "current owner -> target owner" for each row.
- Call out contract seams intended for external implementation (third-party Mother/child/toy builders).

### Slice B: Runtime policy matrix (GF2, GF5)

- Build command behavior matrix for Mother on/off.
- Document failure behavior and error message contracts.
- Lock guest-runtime rules (Claude/OpenCode/Gemini).
- Keep control-plane verbs (`spec`, `lake`, `doctor`) under explicit `mother-required` policy.
- Keep core knowledge verbs under explicit `standalone-core` policy.

### Slice C: Data and lifecycle model (GF3, GF4)

- Define canonical data stores and ownership (`events`, projections, sessions).
- Define child lifecycle: install, load, health, invoke, revoke.
- Define toy grants/scopes and enforcement points.
- Ensure schema language matches active child manifest contract (`[needs].toys`, `[needs.scopes]`).
- Define minimum SDK guarantees required for custom lakes/data blocks/apps and persona orchestration.

### Slice D: Migration map and risk model (GF6, GF7)

- Translate greenfield target into bounded migration slices.
- Add parity gates, rollback protocol, and blast-radius notes.
- Add one migration ledger row per ownership move before promoting to active.

## Migration Ledger Template (authoritative)

| Slice | Current owner | Target owner | Parity gates | Rollback trigger | Rollback action | Blast radius |
| --- | --- | --- | --- | --- | --- | --- |
| M1: CLI -> Mother daemon seam extraction | `src/commands/mother/daemon.rs:670` | `mother/src/daemon.rs`, `mother/src/microserver.rs`, `mother/src/socket.rs`, `mother/src/lifecycle.rs` and supporting runtime modules | `cargo check -q`; `cargo test -q`; `patina mother start`; `curl -s --unix-socket ~/.patina/run/serve.sock http://localhost/health`; probe `spec`/`lake`/`doctor` with Mother running and stopped to confirm `mother-required` failures still match contract | Any regression in daemon startup, `/health` response shape, child dispatch behavior, or Mother-required error contract | Revert M1 commits and restore routing/handler path to `src/commands/mother/daemon.rs`; re-run parity gates before retry | `patina mother*`; control-plane verbs (`spec`, `lake`, `doctor`); session-writer + child routing surfaces |
| M2: Mother runner extraction + `patina-mother` Option B path | `src/commands/mother/daemon.rs:152` orchestration shell and env-gated extracted path (`PATINA_MOTHER_EXTRACTED`) | Mother-owned bootstrap/runner API (`mother/src/*`) consumed by Patina CLI as thin adapter; publishable direction for `patina-mother` runtime entrypoint | `cargo check -q`; `cargo test -q`; parity probes from M1 checklist; verify `patina mother start` behavior unchanged; verify no dependency inversion (`mother` must not depend on Patina command modules) | Any behavior drift in start/health/control-plane routing, or bootstrap API requiring Patina internals | Revert M2 runner commits and keep Patina composition shell as temporary adapter; preserve M1-owned runtime modules | daemon startup/orchestration surface; publish policy and runtime API boundary for `patina-mother` |
| M3: SDK contract stabilization (not redesign) | Mixed SDK + legacy compatibility surfaces (`sdk/patina-sdk/src/lib.rs:1`, `sdk/patina-sdk/Cargo.toml:16`) and runtime contract touchpoints (`src/child/internal/tests.rs:317`, `mother/src/toys.rs:13`) | Tiered SDK as canonical external contract (`sdk/patina-sdk-core`, `sdk/patina-sdk-data`, `sdk/patina-sdk-agent`, `sdk/patina-sdk` umbrella) with legacy features treated as migration shims until removed by parity | `cargo check -q`; `cargo test -q`; verify manifest capability schema enforcement stays `[needs].toys` + `[needs.scopes]`; verify existing child loading/runtime behavior remains stable via Mother start + health + child dispatch probes; ensure docs/spec language uses child/kind vocabulary and matches shipped SDK surfaces | Any break in child authoring ergonomics, manifest compatibility, toy grant enforcement, or third-party builder path that currently works | Revert M3 commits affecting SDK public surface/docs; restore prior exported features and compatibility shims; reopen with narrower SDK slice | SDK crates; child authoring docs/templates; manifest parser contracts; toy grant/capability path |

Add one row per additional ownership-moving slice before promoting this spec to active.

## M2a Handshake Contract (project + persona)

M2 starts by locking the Patina -> Mother handshake for single-node now and multi-Mother later.

Required request fields:

- `agent` (interface/runtime caller identity)
- `interface_kind` (opencode/claude/gemini/legacy-cli)
- `project_uid` (resolved from `.patina/uid`)
- `persona_uid` (resolved from request/project binding; optional during transition)
- `requested_session` (optional explicit attach target)

Required response fields:

- `mother_node_id` (placeholder allowed pre-networking; must become stable with iroh)
- `accepted_persona_uid` (echo/resolve result)
- `session_runtime_id`
- `session_file_id`
- `policy_flags` (e.g. `mother-required` surfaces)

Session attach rule (normative):

- Reuse/attach decisions MUST be scoped at minimum by `(project_uid, adapter_name, interface_kind, persona_uid)`.
- If `persona_uid` is provided, mismatched persona sessions MUST NOT be auto-attached.

Persona resolution precedence (normative):

- explicit launch argument (`--persona`) -> project binding (`.patina/persona`) -> no persona scope.

Typed bootstrap config rule (normative):

- Mother startup mode MUST be represented with typed enums (transport/auth/lifecycle), not stringly/boolean combinations in CLI command code.
- Patina `run_server` acts as argument translation into Mother bootstrap config; Mother executes startup orchestration from that config.

## M1 Acceptance Checklist (binary pass/fail)

Run these checks for M1 seam extraction and record outputs in session/spec evidence.

1. Build/test baseline
   - `cargo check -q`
   - `cargo test -q`
2. Mother daemon availability
   - `patina mother start`
   - `curl -s --unix-socket ~/.patina/run/serve.sock http://localhost/health`
   - Expected: HTTP 200 JSON payload with stable health shape.
3. Mother-required control-plane commands while daemon is running
   - `patina spec next`
   - `patina lake list`
   - `patina doctor --json`
   - Expected: commands succeed via Mother child dispatch.
4. Mother-required failure contract while daemon is stopped
   - `patina mother stop`
   - Re-run `patina spec next`, `patina lake list`, `patina doctor --json`
   - Expected: explicit Mother-required failure messaging; no silent fallback.
5. Regression gate
   - No output/contract drift outside known approved message text changes.

M1 is complete only when all five checks pass and evidence is attached.

M1 completion evidence (this session):

- `9e264a1c` refactor: extract Mother HTTP daemon routing core
- `ac2e8506` refactor: move daemon lifecycle helpers into mother crate
- `7b2696a0` refactor: extract daemon child bootstrap composition
- `e4f9a7ab` refactor: isolate daemon loader and builtin dispatch adapters
- `79717da0` refactor: move daemon heartbeat runtime into mother crate

## Seam Classification Table (GF1 enforcement)

| Seam | Classification | Owner | Removal trigger |
| --- | --- | --- | --- |
| CLI command entrypoint -> Mother runtime invocation (`patina mother start`) | permanent contract | `src/commands/mother/mod.rs` + `mother/src/lifecycle.rs` | none |
| CLI-owned daemon internals in `src/commands/mother/daemon.rs` | migration scaffold | M1 slice owner | remove when ownership parity is proven in Mother crate and M1 checklist passes |
| Child capability manifest schema (`[needs].toys`, `[needs.scopes]`) | permanent contract | child manifest parser + SDK docs | none |
| Legacy SDK world compatibility features | migration scaffold | M3 slice owner | remove only after M3 parity gates prove no active dependency |

## SDK Stability Tiers (M3 enforcement)

Use these tiers for all SDK-facing APIs and docs:

- stable: default third-party target; semver-protected surface under `patina-sdk`.
- experimental: opt-in surface with explicit instability warning and migration guidance.
- internal: not documented as external contract; may change without SDK guarantees.

Rules:

1. `patina-sdk` remains the single public SDK brand.
2. Tier crates (`patina-sdk-core`, `patina-sdk-data`, `patina-sdk-agent`) support the SDK and are not separate SDK brands.
3. Any `stable` promotion must include compatibility tests and example consumer proof.

## Publish Gate for `patina-mother`

Mother runtime publication is optional and gated. Do not publish by default.

Publish only when all are true:

1. M1 seam extraction is complete with recorded parity evidence.
2. M2 runner/bootstrap extraction is complete with no behavior drift from `patina mother start` contracts.
3. Runtime API surface is intentionally minimal and documented.
4. Compatibility tests cover core runtime contracts (daemon lifecycle, health, child dispatch, grants/session envelopes).
5. No transitional migration scaffolds are exposed as public API.
6. Versioning/ownership policy is documented in release notes and architecture docs.

## Direct Documentation Targets

- `layer/surface/build/refactor/greenfield-mother-patina-rebuild/SPEC.md`
- `layer/surface/build/refactor/greenfield-mother-patina-rebuild/DESIGN.md`
- Optional follow-on references in architecture docs once reviewed.

## Verification Plan

1. Baseline proof pass: `cargo check -q`, `cargo test -q`.
2. Boundary proof pass: every ownership claim references `path:line` evidence.
3. Runtime policy proof pass: Mother on/off matrix rows have reproducible command probes.
4. Migration proof pass: every ledger slice has parity and rollback checks.
5. Review pass: check consistency with locked beliefs and AGENTS runtime policy.

## Risks and Controls

- Risk: greenfield design drifts into fantasy architecture with no migration path.
  - Control: every target decision needs a migration slice and parity gate.
- Risk: accidental reopening of settled beliefs.
  - Control: conflicts require explicit contradiction evidence and rationale.
- Risk: overfitting to one interface runtime.
  - Control: keep runtime-guest contract provider-neutral by default.

## Open Questions

- Should Mother query orchestration remain a permanent adapter seam to core retrieval,
  or move into a shared domain crate long-term?
- Should child manifests adopt stricter version pinning in project manifests by default?
- What minimum observability surface is required before enforcing child hard-fail policies?

## Build Readiness

- [ ] GF1-GF7 have concrete sections and evidence links.
- [ ] Slice 0 evidence anchors are captured and contradictions are explicit.
- [ ] Runtime policy matrix is command-level, not principle-only.
- [ ] Migration ledger has at least one real row per ownership-moving lane.
- [ ] Migration map has executable parity gates.
- [ ] No contradictions with active refactor truth map remain unresolved.
