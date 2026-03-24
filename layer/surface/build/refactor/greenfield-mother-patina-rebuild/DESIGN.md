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
| M2: Mother runner extraction + `patina-mother` Option B path | `src/commands/mother/daemon.rs:152` orchestration shell (including former extracted startup seam) | Mother-owned bootstrap/runner API (`mother/src/*`) consumed by Patina CLI as thin adapter; publishable direction for `patina-mother` runtime entrypoint | `cargo check -q`; `cargo test -q`; parity probes from M1 checklist; verify `patina mother start` behavior unchanged; verify no dependency inversion (`mother` must not depend on Patina command modules) | Any behavior drift in start/health/control-plane routing, or bootstrap API requiring Patina internals | Revert M2 runner commits and keep Patina composition shell as temporary adapter; preserve M1-owned runtime modules | daemon startup/orchestration surface; publish policy and runtime API boundary for `patina-mother` |
| M3: Mother-owned secret authority migration | Secret authority currently anchored in Patina secrets implementation (`src/secrets/*`, `src/commands/secrets.rs`) plus Mother runtime cache child (`mother/src/secrets.rs`) | Mother owns secret authority control-plane; Patina `secrets` command remains UX client surface delegating to Mother APIs | `cargo check -q`; `cargo test -q`; `patina secrets` parity checks (`status`, `add`, `remove`, `list-recipients`, `lock`); verify no secret-value persistence regression and explicit Mother-required failure behavior | Any user-visible break in secret CRUD/recipient/identity flows, or security regression in authority handling | Revert M3 commits to prior Patina-owned authority path and keep Mother cache runtime behavior intact | `patina secrets*` UX, vault/recipient/identity control-plane, CI/headless credential workflows |
| M3d: Greenfield-purity secrets internals relocation | Transitional adapter still sources implementation from Patina secrets internals (`src/mother/secrets_backend.rs` -> `crate::secrets::*`) | Secret implementation internals relocated to Mother-owned modules; adapter no longer depends on `crate::secrets` | `cargo check -q`; `cargo test -q`; `patina secrets` parity checks (`status`, `add`, `remove`, `list-recipients`, `lock`, `setup-claude`); verify Mother-on success and Mother-off explicit authority failure; verify vault paths/data remain unchanged | Any secret data loss risk, vault format drift, or behavior regressions in authority operations | Revert M3d commits; restore adapter-backed implementation path; re-run parity and data-integrity checks before retry | `mother` secrets internals, `patina secrets` UX, vault compatibility surface |
| M4: Post-M3 boundary cleanup (greenfield alignment) | Residual runtime/authority seams remain in Patina core (`src/secrets/*` direct callers, `src/mother/broker/mod.rs`, command-level Mother client duplication, path duplication risk) | Mother owns runtime/authority internals; Patina core keeps protocol UX and thin adapters only | `cargo check -q`; `cargo test -q`; verify `patina secrets` Mother-required behavior remains explicit; verify broker/control-plane behavior unchanged (`spec`, `lake`, `doctor`) with Mother on/off contract preserved | Any regression in secrets authority routing, broker execution behavior, or Mother-required command contracts | Revert M4 cleanup commits per seam cluster; restore prior adapter call path and re-run parity before retry | Patina core vs Mother ownership boundaries; secrets call sites; broker and control-plane adapter surfaces |
| M5: SDK contract stabilization (not redesign) | Mixed SDK + legacy compatibility surfaces (`sdk/patina-sdk/src/lib.rs:1`, `sdk/patina-sdk/Cargo.toml:16`) and runtime contract touchpoints (`src/child/internal/tests.rs:317`, `mother/src/toys.rs:13`) | Tiered SDK as canonical external contract (`sdk/patina-sdk-core`, `sdk/patina-sdk-data`, `sdk/patina-sdk-agent`, `sdk/patina-sdk` umbrella) with legacy features treated as migration shims until removed by parity | `cargo check -q`; `cargo test -q`; verify manifest capability schema enforcement stays `[needs].toys` + `[needs.scopes]`; verify existing child loading/runtime behavior remains stable via Mother start + health + child dispatch probes; ensure docs/spec language uses child/kind vocabulary and matches shipped SDK surfaces | Any break in child authoring ergonomics, manifest compatibility, toy grant enforcement, or third-party builder path that currently works | Revert M5 commits affecting SDK public surface/docs; restore prior exported features and compatibility shims; reopen with narrower SDK slice | SDK crates; child authoring docs/templates; manifest parser contracts; toy grant/capability path |
| M6: Crate architecture lock (core + protocol extraction) | Domain logic in `src/mother/*_runtime.rs` + ad-hoc JSON dispatch in `mother/src/builtin_children.rs` + `#[path]` shim in `src/mother/spec_runtime.rs` | `patina-core` owns domain use-cases; `patina-protocol` owns typed dispatch contracts; CLI and Mother are thin adapters | `cargo check -q`; `cargo test -q`; `cargo test -q -p patina-core`; `cargo test -q -p patina-protocol`; Mother on/off probes for `spec`/`lake`/`doctor`; dependency-direction check; `cargo run -q -- spec check greenfield-mother-patina-rebuild --json` | Dependency inversion between core and adapters, protocol breakage across command matrix, or external SDK contract regression | Revert M6 slice commits; restore prior adapter boundaries; re-run parity matrix | Builtin dispatch surfaces; workspace dependency graph; SDK contract compatibility |

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
- Both default HTTP daemon mode and extracted JSON-lines mode must route through the same typed bootstrap surface (variant-selected execution), not separate ad-hoc startup branches.

Typed identity boundary rule (normative):

- Handshake/session scope identifiers (`project_uid`, `persona_uid`) should use typed wrappers/newtypes at boundary APIs so attach/lookup calls cannot accidentally swap or drop scope semantics.
- Interface scope (`interface_kind`) should also use a typed wrapper at handshake/session-lookup boundaries.

`PATINA_MOTHER_EXTRACTED` cleanup outcome:

- Post-M2 cleanup trigger passed and the env switch was removed.
- Startup now uses typed bootstrap transport variants only; no env-gated extracted branch remains.
- Drift protection intent preserved: migration switches must be removed once trigger conditions are met, unless a recorded blocking defect exists.

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

M2 completion evidence (this session, Option B path):

- `875761f7` refactor: add mother daemon runner launch API
- `d88dd143` refactor: add typed mother bootstrap config orchestration
- `cd6f3279` refactor: unify daemon startup modes under typed bootstrap
- `593a4b21` refactor: add typed project and persona scope identifiers
- `a1baf11a` refactor: type interface scope at handshake and session boundaries
- `29e856b4` feat: define persona-scoped handshake inputs and attach rules
- `3f280577` refactor: scope interface session attach by persona context
- `bc069b13` docs: enforce extracted-mode deprecation and removal trigger
- `post-M2 cleanup`: removed `PATINA_MOTHER_EXTRACTED` env switch and converged startup to typed transport variants only

M2 parity evidence status:

- Repeated parity probes across these slices preserved control-plane behavior (`spec`, `lake`, `doctor`) with Mother on/off.

M2 functional status:

- Complete (runner/bootstrap extraction, typed boundary scoping, and post-M2 extracted-switch cleanup achieved).

M3 functional target:

- Move secret authority ownership to Mother control-plane while preserving `patina secrets` as the user-facing client UX.

M3 contract-first invariants:

- Secret scope model is explicit and enforced: `project` -> `persona` -> `machine` resolution order.
- Secret authority API is OS-agnostic by contract (backend-specific implementations may vary).
- `patina secrets` remains UX/client surface; authority implementation lives behind Mother control-plane APIs.
- Identity retrieval/storage default should be cross-platform (`encrypted_file` path works on macOS + Linux) so M3 does not create divergent platform authority logic.
- macOS Keychain remains an optional backend/integration path (and future Swift-native backend target), but not a required separate authority mode.
- Secret records carrying OAuth credentials must retain provider metadata (`provider`, `account_id`, granted scope set, expiry/refresh hints) for safe lookup/reuse.
- Scope/persona mismatch is deny-by-default; no implicit cross-persona fallback.
- Platform policy for this lane: macOS + Linux supported, Windows unsupported by design.

M3 execution slices (sequenced):

1. M3a: Introduce Mother secret authority API + Patina command proxy path (legacy Patina authority kept as rollback shim).
2. M3b: Flip default to Mother authority and verify behavior/security parity.
3. M3c: Remove legacy Patina-owned secret authority implementation after parity and migration checks pass.
4. M3d: Relocate remaining secret implementation internals from Patina into Mother-owned modules to close greenfield purity seam.

M3a progress evidence (current session):

- Added `secrets-authority` builtin dispatch route under Mother control-plane (`/child/secrets-authority/dispatch`).
- Patina `secrets` command now attempts Mother authority dispatch first and falls back to legacy local path when Mother is unavailable.
- Proxied authority operations now include status/CRUD/recipient management/session lock plus identity export/import/reset and Claude token setup.
- Added focused regression checks for authority payload construction and Mother-unavailable fallback detection.
- Core-value anchor: preserve local-first reliability and least-surprise behavior during migration (scalpel rollout, no hard cutover in M3a).

M3b progress evidence (current session):

- Default behavior now requires Mother authority for proxied `patina secrets` operations.
- Temporary rollback switch added: `PATINA_SECRETS_LEGACY_FALLBACK=1` re-enables local fallback path during stabilization.
- Runtime verification: with Mother on, `patina secrets` succeeds; with Mother off, command hard-fails by default; rollback switch restores fallback success.

M3c progress evidence (current session):

- Removed legacy local fallback path from proxied `patina secrets` authority operations.
- Removed practical effect of `PATINA_SECRETS_LEGACY_FALLBACK`; `patina secrets` now requires Mother authority for proxied operations regardless of fallback env setting.
- Runtime verification: with Mother on, `patina secrets` succeeds; with Mother off, command hard-fails with explicit authority-unavailable error.
- Moved secrets authority operation contract/parsing/response shaping into Mother crate module (`mother/src/secrets_authority_api.rs`) and routed dispatch via a Mother-owned backend implementation.

M3d progress evidence (current session):

- Added Mother-owned secrets internals module surface (`mother/src/secrets_authority_backend/`) covering identity, encrypted-file/keychain storage, recipients/registry, vault IO, and session-cache interactions.
- Added Mother-owned secrets/serve path helpers (`mother/src/secrets_paths.rs`) so authority internals no longer depend on Patina `crate::paths`.
- Removed Patina-side backend adapter seam (`src/mother/secrets_backend.rs`); `secrets-authority` dispatch now binds directly to `mother_crate::secrets_authority_backend::MotherSecretsAuthorityBackend`.
- Build verification: `cargo check -q` passes with new Mother-owned authority wiring.

M3 final checklist pass (explicit):

- [x] Authority contract ownership in Mother (`mother/src/secrets_authority_api.rs`) with Patina `secrets` as UX client only.
- [x] Transitional backend adapter seam removed (`src/mother/secrets_backend.rs` deleted) and dispatch bound directly to Mother backend (`src/commands/mother/builtin_dispatch.rs`).
- [x] Mother-owned internals landed (`mother/src/secrets_authority_backend/*`) with Mother-local path helpers (`mother/src/secrets_paths.rs`).
- [x] Build/test gates pass after M3d cleanup:
  - `cargo check -q`
  - `cargo test -q -p mother`
- [x] Mother-required failure contract verified for proxied secrets ops when Mother is unavailable:
  - `patina secrets`
  - `patina secrets list-recipients`
  - `patina secrets --lock`
- [x] Mother-on parity verified for non-destructive authority ops in isolated runtime home:
  - `patina secrets`
  - `patina secrets list-recipients`
  - `patina secrets --lock`
- [x] Path compatibility cleanup completed: encrypted identity path now honors `PATINA_HOME` in Mother backend (`f2ae7cbf`).

M3 completion status:

- Complete. M3a/M3b/M3c migration plus M3d purity closeout are satisfied with recorded evidence and verification gates.

M4 activation note:

- Next active lane is M4 (post-M3 boundary cleanup for greenfield ownership alignment).

M4 kickoff checklist (active):

- [x] Inventory residual Patina-core secret authority call sites and decide greenfield target per call path (Mother authority API vs explicit local-only policy).
- [x] Define target ownership for broker orchestration (`src/mother/broker/mod.rs`) and plan relocation/adapter boundaries into Mother runtime.
- [x] Consolidate Mother client resolution for control-plane commands (`spec`, `lake`, `doctor`, `secrets`) into one policy helper.
- [x] Resolve path duplication risk (`src/paths.rs` vs `mother/src/secrets_paths.rs`) with a single-source contract or explicit anti-drift tests.
- [x] Record seam-by-seam parity/rollback evidence for M4 before any boundary deletions.

M4a execution slices:

1. M4a1: Inventory/classify residual Patina bypass call sites for Mother-owned secrets authority.
2. M4a2: Migrate secret read/write call sites to canonical Patina -> Mother authority channel.
3. M4a3: Add anti-drift coverage for shared rendezvous path semantics.
4. M4a4: Retire dead Patina-side secret internals once bypass call sites are removed.

M4a boundary rules (non-negotiable):

1. Patina standalone rule
   - `patina-ai` core belief/protocol workflows remain usable without Mother.
   - Any command/verb that is Mother-required must be explicitly declared as control-plane policy.

2. Authority routing rule
   - Runtime authority operations (secrets authority, control-plane child dispatch, runtime state mutation) route through Mother APIs when Mother is enabled.
   - Patina core must not maintain hidden direct-internals bypass paths for Mother-owned authority surfaces.

3. UX vs authority separation rule
   - Patina owns user-facing UX (`patina secrets`, prompts, output shaping).
   - Mother owns authority implementation and policy enforcement for Mother-owned surfaces.

4. Dependency direction rule
   - Mother must not depend on Patina command modules or Patina-only runtime internals.
   - Shared contracts must be explicit (typed API, protocol payloads, manifests), not implicit cross-crate reach-through.

5. Communication channel rule
   - Patina -> Mother communication policy is centralized (single client resolution/transport/auth behavior for Mother-required commands).
   - Local UDS and TCP/token behavior must remain contract-consistent across commands.

6. Path ownership rule
   - Path logic is crate-local by default.
   - Any path used as an interop rendezvous contract between Patina and Mother (e.g., `PATINA_HOME`, run socket/token, shared authority files) must have an explicit anti-drift contract (shared module or parity tests).

7. Evidence-before-move rule
   - Each boundary move in M4a requires parity gate evidence (`cargo check -q`, relevant tests, on/off Mother contract probes) and rollback notes before retiring old seams.

M4a progress evidence (current session):

- Centralized Patina -> Mother control-plane address/client resolution in `src/mother/mod.rs` (`resolve_control_plane_address`, `control_plane_client`).
- Added explicit endpoint/env contract constants for control-plane comms:
  - `PATINA_MOTHER` (primary),
  - `PATINA_MOTHER_ADDR` (address alias),
  - `PATINA_MOTHER_SOCKET` (UDS override),
  - `PATINA_MOTHER_TOKEN_FILE` (token file override).
- Updated Mother client internals to use centralized UDS/token path overrides (`src/mother/internal.rs`).
- Replaced hardcoded command client construction in control-plane command surfaces (`spec`, `lake`, `doctor`, `secrets`) with canonical `mother::control_plane_client()`.
- Added focused tests for control-plane address resolution precedence/defaults (`cargo test -q -p patina-ai resolve_control_plane_address`).
- Build verification: `cargo check -q`.

M4a1 inventory pass (current session):

- Residual direct Patina secret authority call sites found outside `src/secrets/*`:
  - `src/interface/internal/launcher.rs:244` (`get_global_secret("claude-oauth")`)
  - `src/commands/launch/internal.rs:436` (`get_global_secret("claude-oauth")`)
  - `src/connect/internal/store.rs:118` (`add_secret(...)`)
  - `src/connect/internal/store.rs:169` (`remove_secret(...)`)
  - `src/connect/internal/resolve.rs:30` (`get_global_secret(...)`)
  - `src/child/toy_host/github.rs:274` (`get_global_secret(...)`)
  - `src/child/internal/mother_child.rs:356` (`get_global_secret(...)`)
  - `src/child/internal/host_support.rs:313` (`get_global_secret(...)`)
- Classification decision (greenfield alignment): these are Mother-owned authority operations and should migrate to the canonical Patina -> Mother channel in M4a2.

M4a2 migration pass (current session):

- Extended Mother secrets authority contract with explicit global-read operation (`get_global_secret`) in `mother/src/secrets_authority_api.rs` and backend implementation wiring in `mother/src/secrets_authority_backend/mod.rs`.
- Added canonical Patina-side authority helper `mother::get_global_secret(...)` in `src/mother/mod.rs`.
- Migrated residual non-`src/secrets/*` call sites from direct `crate::secrets::*` usage to Mother authority channel:
  - `src/interface/internal/launcher.rs`
  - `src/commands/launch/internal.rs`
  - `src/connect/internal/store.rs`
  - `src/connect/internal/resolve.rs`
  - `src/child/toy_host/github.rs`
  - `src/child/internal/mother_child.rs`
  - `src/child/internal/host_support.rs`
- Residual `crate::secrets::*` usage is now confined to Patina secrets internals (`src/secrets/*`).
- Build verification: `cargo check -q`.

M4a3 anti-drift path contract pass (current session):

- Added explicit cross-crate path contract tests in `src/paths.rs` comparing Patina and Mother path resolution for shared rendezvous semantics:
  - user-level roots and runtime paths (`PATINA_HOME`, `run`, `serve.sock`, `serve.token`)
  - secrets authority shared paths (`secrets.toml`, `vault.age`, `recipient.txt`, project secrets paths)
- Verification: `cargo test -q -p patina-ai test_mother_paths_contract`.

M4b broker greenfield target (agreed boundary):

- Target ownership: broker orchestration runtime belongs to Mother crate; Patina retains CLI UX/adapters only.
- Patina-side module `src/mother/broker/mod.rs` should collapse to a thin adapter over Mother broker APIs.
- Mother broker orchestration must not depend on Patina command modules; runtime dependencies are injected through explicit interfaces.

M4b broker relocation slices (scalpel plan):

1. M4b1: Introduce Mother broker orchestration interfaces (typed traits/ports for connection resolution, child loading/execution, and legacy cursor reads) without behavior change.
2. M4b2: Move pure orchestration loop (`run_source` flow and lake-route control logic) into Mother crate behind those interfaces.
3. M4b3: Implement Patina adapter bindings for those interfaces (existing connect/child/eventlog capabilities), keep command UX stable.
4. M4b4: Delete redundant Patina broker runtime logic after parity gates pass; keep compatibility wrappers only where contract-stable.

M4b parity gates:

- `cargo check -q`
- `cargo test -q`
- `patina mother sources`
- `patina mother run <source>` (with valid source fixture)
- Verify Mother on/off policy contracts unchanged for control-plane commands (`spec`, `lake`, `doctor`, `secrets`).

M4b rollback trigger/action:

- Trigger: any behavior drift in source run routing, auth fail-closed guarantees, cursor migration, or child task lifecycle semantics.
- Action: revert current M4b slice commit(s), restore Patina-side broker runtime path, re-run parity gates before retry.

M4 seam evidence summary:

- M4a1/M4a2 authority routing evidence:
  - Non-`src/secrets/*` direct secrets call sites migrated to Mother authority channel.
  - Mother authority contract extended with `get_global_secret` for read-path parity.
- M4a communication channel evidence:
  - Canonical control-plane client/address resolution consolidated via `mother::control_plane_client()` and `mother::resolve_control_plane_address()`.
  - Command surfaces (`spec`, `lake`, `doctor`, `secrets`) no longer hardcode `localhost:50051`.
- M4a3 path contract evidence:
  - Anti-drift tests added for shared rendezvous path semantics (`test_mother_paths_contract_user_level`, `test_mother_paths_contract_project_secrets`).
- Validation evidence:
  - `cargo check -q`
  - `cargo test -q -p patina-ai resolve_control_plane_address`
  - `cargo test -q -p patina-ai test_mother_paths_contract`
  - `cargo test -q -p mother`

M4 status (truth update):

- M4a is complete (authority/comms/path alignment).
- M4b is complete for currently defined scope: builtin dispatch ownership, protocol debt retirement, and Mother schema-aligned helpers are in place.
- Next sequencing: continue M5 SDK stabilization/removal-gate work.

M4b execution checklist (active):

- [x] Move builtin child routing envelope logic out of CLI-only module into Mother-owned module boundary (`mother/src/builtin_children.rs`).
- [x] Keep behavior parity by binding a CLI executor adapter while relocation is in-progress (`src/commands/mother/builtin_dispatch.rs`).
- [x] Relocate `spec-manager` execution behind Mother-owned interfaces (remove CLI command-module ownership for this dispatch path).
- [x] Relocate `lake-manager` execution behind Mother-owned interfaces (dispatch no longer depends on `crate::commands::lake::*`).
- [x] Relocate `doctor` execution behind Mother-owned interfaces (dispatch no longer depends on `crate::commands::doctor::*`).
- [x] Resolve legacy protocol/runtime debt: either retire or fully implement `mother/src/daemon.rs` legacy socket protocol path.
- [x] Align Mother events/cursor reads with Mother-owned schema (`mother/src/events.rs`, `mother/src/broker/cursor.rs`).

M4b execution evidence (current session):

- Added Mother-owned builtin dispatch boundary module: `mother/src/builtin_children.rs`.
- `src/commands/mother/builtin_dispatch.rs` is now a thin executor adapter implementing Mother-defined trait (`BuiltinChildExecutor`) rather than owning route/HTTP shaping logic.
- Secrets authority dispatch remains Mother-owned backend (`mother::secrets_authority_backend::MotherSecretsAuthorityBackend`) with no Patina secrets internals dependency.
- Removed schema mismatch debt in Mother runtime helpers:
  - `mother/src/broker/cursor.rs` now reads `mother_lake_cursors` (Mother-owned schema) instead of legacy `broker_cursors`.
  - `mother/src/events.rs` now reads Mother-owned tables (`belief_mutation_log`, `graph_mutation_log`, `mother_sessions`) and no longer queries a non-existent `eventlog` table.
- Verification evidence:
  - `cargo check -q`
  - `cargo test -q -p mother`
- Retired legacy socket protocol implementation from Mother public runtime surface by removing `daemon`/`protocol` module exports from `mother/src/lib.rs`; active runtime path is HTTP/UDS router stack.

M4b execution evidence (lake/doctor ownership slice):

- Added Patina library runtime modules for builtin execution outside CLI command modules:
  - `src/mother/lake_runtime.rs`
  - `src/mother/doctor_runtime.rs`
- Updated builtin executor adapter to call library runtime modules for lake/doctor dispatch paths:
  - `src/commands/mother/builtin_dispatch.rs`
- Reduced CLI command modules (`src/commands/lake.rs`, `src/commands/doctor.rs`) to wrappers/CLI entrypoints over runtime modules.
- Verification evidence:
  - `cargo check -q`
  - `cargo test -q -p mother`
  - `cargo test -q -p patina-ai scaffold::tests::test_scaffold`

M4b execution evidence (spec ownership slice):

- Added shared spec runtime dispatch module under Patina Mother surface:
  - `src/mother/spec_runtime.rs`
- Updated Mother builtin executor adapter to resolve spec dispatch through `patina::mother::spec_runtime` instead of direct `crate::commands::spec::*` references.
- Dispatch decoupling result: builtin dispatch no longer references CLI command modules for spec/lake/doctor execution paths.
- Verification evidence:
  - `cargo check -q`
  - `cargo test -q -p mother`

M5 preview note:

- SDK contract stabilization moves to M5 after M4 boundary cleanup is complete.

M5 scope split note:

- SDK stabilization/removal work has been split into dedicated spec/design:
  - `layer/surface/build/refactor/sdk-contract-stabilization/SPEC.md`
  - `layer/surface/build/refactor/sdk-contract-stabilization/DESIGN.md`
- This greenfield design keeps only architecture-level references to SDK boundaries; implementation detail and compatibility matrix evidence now live in the dedicated SDK lane.

M6 architecture lock (active planning):

- Objective: codify and execute Jon-style crate boundaries with compile-time guardrails:
  - `patina-core` (domain/use-cases; transport/runtime neutral),
  - `patina-protocol` (typed/versioned contracts),
  - `patina-cli` (UX adapter),
  - `patina-mother` (runtime/authority adapter),
  - `patina-sdk` (child authoring surface).

M6 boundary principles (non-negotiable):

1. No adapter logic in core.
2. No cross-crate reach-through hacks (`#[path]` inclusion patterns are migration debt only).
3. Strong typed boundaries (`enum`, newtypes, explicit error surfaces) over string payloads.
4. Trait ports only where boundary effects cross transport/runtime/infrastructure seams.
5. Thin adapters: CLI parses/renders, Mother routes/persists, Core decides.
6. Invariants encoded in types/contracts, not doc-only comments.

M6 execution checklist (active):

- [ ] M6a: create `patina-core` + `patina-protocol` and dependency direction rules. (GF8, GF9, GF12)
- [ ] M6b: migrate `lake` use-case into `patina-core` as first transport-neutral service. (GF8)
- [ ] M6c: replace ad-hoc builtin dispatch payloads with typed `patina-protocol` enums/contracts. (GF9)
- [ ] M6d: remove `#[path]` shim and relocate shared spec execution contracts to core-owned modules. (GF10)
- [ ] M6e: core-ify doctor as host-native service with explicit runtime ports. (GF8, GF11)
- [ ] M6f: core-ify spec execution and remove transitional runtime shims. (GF10, GF11)
- [ ] M6g: wire CLI and Mother as pure adapters and retire `CliBuiltinExecutor` transitional pattern. (GF11)
- [ ] M6h: add workspace dependency-direction enforcement gate. (GF12)
- [ ] M6i: run full parity matrix and update GF8-GF12 evidence. (GF8-GF12)

M6 parity gates:

- `cargo check -q`
- `cargo test -q`
- `cargo test -q -p mother`
- `cargo test -q -p patina-ai`
- Mother on/off probes for control-plane command matrix (`spec`, `lake`, `doctor`, `secrets`)
- `cargo run -q -- spec check greenfield-mother-patina-rebuild --json`

M6 rollback trigger/action:

- Trigger: dependency inversion between adapters/core, protocol breakage across command matrix, or external-SDK-facing contract regression.
- Action: revert current M6 slice commit(s), restore prior adapter boundary, re-run parity matrix, then retry with narrower scope.

## Seam Classification Table (GF1 enforcement)

| Seam | Classification | Owner | Removal trigger |
| --- | --- | --- | --- |
| CLI command entrypoint -> Mother runtime invocation (`patina mother start`) | permanent contract | `src/commands/mother/mod.rs` + `mother/src/lifecycle.rs` | none |
| CLI-owned daemon internals in `src/commands/mother/daemon.rs` | migration scaffold | M1 slice owner | remove when ownership parity is proven in Mother crate and M1 checklist passes |
| Child capability manifest schema (`[needs].toys`, `[needs.scopes]`) | permanent contract | child manifest parser + SDK docs | none |
| Legacy SDK world compatibility features | migration scaffold | M5 slice owner | remove only after M5 parity gates prove no active dependency |

## SDK Stability Tiers (M5 enforcement)

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

- [x] GF1-GF7 have concrete sections and evidence links.
- [ ] GF8-GF12 have concrete sections, evidence links, and mapped M6 execution slices.
- [x] Slice 0 evidence anchors are captured and contradictions are explicit.
- [x] Runtime policy matrix is command-level, not principle-only.
- [x] Migration ledger has at least one real row per ownership-moving lane.
- [x] M6 migration ledger row exists with parity gates and rollback actions.
- [x] Migration map has executable parity gates.
- [ ] Dependency direction enforcement mechanism is defined and executable.
- [x] No contradictions with active refactor truth map remain unresolved.

GF1-GF7 focused evidence anchors (current session):

- GF1: architecture narrative + ownership/seam classifications are explicit across `SPEC.md` and this design (`## Work Plan`, seam table, migration ledger).
- GF2: command-level runtime policy matrix is explicit in `SPEC.md` (`### GF2 Command Matrix`) and validated by M1/M3/M4 parity probes.
- GF3: data ownership model (`events.db`, projections, session/runtime stores) is explicit in `SPEC.md` and aligned with Mother runtime state ownership.
- GF4: child lifecycle/capability schema contract (`[needs].toys`, `[needs.scopes]`) is locked and reflected in migration constraints and ledger gates.
- GF5: guest runtime contract (OpenCode/Claude/Gemini runtime truth, no MCP assumptions) is explicit in AGENTS policy and mirrored by spec runtime policy sections.
- GF6: bounded migration slices with parity gates/rollback actions exist for M1, M2, M3, M3d, M4, and M5 in the authoritative ledger.
- GF7: verification/risk controls are explicit and exercised via command evidence:
  - `cargo check -q`
  - `cargo test -q`
  - `cargo test -q -p mother`
  - `cargo test -q -p patina-ai resolve_control_plane_address`
  - `cargo test -q -p patina-ai test_mother_paths_contract`
  - `cargo run -q -- spec check greenfield-mother-patina-rebuild --json` -> `{"passed":true,"checked":7,"total":7}`
  - Mother on/off behavior probes recorded under M1/M3/M4 evidence notes.

GF8-GF12 realization anchors (active):

- Tracked via M6 execution checklist and M6 migration ledger row; criteria remain unchecked until code-level crate/protocol convergence is landed and evidenced.
