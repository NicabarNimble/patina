# Design: DuckLake GitHub Lakehouse Ingestion

## Why This Design

DuckLake runtime identity is now stabilized as a knowledge-child path, but the
ingestion behavior is still migration-grade. This design upgrades ingestion to
enterprise-correctness while preserving Mother/Child/Toy doctrine:

- Mother decides policy, grants, and continuity guarantees.
- Child acts as ingestion orchestration brain.
- Toys constrain authority to explicit lake/connector/state/checkpoint/measure
  surfaces.

## Build Target

Deliver a production-grade GitHub ingestion pipeline that:

- captures complete issues/PR scope,
- runs deterministic incremental sync with idempotent materialization,
- persists lake outputs as encrypted parquet blocks with DuckDB metadata,
- exposes reconciliation and operator telemetry,
- provides bronze/silver/gold outputs for downstream transforms and agents.

## Resolved Decisions

- Keep ingestion authority in DuckLake knowledge-child orchestration, not broker
  business logic.
- Use two-phase ingestion: list pagination first, bounded detail fanout second.
- Treat stable entity IDs + watermarks as mandatory idempotency primitives.
- Keep metadata/checkpoint state in Mother-owned DuckDB state surfaces.
- Use explicit retry classes and bounded concurrency to protect API and runtime.
- Keep this lane GitHub-focused; multi-provider abstraction is follow-on work.

## Commit Slices

1. `spec(ducklake): lock github scope + keys + watermark contract`
   - Finalize per-entity keys, retention, backfill, and watermark policy.

2. `feat(ducklake): implement phase-a list ingestion with cursor persistence`
   - Issues + pulls list pagination and durable cursors/checkpoints.

3. `feat(ducklake): implement phase-b bounded fanout for child entities`
   - Comments/events/reviews/review-comments/optional commits with adaptive
     backoff and concurrency bounds.

4. `feat(lake): materialize idempotent upserts with stable keys`
   - No duplicate writes under rerun/retry/restart conditions.

5. `feat(lakehouse): add parquet partition writer + metadata manifests`
   - Encrypted parquet partitioning + run/manifests/schema/checkpoint metadata.

6. `feat(quality): add reconciliation, late-arrival, and dead-letter flow`
   - API count parity, lag checks, replay handling, DLQ surface.

7. `feat(observability): emit ingestion operator telemetry`
   - Duration/calls/bytes/retries/lag per run and endpoint.

8. `feat(outputs): expose bronze/silver/gold blocks`
   - Raw snapshots, normalized entities, analytics/agent-ready blocks.

## Direct Code Targets

- `children/ducklake/src/lib.rs`
  - Planner, phase orchestration, per-entity processing, checkpoint writes.
- `src/toys/lake.rs`
  - Cursor/checkpoint semantics and table/materialization helpers.
- `src/toys/connector.rs`
  - Repo-scoped connector binding + sync contract integration.
- `src/mother/broker/mod.rs`
  - Destination lake invocation boundaries and bounded wait semantics.
- `src/mother/state.rs`
  - Metadata/checkpoint/run-state persistence used by ingestion control-plane.
- `src/plugin/internal/knowledge_child.rs`
  - Host-granted connector/lake execution path and telemetry emission plumbing.
- `src/plugin/internal/tests.rs`
  - Manifest grant checks and ingestion-path behavior assertions.

## Verification Plan

1. **Contract scope checks**
   - Verify issues + issue comments + issue events + pulls + pull comments +
     reviews + review comments (+ optional commits flag) are represented in
     planner/handlers.

2. **Two-phase ingestion tests**
   - Phase A pagination feeds Phase B bounded fanout with retry/backoff behavior.

3. **Idempotency tests**
   - Re-running same window produces no duplicate logical entities.
   - Watermark progression is monotonic for each repo/entity.

4. **Lakehouse storage tests**
   - Parquet partition shape matches contract: org/repo/entity/date.
   - Metadata rows exist for run/manifests/checkpoints/schema snapshots.

5. **Quality controls tests**
   - Reconciliation counts are emitted and enforce parity thresholds.
   - Late arrivals and dead-letter paths are observable and replayable.

6. **Telemetry tests**
   - Ingestion run emits duration/calls/bytes/retries/lag metrics.

7. **Output-tier tests**
   - Bronze/silver/gold artifacts are produced for one reference repo binding.

## Build Readiness

- [ ] SPEC status/body consistency fixed (active vs blocked)
- [ ] Scope/retention/backfill/watermark contract made explicit in SPEC
- [ ] Verification commands mapped to each exit criterion in SPEC
- [ ] Fixture repo(s) and deterministic replay windows selected
- [ ] Slice order chosen (single-lane vs split)

## Final Decisions

- Pull commits remain feature-flagged and off by default.
- Reconciliation parity uses bounded tolerance windows (default threshold in
  SPEC policy values).
- Bronze/silver/gold ship as physical outputs plus minimal stable gold views.
