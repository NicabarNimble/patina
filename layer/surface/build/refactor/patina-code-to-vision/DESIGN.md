# Design: Make the codebase reflect the architecture vision

> Read SPEC.md first. Read the beliefs. Then read this.

## Why This Design

The architecture is clear. The code isn't there yet. This is the mechanical plan.

## Build Target

One Mother daemon. Core verbs standalone in CLI. Children opt-in via WASM. Zero stubs, zero dead code, zero confusion about what lives where.

## Execution Slices (risk control)

This is one architecture spec, executed in bounded slices to avoid half-complete lockups:

1. Foundation slice: CV3, CV4, CV6 (+ warning pressure toward CV5).
2. Runtime consolidation slice: CV1, CV2, CV10.
3. Child-surface slice: CV7, CV8, CV9, CV12-CV18.
4. Final parity slice: CV5, CV11 and full verification matrix.

If a slice blocks on newly discovered prerequisites, SPEC/DESIGN must be patched before code continues.

## Agent Execution Rules (no drift)

0. Anchor every phase in `layer/core/` values before changes:
   - `spec-driven-design.md`, `dependable-rust.md`, `safety-boundaries.md`, `unix-philosophy.md`.
1. Execute phases sequentially. No parallel phase work.
2. Before touching code, verify phase entry conditions and record them. Read code before write code.
3. After code changes, run phase proof commands and record key lines.
4. Update SPEC CV truth map before declaring phase complete.
5. Never check a CV without direct proof.
6. If reality disagrees with design, patch SPEC/DESIGN first, then continue.
7. Git updates with scalpel, not shotgun (small, phase-bounded commits only).

## Resolved Decisions

1. Mother is one crate — merge the three tangled modules.
2. Core verbs call internals directly — no daemon routing.
3. Pre-v1 JSON-lines daemon stubs are removed (pure waste). Scry's Mother HTTP path preserved (real cross-project search).
4. Dead code from MCP/interface/pipe removal is deleted — these are formatters that lost all callers. Rebuild from child context if needed later.
5. Plugin → child vocabulary everywhere.
6. New toys (layer-fs, git) before children that need them.
7. Spec, lake, doctor, session become children; measure remains core.
8. Project manifests declare needs, Mother resolves.
9. Child resolution behind pluggable interface — local inventory first, registry/P2P later.
10. Bundled runtime-child loading is explicit: compiled-in native registration (`secrets`) + first-party WASM inventory under `~/.patina/children/` (e.g., `session-writer`).

## Phase 0: Reality Audit (required)

Before refactor commits, generate and maintain:
- CV1-CV18 truth map embedded in SPEC.md
- phase verification logs embedded in this DESIGN.md

Also lock and record per-command runtime policy:

- mother-required (living mode)
- snapshot/degraded behavior when Mother unavailable
- hard-fail behavior where required

No phase starts until its prerequisite claims are marked `verified`.

### Phase Verification Log Template

For each phase completion, append:

- Date:
- Commits:
- Commands run:
  - `<command>`
- Observed output (key lines):
  - `<line>`
- CVs affected:
  - CVx, CVy
- Truth-map updates:
  - `<what changed in SPEC.md CV truth map>`

### Phase Entry/Exit Checklist Template

- Entry:
  - [ ] prerequisite CV statuses verified
  - [ ] runtime policy impact reviewed
  - [ ] target files listed
- Exit:
  - [ ] proof commands pass
  - [ ] key output lines captured
  - [ ] SPEC CV truth map updated
  - [ ] no unresolved contradiction remains

### Phase 0 Baseline Report

- Date: 2026-03-22
- Command: `cargo check`
- Observed output: `patina-ai (bin "patina") generated 40 warnings`

### Phase 0 Verification Report

- Date: 2026-03-22
- Commands run:
  - `rg "try_daemon_|not yet implemented" src/commands/{context.rs,lake.rs,spec/mod.rs,measure/mod.rs,scry/internal/routing.rs}`
  - `rg "\\b(rename|reopen)\\b" src/commands/spec --glob "*.rs"`
  - `rg "(complete|abandon).*confirm|confirm.*(complete|abandon)|--yes|Are you sure" src/commands/spec --glob "*.rs"`
  - `rg "ready spec|get_ready_spec_ids|spec complete" src/commands/version/internal.rs`
  - `cargo check`
  - `test -e children/spec-manager/child.toml || echo missing`
  - `patina mother status`
  - `test -e wit/toys/layer-fs.wit || echo missing`
  - `test -e wit/toys/git.wit || echo missing`
  - `test -e src/commands/doctor.rs || echo missing`
  - `test -e src/commands/lake.rs || echo missing`
  - `test -e src/commands/session/mod.rs || echo missing`
- Observed key lines:
  - extracted daemon probe/fallback paths still present in `context`, `measure`, `spec`, `lake`, `scry`
  - `children/spec-manager/` not found
  - `session-writer` not present in loaded child status output
  - `wit/toys/layer-fs.wit` and `wit/toys/git.wit` not found
  - `rename`/`reopen` not present in spec command implementation
  - version command still queries ready specs
  - core `doctor`, `session`, and `lake` commands still present
  - warning count remains 40
- Outcome:
  - CV truth map updated in SPEC.
  - Phase 0 complete; Phase 1 may begin.

## Phase 1: Clean core paths (CV3, CV4, CV5)

Entry checklist:

- [ ] CV3/CV4/CV5 are `verified-false` in SPEC truth map.
- [ ] daemon-probe callsites enumerated from current code.

### Scope rules

**Remove** extracted-daemon probe wrappers (`try_daemon_*`) from core command success paths where CV3/CV4 require direct local behavior.

**Preserve** any genuinely functional Mother integration paths that are not scaffold placeholders.

**Warning cleanup is evidence-driven**: remove only symbols proven dead by call-site grep + compile/test checks. Do not pre-delete from assumption lists.

### Commits

1. `core: remove extracted-daemon probe from context`
2. `core: remove extracted-daemon probes from measure/spec/lake`
3. `core: remove extracted-daemon probe path from scry where placeholder-filtering is used`
4. `core: remove daemon client module when unused`
5. `core: warning cleanup pass from verified dead symbols`

### Verification
- `cargo check -q`
- Warning gate for Phase 1: no new warnings introduced by Phase 1 edits; carry-forward warnings outside Phase 1 scope are explicitly logged with owner phase.
- CV5 remains unchecked until Phase 9 where `cargo check -q` must be fully warning-free.
- Add `resources/scripts/check-core-verb-policy.sh` (checked into repo).
  - Deterministic proof path: `--mode off --isolated` (fresh temporary `PATINA_HOME`).
  - Integration proof path: `--mode on` (live daemon/runtime).
  - Integration preflight: capture `patina mother status`; only if status is `stopped`, clear stale runtime artifacts under `~/.patina/run/`; then start Mother in a dedicated shell before recording proof output.
- Core-verb policy matrix (Mother stopped):
  - `patina scrape <phase-proof-args>` succeeds in snapshot/degraded mode and does not route through extracted-daemon stubs
  - `patina scry <phase-proof-args>` succeeds in snapshot/degraded mode; no placeholder-filter fallback path
  - `patina assay <phase-proof-args>` succeeds in snapshot/degraded mode
  - `patina context <phase-proof-args>` succeeds in snapshot/degraded mode
  - `patina belief <phase-proof-args>` succeeds in snapshot/degraded mode
  - `patina oxidize <phase-proof-args>` succeeds in snapshot/degraded mode
- Cross-check (Mother running): `patina scry <phase-proof-args>` retains additive Mother path for cross-project search
- All tests pass

Exit checklist:

- [ ] no `try_daemon_*` on canonical success paths for targeted commands
- [ ] no `contains("not yet implemented")` placeholder-filter routing in targeted commands
- [ ] no new warnings introduced by Phase 1 edits; remaining warnings logged and mapped to future phases

### Phase 1 Verification Report (2026-03-22)

- Commits:
  - `f12dc81a` — core: remove extracted-daemon probe from context
  - `d56c15ee` — core: remove extracted-daemon probes from measure/spec/lake
  - `5dbd03d4` — core: remove extracted-daemon probe path from scry
  - `e8e1077f` — core: remove unused daemon client module
- Commands run:
  - `cargo check -q`
  - `grep "try_daemon_|not yet implemented" src/commands/{context.rs,measure/mod.rs,spec/mod.rs,lake.rs,scry/internal/routing.rs}`
  - `resources/scripts/check-core-verb-policy.sh --mode off --isolated`
  - `patina context --topic architecture --no-tmux`
  - `patina measure --no-tmux`
  - `patina spec next`
  - `patina lake list`
- Observed key lines:
  - target `grep` returned no matches for `try_daemon_`/`not yet implemented` in targeted files
  - deterministic Mother-off policy script passed all six core verbs
  - `patina spec next` returned `RECOMMENDED: patina-code-to-vision`
  - `cargo check -q` succeeded with existing warning debt (no new hard failures)
- CVs affected:
  - CV3, CV4 (probe/fallback removal on targeted core paths)
  - CV5 (still unchecked; warning debt remains and is deferred per gate policy)

## Phase 2: Finish vocabulary (CV6)

**Note:** src/child/ already exists (runtime re-exports from mother crate). src/plugin/ has the engine/manifest/linker code. This is a merge, not a rename.

Manifest bridge plan (explicit):

1. Keep `child.toml` filename canonical throughout.
2. Treat `[plugin]` section parsing as a temporary bridge only; document bridge status in-code and in SDK docs.
3. Add first-party manifest migration commits to canonical child vocabulary (`kind` + child terminology) before bridge removal.
4. Add a guard that blocks new plugin-era wording in docs/templates once migration lands.
5. Remove parser/type alias bridge only after parity proof across all first-party children.

1. `refactor: merge src/plugin/ internals into src/child/`
2. `refactor: rename Plugin types to Child types with bridge aliases`

### Verification
- cargo build succeeds
- grep -r "PluginManifest\|PluginWorld\|PluginEngine" src/ — zero matches outside bridge
- ls src/plugin/ — should not exist
- ls crates/patina-pipe/ — should not exist
- ls src/mcp/ — should not exist

### Phase 2 Verification Report (2026-03-22)

- Commits:
  - `ba0b3848` — refactor: introduce child engine surface and migrate runtime callers
  - `cf9d586e` — refactor: migrate CLI surfaces to child engine vocabulary
  - `737878cc` — refactor: make plugin module a child-vocabulary bridge
  - `4a73cd17` — refactor: accept [child] section and migrate child manifests
  - `754885c6` — refactor: canonicalize child kind and role types
  - `c5b39516` — refactor: canonicalize child manifest and provides types
  - `bffb0ed6` — refactor: migrate scaffold world type to child kind
  - `f4230706` — refactor: migrate internal engines to child manifest vocabulary
  - `60cc29c5` — refactor: tighten child-first bridge exports
  - `26affc95` — test: migrate internal plugin tests to child vocabulary
  - `920be97b` — refactor: move runtime engine module from plugin to child
  - `3abbac5a` — refactor: remove plugin engine alias from child internals
  - `7d1dca22` — refactor: scaffold child.toml manifests with child vocabulary
  - `1a53d91a` — refactor: prefer child manifests in runtime discovery paths
  - `9b29a89b` — refactor: remove legacy plugin manifest bridge
  - `27b2b7cd` — test: align child manifest error expectations
  - `00c9a3f3` — refactor: make child naming canonical in command and path surfaces
- Commands run:
  - `cargo check -q`
  - `cargo build -q`
  - `rg "PluginManifest|PluginWorld|PluginEngine|PluginRole|PluginProvides" src/`
  - `test -d src/plugin && echo exists || echo missing`
  - `rg "plugin\.toml|\[plugin\]" src/child src/main.rs src/lib.rs src/commands/setup/grammars.rs sdk/patina-sdk/src`
  - `cargo test -q`
  - `cargo test -q manifest_valid_minimal`
- Observed key lines:
  - build/compile pass with existing warning debt
  - `src/plugin` directory removed (`missing`)
  - runtime/CLI engine code lives under `src/child/` and uses `ChildManifest`/`ChildKind`/`ChildEngine`
  - no plugin-era type identifiers found in `src/`
  - no plugin-era manifest vocabulary found in runtime/SDK surfaces checked above
  - grammars and plugin assets migrated from `plugin.toml` to `child.toml` with `[child]` + `kind`
  - full suite parity proof passed (`cargo test -q`: 397 passed, 0 failed, 2 ignored)

Exit checklist:

- [x] canonical child vocabulary is dominant in runtime code
- [x] legacy names only remain in intentional compatibility bridges
- [x] child manifests use `child.toml` + `kind`; transitional parser support for legacy keys is explicitly documented until bridge removal
- [x] 1:1 parity proof captured before bridge removal
- [x] temporary bridge removed after parity proof (unless user explicitly approves exception)

## Phase 3: Consolidate Mother (CV1, CV2, CV7)

This is the big one. Three tangled modules become one crate.

Single-path ownership invariant (required at each step):

- For each moved responsibility (state, events, broker, registry, daemon server, toy hosts), exactly one active runtime path owns behavior after each commit.
- No dual active implementations are allowed beyond a short re-export handoff window in the same phase step.
- Caller-switch + parity proof must happen before deleting the old path.
- If parity fails, do not proceed to next responsibility slice.

Dependency extraction rule (required):

- If a move target depends on `patina`-crate internals (`beliefs`, `retrieval`, `session`, `repo`, `child::engine`, host toy internals), do not force-move the module directly.
- First introduce explicit adapter contracts at the boundary, then move the orchestrator against those contracts.
- Treat this as Phase 3 work, not scope creep into unrelated feature phases.

Current blocked dependencies (2026-03-22 reality check):

- `graph` orchestration depends on `crate::beliefs`, `crate::commands::repo::internal::Registry`, `patina::session::SessionManager`.
- daemon query path depends on `crate::retrieval::QueryEngine`.
- toy host implementations depend on child runtime capability surfaces and DuckDB-linked code paths.
- CLI thinning and shell deletion remain gated on the above extractions.

### Phase 3 rollback protocol (required)

- Each Phase 3 substep is a separate commit (no bundled multi-step commits).
- After each substep commit, run parity/build proofs before starting the next substep.
- If parity fails on a substep: stop Phase 3, revert only that substep commit, and do not continue.
- Log failure in the Phase 3 verification log with: failing proof command, root cause, and next action.
- Advancing to the next substep with broken parity is forbidden.

1. `mother: move state store into mother crate` — Move src/mother/state.rs → mother/src/state.rs. Re-export from src/mother/mod.rs temporarily.

2. `mother: move event streams and tasks into mother crate` — Move src/mother/events.rs, tasks.rs, checkpoint.rs → mother/src/.

3. `mother: move broker into mother crate` — Move src/mother/broker/ → mother/src/broker/.

4. `mother: move graph into mother crate` — Move src/commands/mother/graph.rs → mother/src/graph.rs.

5. `mother: move daemon server into mother crate` — Move src/commands/mother/daemon.rs, microserver.rs, registry.rs → mother/src/. This makes the mother/ crate the complete daemon. Note: mother/src/daemon.rs already has 327 lines of real protocol routing — the actions return placeholder text but the infrastructure is solid. This commit adds real runtime behind the existing routing.

6. `mother: move toy host implementations into mother crate` — Move src/toys/ (github.rs, session.rs, lake.rs, connector.rs, http.rs, ingress.rs, query.rs, catalog.rs) → mother/src/toys/. Toys are Mother's responsibility — she implements the host side of WIT interfaces that children consume.

7. `mother: make CLI mother commands thin` — src/commands/mother/mod.rs becomes: start (spawn daemon process), stop (signal), status (query socket). All runtime code gone from CLI.

8. `mother: delete src/mother/ and src/toys/ modules` — After all moves, delete the empty shells. CLI depends on mother crate for types only.

Phase 3a/3b sequencing clarification:

- 3a Structural relocation: state/events/tasks/checkpoint/broker/registry/microserver/secrets/daemon transport shell.
- 3b Functional extraction: graph/query/toys move behind adapter contracts, then CLI-thin and shell deletion.
- Mark 3a complete only when relocated code is canonical ownership; mark 3b complete only when adapter-backed parity proof passes.

### Verification
- cargo build -p mother — succeeds, contains all runtime
- cargo build -p patina-ai — succeeds, no Mother runtime code
- cargo test -p mother — all tests pass
- patina mother start — daemon starts from mother crate
- patina mother status — bundled runtime children are visible (`secrets` compiled-in + `session-writer` from first-party WASM inventory)
- For each adapter extraction slice: document interface, caller-switch proof, and parity result before deleting prior path.

### Phase 3 Progress Report (2026-03-22, in-progress)

- Structural relocation commits complete so far:
  - `9c97cb73` — mother: move state store into mother crate
  - `d456a7a6` — mother: move event streams and tasks into mother crate
  - `6250ce07` — mother: move broker into mother crate
  - `badc700e` — mother: move daemon server into mother crate
- Adapter extraction commits in progress:
  - `1a860445` — spec: align phase 3 to adapter-first consolidation
  - `8b4daba0` — refactor: extract mother daemon scry backend adapter
  - `ab425dc8` — refactor: extract mother graph and scry dependency adapters
  - `ed8ab975` — refactor: add graph registry and session adapter traits
  - `80491bcc` — refactor: route child toy calls through toy-host adapter
  - `7a0f1258` — refactor: move mother stop/status lifecycle logic into mother crate
  - `ecbf2695` — refactor: move mother socket lifecycle utilities into mother crate
  - `5faebbc8` — refactor: relocate toy host modules under child runtime
  - `697eb842` — refactor: move event toy access out of mother shell module
- Active blockers captured as adapter work (not direct moves):
  - graph orchestration dependencies (`beliefs`, registry/session wiring)
  - daemon query dependency (`retrieval::QueryEngine`)
  - toy host dependency wiring (child capability + DuckDB-linked surfaces)
- Next required path:
  - introduce adapter contracts per blocked dependency
  - switch callers to contracts with parity proof
  - only then delete shell modules and finalize CLI-thin state

Exit checklist:

- [ ] runtime ownership map shows Mother logic centralized in `mother/`
- [ ] CLI Mother commands are transport/client wrappers only
- [ ] graph/query/toy orchestration paths are switched to explicit adapter contracts before any shell deletion

## Phase 4: New toys (CV10)

1. `wit: define toy-layer-fs interface and host implementation` — wit/toys/layer-fs.wit: read-file, write-file, list-dir, delete-file, move-path, exists. Scoped to layer/ directory. Host impl in mother/src/toys/layer_fs.rs.

2. `wit: define toy-git interface and host implementation` — wit/toys/git.wit: create-tag, delete-tag, tag-exists, commit, log-oneline, diff-stat. Host impl in mother/src/toys/git.rs.

3. `sdk: add toy-layer-fs and toy-git to tiered surface` — Backends and ZST wrappers. Feature gates.

### Verification
- WIT sync check passes
- Mother host implementations have tests

## Phase 5: Move children out of core (CV9, CV12, CV13, CV14, CV15, CV17, CV18)

Entry checklist (pre-Phase-5 WASM readiness gate):

- [ ] `cargo check -q -p patina-ai-child-session-writer -p patina-ai-child-ducklake -p patina-ai-child-belief-verifier`
- [ ] `cargo build -q -p patina-ai-child-session-writer --target wasm32-wasip2`
- [ ] Mother started and child visibility proven via `patina mother status` (at least one child listed)
- [ ] One real child path through Mother proven and logged (stable gate command): `curl -s --unix-socket ~/.patina/run/serve.sock http://localhost/child/secrets/health`
- [ ] Key output lines for all gate proofs are recorded in Phase 5 verification log before implementation starts

1. `child: scaffold spec-manager` — children/spec-manager/child.toml (needs: log, state, layer-fs, git). handle() stub.

2. `child: implement spec-manager operations` — Move logic from src/commands/spec/internal/. All CRUD, lifecycle, queries, packets. Add rename and reopen. Add HITL confirmation for complete/abandon.

3. `cli: rewrite patina spec as thin Mother client` — Route to spec-manager child. Delete src/commands/spec/internal/. Graceful error without Mother.

4. `child: move doctor to child` — Move src/commands/doctor.rs logic into children/doctor/. Thin CLI wrapper.

5. `child: move lake to child` — Move src/commands/lake.rs logic into children/lake-manager/. Thin CLI wrapper.

6. `core: remove session as core command` — Session artifacts are written by session-writer child (already exists). Remove src/commands/session/ from core CLI surface. Session operations go through Mother.

7. `cli: rewrite thin wrappers for doctor, lake, session` — Each becomes: connect to Mother, route to child, format output. Error gracefully without Mother.

### Verification
- Pre-Phase-5 WASM readiness gate proof is present in phase log (all entry checklist items above)
- patina spec list — works with Mother + spec-manager
- patina spec list — fails gracefully without Mother
- patina measure — works with Mother stopped (core primitive path)
- patina doctor — works with Mother + doctor child
- patina lake list — works with Mother + lake-manager
- Core verbs still work without Mother

Exit checklist:

- [ ] core command removals match CV set
- [ ] child replacements provide parity (or explicit policy-compliant failure)
- [ ] spec lifecycle features `rename` + `reopen` + HITL are proven

## Phase 6: Separable scrape strategies (CV11)

1. `scrape: harden strategy registration interface` — Preserve current grammar abstraction and existing outputs.
2. `scrape: make non-core strategy lanes extraction-ready` — explicit seam, no behavior regression.
3. `child: extract scrape strategies only after parity proof` — childization is gated, not assumed.
4. `scrape: orchestrator discovers strategy children` — enabled once extracted lanes meet parity.

### Verification
- `patina scrape` baseline behavior matches pre-phase outputs (1:1 parity proof)
- layer/belief scrape remains available without Mother
- extraction-ready seams exist for non-core scrape strategy lanes without forcing immediate childization
- if childization is executed in this phase, parity proof is attached before switching default lane ownership

Exit condition note:

- Phase 6 may close without childizing scrape strategy lanes if seam hardening and parity proofs are complete.

## Phase 7: Project manifests (CV8)

Project child-needs manifest path is fixed for this spec: `.patina/manifest.toml`.

Schema boundary note (to avoid `needs` confusion):

- Child manifest schema (`child.toml`) uses `[needs].toys` with optional `[needs.scopes]`.
- Project manifest schema (`.patina/manifest.toml`) uses `[needs].children`.
- These are different layers and are intentionally both named `needs`.

Schema for this spec:

```toml
[project]
schema = 1

[needs]
children = ["spec-manager", "doctor", "lake-manager", "session-writer"]
```

1. `mother: define project manifest format` — enforce `.patina/manifest.toml` schema and validation errors.

2. `mother: resolve children on project connect` — Mother reads manifest, checks inventory, reports missing.

3. `mother: child installation from local inventory` — Mother can install available children for a project.

### Verification
- Project connects, Mother reads manifest, reports status
- Missing children identified clearly

## Phase 8: Version cleanup (CV16)

1. `cli: decouple version from spec system` — patina version shows version. Period. No spec status query.

### Verification
- patina version works without Mother, without patina.db

## Phase 9: Verify (all CVs)

1. Full test suite — zero failures, zero warnings.
2. Core verbs with Mother stopped.
3. Child commands through Mother.
4. Child commands fail gracefully without Mother.
5. Project connect resolves children.

## Direct Code Targets

### Phase 1 — daemon routing + dead code
- src/commands/context.rs, measure/mod.rs, scry/internal/routing.rs, lake.rs, spec/mod.rs
- src/mother/daemon_client.rs (delete)
- candidate dead-code areas (`src/commands/assay/internal/`, `src/commands/scry/`, `src/schema/`, `src/retrieval/`) are deletion-eligible only after per-symbol caller proof + compile/test proof

### Phase 2 — vocabulary
- src/plugin/ → src/child/
- All Plugin* type references

### Phase 3 — Mother consolidation
- src/mother/*.rs → mother/src/
- src/commands/mother/{daemon,microserver,registry,graph,secrets}.rs → mother/src/
- src/commands/mother/mod.rs → thin start/stop/status

### Phase 4 — new toys
- wit/toys/layer-fs.wit, git.wit (new)
- mother/src/toys/ (new host impls)

### Phase 5 — children
- children/spec-manager/ (new)
- children/doctor/ (new)
- children/lake-manager/ (new)
- src/commands/spec/internal/ (delete after migration)
- src/commands/doctor.rs (delete after migration)
- src/commands/lake.rs (delete after migration)
- src/commands/session/ (delete — session-writer child handles this)

### Phase 6 — scrape strategies
- src/commands/scrape/ (seam hardening in-place)
- optional: strategy lane extraction targets under `children/` after parity proof
- src/commands/scrape/mod.rs — strategy discovery via Mother

### Phase 7 — project manifests
- .patina/manifest.toml (new format)
- mother/src/ — manifest reading + child resolution

### Phase 8 — version
- src/commands/version/mod.rs and src/commands/version/internal.rs — remove spec query

## Locked Decisions

- Git scrape lane is non-core and extraction-ready in this spec; childization is parity-gated (not mandatory in Phase 6).
- Bundled runtime-child loading is defined in this spec as two explicit modes only: compiled-in native registration + first-party WASM inventory.
- `plugins/` root is a legacy transitional surface tracked under CV6 vocabulary migration; this refactor does not require behavioral changes there before CV6 execution.
- Project child-needs manifest is a dedicated project artifact at `.patina/manifest.toml` with `[project].schema` + `[needs].children` contract.
- Secrets-cache representation is out of scope for this refactor unless it blocks CV satisfaction.

## Build Readiness

Ready when promoted to active. Phases are sequential — complete each before starting the next. Any agent can execute from this spec + beliefs.

This design is authoritative for execution order and proof quality. If an agent cannot produce proof for a claimed completion, the phase is not complete.
