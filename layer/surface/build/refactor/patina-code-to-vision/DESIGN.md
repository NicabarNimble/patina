# Design: Make the codebase reflect the architecture vision

> Read SPEC.md first. Read the beliefs. Then read this.

## Why This Design

The architecture is clear. The code isn't there yet. This is the mechanical plan.

## Build Target

One Mother daemon. Core verbs standalone in CLI. Children opt-in via WASM. Zero stubs, zero dead code, zero confusion about what lives where.

## Agent Execution Rules (no drift)

1. Execute phases sequentially. No parallel phase work.
2. Before touching code, verify phase entry conditions and record them.
3. After code changes, run phase proof commands and record key lines.
4. Update SPEC CV truth map before declaring phase complete.
5. Never check a CV without direct proof.
6. If reality disagrees with design, patch SPEC/DESIGN first, then continue.

## Resolved Decisions

1. Mother is one crate — merge the three tangled modules.
2. Core verbs call internals directly — no daemon routing.
3. Pre-v1 JSON-lines daemon stubs are removed (pure waste). Scry's Mother HTTP path preserved (real cross-project search).
4. Dead code from MCP/interface/pipe removal is deleted — these are formatters that lost all callers. Rebuild from child context if needed later.
5. Plugin → child vocabulary everywhere.
6. New toys (layer-fs, git) before children that need them.
7. Spec, lake, doctor, session, measure — all become children.
8. Project manifests declare needs, Mother resolves.
9. Child resolution behind pluggable interface — local inventory first, registry/P2P later.

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
- cargo check -q — zero warnings after commit 5
- `patina context` works with daemon stopped — calls internal directly
- `patina scry` works with daemon stopped — local search works; cross-project still routes through Mother HTTP when available
- All tests pass

Exit checklist:

- [ ] no `try_daemon_*` on canonical success paths for targeted commands
- [ ] no `contains("not yet implemented")` placeholder-filter routing in targeted commands
- [ ] warning count materially reduced toward CV5 target

## Phase 2: Finish vocabulary (CV6)

**Note:** src/child/ already exists (runtime re-exports from mother crate). src/plugin/ has the engine/manifest/linker code. This is a merge, not a rename.

1. `refactor: merge src/plugin/ internals into src/child/`
2. `refactor: rename Plugin types to Child types with bridge aliases`

### Verification
- cargo build succeeds
- grep -r "PluginManifest\|PluginWorld\|PluginEngine" src/ — zero matches outside bridge
- ls src/plugin/ — should not exist
- ls crates/patina-pipe/ — should not exist
- ls src/mcp/ — should not exist

Exit checklist:

- [ ] canonical child vocabulary is dominant in runtime code
- [ ] legacy names only remain in intentional compatibility bridges

## Phase 3: Consolidate Mother (CV1, CV2)

This is the big one. Three tangled modules become one crate.

7. `mother: move state store into mother crate` — Move src/mother/state.rs → mother/src/state.rs. Re-export from src/mother/mod.rs temporarily.

8. `mother: move event streams and tasks into mother crate` — Move src/mother/events.rs, tasks.rs, checkpoint.rs → mother/src/.

9. `mother: move broker into mother crate` — Move src/mother/broker/ → mother/src/broker/.

10. `mother: move graph into mother crate` — Move src/commands/mother/graph.rs → mother/src/graph.rs.

11. `mother: move daemon server into mother crate` — Move src/commands/mother/daemon.rs, microserver.rs, registry.rs → mother/src/. This makes the mother/ crate the complete daemon. Note: mother/src/daemon.rs already has 327 lines of real protocol routing — the actions return placeholder text but the infrastructure is solid. This commit adds real runtime behind the existing routing.

12. `mother: move toy host implementations into mother crate` — Move src/toys/ (github.rs, session.rs, lake.rs, connector.rs, http.rs, ingress.rs, query.rs, catalog.rs) → mother/src/toys/. Toys are Mother's responsibility — she implements the host side of WIT interfaces that children consume.

13. `mother: make CLI mother commands thin` — src/commands/mother/mod.rs becomes: start (spawn daemon process), stop (signal), status (query socket). All runtime code gone from CLI.

14. `mother: delete src/mother/ and src/toys/ modules` — After all moves, delete the empty shells. CLI depends on mother crate for types only.

### Verification
- cargo build -p mother — succeeds, contains all runtime
- cargo build -p patina-ai — succeeds, no Mother runtime code
- cargo test -p mother — all tests pass
- patina mother start — daemon starts from mother crate

Exit checklist:

- [ ] runtime ownership map shows Mother logic centralized in `mother/`
- [ ] CLI Mother commands are transport/client wrappers only

## Phase 4: New toys (CV10)

15. `wit: define toy-layer-fs interface and host implementation` — wit/toys/layer-fs.wit: read-file, write-file, list-dir, delete-file, move-path, exists. Scoped to layer/ directory. Host impl in mother/src/toys/layer_fs.rs.

16. `wit: define toy-git interface and host implementation` — wit/toys/git.wit: create-tag, delete-tag, tag-exists, commit, log-oneline, diff-stat. Host impl in mother/src/toys/git.rs.

17. `sdk: add toy-layer-fs and toy-git to tiered surface` — Backends and ZST wrappers. Feature gates.

### Verification
- WIT sync check passes
- Mother host implementations have tests

## Phase 5: Move children out of core (CV7, CV9, CV12, CV13, CV14, CV15, CV17, CV18)

18. `child: scaffold spec-manager` — children/spec-manager/child.toml (needs: log, state, layer-fs, git). handle() stub.

19. `child: implement spec-manager operations` — Move logic from src/commands/spec/internal/. All CRUD, lifecycle, queries, packets. Add rename and reopen. Add HITL confirmation for complete/abandon.

20. `cli: rewrite patina spec as thin Mother client` — Route to spec-manager child. Delete src/commands/spec/internal/. Graceful error without Mother.

21. `child: make measure-health a bundled Mother child` — Move measure logic into a child that ships with Mother. Bundled = always available when Mother runs.

22. `child: move doctor to child` — Move src/commands/doctor.rs logic into children/doctor/. Thin CLI wrapper.

23. `child: move lake to child` — Move src/commands/lake.rs logic into children/lake-manager/. Thin CLI wrapper.

24. `core: remove session as core command` — Session artifacts are written by session-writer child (already exists). Remove src/commands/session/ from core CLI surface. Session operations go through Mother.

25. `cli: rewrite thin wrappers for measure, doctor, lake, session` — Each becomes: connect to Mother, route to child, format output. Error gracefully without Mother.

### Verification
- patina spec list — works with Mother + spec-manager
- patina spec list — fails gracefully without Mother
- patina measure — works with Mother + measure-health
- patina doctor — works with Mother + doctor child
- patina lake list — works with Mother + lake-manager
- Core verbs still work without Mother

Exit checklist:

- [ ] core command removals match CV set
- [ ] child replacements provide parity (or explicit policy-compliant failure)
- [ ] spec lifecycle features `rename` + `reopen` + HITL are proven

## Phase 6: Separable scrape strategies (CV11)

1. `scrape: extract strategy registration interface` — Preserve current grammar abstraction.
2. `child: make scrape-code a strategy child`
3. `scrape: orchestrator discovers strategy children`

### Verification
- patina scrape without Mother — scrapes layer + beliefs only
- patina scrape with Mother + scrape-code — also scrapes code
- patina scrape with Mother but no code child — scrapes layer + beliefs only (no error)

## Phase 7: Project manifests (CV8)

28. `mother: define project manifest format` — What children does this project need? File at .patina/manifest.toml or similar.

29. `mother: resolve children on project connect` — Mother reads manifest, checks inventory, reports missing.

30. `mother: child installation from local inventory` — Mother can install available children for a project.

### Verification
- Project connects, Mother reads manifest, reports status
- Missing children identified clearly

## Phase 8: Version cleanup (CV16)

31. `cli: decouple version from spec system` — patina version shows version. Period. No spec status query.

### Verification
- patina version works without Mother, without patina.db

## Phase 9: Verify (all CVs)

32. Full test suite — zero failures, zero warnings.
33. Core verbs with Mother stopped.
34. Child commands through Mother.
35. Child commands fail gracefully without Mother.
36. Project connect resolves children.

## Direct Code Targets

### Phase 1 — daemon routing + dead code
- src/commands/context.rs, measure/mod.rs, scry/internal/routing.rs, lake.rs, spec/mod.rs
- src/mother/daemon_client.rs (delete)
- src/commands/assay/internal/, scry/, schema/, retrieval/ (dead code)

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
- src/commands/scrape/code/ → children/scrape-code/ (new)
- src/commands/scrape/mod.rs — strategy discovery via Mother

### Phase 7 — project manifests
- .patina/manifest.toml (new format)
- mother/src/ — manifest reading + child resolution

### Phase 8 — version
- src/commands/version.rs — remove spec query

## Open Questions

- Should git scraping stay built-in or become a child now? (Recommend: stay built-in, most projects have git)
- Should measure-health be a WASM child or native code bundled in the mother crate? (Recommend: start as native, migrate to WASM later if needed)
- Project manifest format — TOML? Part of .patina/config? Separate file?
- Should the secrets cache child stay as native in Mother or become WASM?

## Build Readiness

Ready when promoted to active. Phases are sequential — complete each before starting the next. Any agent can execute from this spec + beliefs.

This design is authoritative for execution order and proof quality. If an agent cannot produce proof for a claimed completion, the phase is not complete.
