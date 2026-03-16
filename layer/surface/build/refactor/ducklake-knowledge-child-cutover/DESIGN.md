# Design: DuckLake Knowledge-Child Cutover

## Why This Design

DuckLake needs one runtime identity to match Mother/Child/Toy doctrine and to
be usable by third-party app builders.

The design keeps the established architecture:

- Mother is authority and runtime substrate owner.
- DuckLake knowledge-child is app orchestration brain.
- Toys are granted capability contracts.

Instead of moving authority into child code, this design upgrades granted
capabilities so the child can express full DuckLake behavior without legacy
runtime ambiguity.

## Build Target

Deliver a single authoritative DuckLake path where:

- user can attach any repository with granted scope,
- issues + pull requests ingest through OAuth vault-backed auth,
- sync is incremental and durable,
- broker lake routing uses knowledge-child path,
- native DuckLake child path is removed after parity verification.

## Execution Model

`Destination::Lake` uses one explicit cutover model:

- broker resolves/creates repo binding,
- broker enqueues knowledge-child sync intent,
- broker waits with bounded timeout for completion signal from
  knowledge-child state/checkpoint status,
- on timeout or failure, broker returns actionable status without falling back
  to implicit native execution.

This keeps orchestration in the knowledge-child runtime while preserving the
existing operator expectation of request/response feedback.

## Resolved Decisions

- Keep DuckLake as a knowledge-child app identity (`plugins/ducklake`).
- Preserve TaskIntent as substrate lane, not a peer toy.
- Use host-granted credential mapping + vault resolution for OAuth auth.
- Expand capability interfaces rather than reintroducing ambient `host_http`.
- Old native path remains migration oracle only until parity passes.
- Use `enqueue + bounded wait` as the only broker invocation model for lake
  cutover.
- Implement a first-class binding control-plane surface (not child-only
  bootstrap by convention).
- Add explicit native cursor/checkpoint migration before native path removal.

## Commits

1. `spec(ducklake): define knowledge-child cutover parity contract`
   - Add parity matrix and fixture expectations.

2. `feat(runtime): freeze broker invocation model for knowledge-child cutover`
   - Implement enqueue + bounded wait flow and status mapping.

3. `feat(wit): add connector-grade repo sync interface and lake parity ops`
   - Expand WIT for repo-scoped issues/prs sync and lake semantics.

4. `feat(runtime): implement host connector/lake parity with vault-backed oauth injection`
   - Implement runtime host behavior for new interfaces.

5. `feat(sdk): expose typed granted connector/lake APIs for app builders`
   - Update toy and child SDKs with typed contracts.

6. `feat(control-plane): add repo binding command/API and grant provisioning`
   - Add create/update/list/remove binding surface.

7. `feat(ducklake): implement multi-repo issues/prs source model on knowledge-child`
   - Upgrade DuckLake app behavior and source config model.

8. `feat(migration): add native cursor/checkpoint migration to knowledge-child model`
   - Add mapping, idempotent migration, and rollback-safe semantics.

9. `refactor(broker): route lake destination through knowledge-child`
   - Cut over broker path to new orchestration route.

10. `test(ducklake): add parity, migration, and failure-path integration suite`
   - Assert legacy parity before removal.

11. `refactor(ducklake): remove native runtime identity after parity pass`
   - Remove old path and dead code.

## Direct Code Targets

- `plugins/ducklake/src/lib.rs`
  - Source model, sync orchestration, issues/prs routing, checkpoint/cursor behavior.
- `plugins/ducklake/plugin.toml`
  - Capability declarations for connector/ingress and lake grants.
- `wit/knowledge-child/deps/patina-host/host.wit`
  - Connector/lake contract expansion.
- `wit/knowledge-child/knowledge-child.wit`
  - Import alignment for new host capability interfaces.
- `crates/patina-child-sdk/wit/knowledge-child/deps/patina-host/host.wit`
  - SDK-side WIT mirror.
- `crates/patina-child-sdk/wit/knowledge-child/knowledge-child.wit`
  - SDK world imports.
- `crates/patina-toy-sdk/src/lib.rs`
  - Typed toy contracts for connector/lake operations.
- `crates/patina-child-sdk/src/lib.rs`
  - Granted API surface and substrate handling for app authors.
- `src/plugin/internal/knowledge_child.rs`
  - Host implementation of new WIT interfaces, capability enforcement.
- `src/plugin/internal/mod.rs`
  - Manifest parsing and grant plumbing for repo-scoped connectors.
- `src/toys/lake.rs`
  - Lake host backend parity semantics.
- `src/broker/mod.rs`
  - Route `Destination::Lake` through knowledge-child enqueue + bounded wait path.
- `src/commands/connect.rs`
  - Add or extend user-facing repo binding management surface.
- `src/commands/mother/mod.rs`
  - Add mother-side binding lifecycle entrypoints if needed.
- `src/commands/mother/daemon.rs`
  - Ensure runtime path supports bounded status observation semantics.
- `src/connect/internal/resolve.rs`
  - OAuth connection resolution integration for connector grants.
- `src/connect/internal/store.rs`
  - Credential lookup path consistency with vault mappings.
- `src/mother/state.rs`
  - Persist and query status needed for bounded wait completion checks.
- `children/ducklake/src/main.rs`
  - Legacy behavior reference for parity and migration mapping.
- `src/plugin/internal/tests.rs`
  - Capability parse, grant enforcement, and auth path tests.

## Verification Plan

1. **WIT/SDK contract checks**
   - Regenerate/build child SDK bindings.
   - Compile `patina-child-sdk` and `patina-toy-sdk` tests.

2. **Knowledge-child behavior tests**
   - Configure multiple repo sources.
   - Sync issues + PRs via granted connector capability.
   - Validate lake writes and cursor progression.

3. **OAuth vault auth tests**
   - Valid OAuth secret injection path works.
   - Missing secret fails with actionable error.
   - Domain/scope mismatch denied by grant policy.

4. **Failure-path tests**
   - Partial type ingestion failure behavior.
   - Retry/escalation behavior for transient and auth failures.

5. **Parity suite**
   - Same fixture repos through old and new path.
    - Compare output row counts, schema shape, cursor state, and status records.

6. **Migration tests**
   - Existing native cursor/checkpoint state migrates or is honored by
     compatibility adapter without duplicate ingestion.

7. **Cutover tests**
   - `Destination::Lake` uses knowledge-child route.
   - Invocation uses enqueue + bounded wait with explicit timeout behavior.
   - No native ducklake spawn path remains after final step.

## Build Readiness

- [ ] Parity fixture set exists for issues + PRs.
- [ ] OAuth vault test fixtures available.
- [ ] WIT changes reviewed for backward compatibility and migration.
- [ ] Broker invocation model (enqueue + bounded wait) implemented and tested.
- [ ] Binding command/API surface implemented and idempotent.
- [ ] Cursor/checkpoint migration plan implemented and verified.
- [ ] Native path removal deferred until parity checklist is green.

## Open Questions

- Should repo-specific connector capability be modeled as an expansion of
  `ingress` or as a new `connector` interface in WIT?
  - Recommended default: add a dedicated `connector` interface for clarity and
    future extensibility while keeping current ingress for simple static fetch.

- Should one DuckLake child instance own many repositories or should each source
  be its own child instance by default?
  - Recommended default: single child identity with many source bindings,
    each binding grant-scoped by Mother policy.

- Should broader third-party SDK documentation and tutorials block cutover?
  - Recommended default: no; require stable typed contracts and one reference
    flow in this spec, and track broader SDK/docs polish in a follow-on spec.
