---
type: explore
id: duckdb-durable-execution
status: draft
created: 2026-04-07
sessions:
  origin: 20260406-192728-341380000
beliefs:
  - "[[wasi-is-foundation-not-option]]"
  - "[[pandos-are-products-children-are-compute]]"
related:
  - layer/surface/build/feat/pando-platform/SPEC.md
  - layer/surface/build/feat/mother-duckdb-ducklake-federation/SPEC.md
exit_criteria:
  - Decide whether durable execution is a Patina internal (Mother infra) or a toy/pando capability, or both
  - Identify which Absurd primitives map cleanly to DuckDB and which need different solutions
  - Determine whether DuckDB's single-writer model is an advantage or a limitation for this pattern
  - Prototype checkpoint/replay for a single child with parquet-backed state
  - Decision on partition strategy (time-bucketed parquet files vs DuckDB native partitioning)
---
# explore: DuckDB-Backed Durable Execution

> Can DuckDB + Mother provide Absurd-style durable execution without Postgres,
> using parquet partitions for time-bucketed state and Mother as the coordinator?

## Question

Absurd (earendil-works/absurd) proves that durable execution can live entirely
inside a database — no external services, no message brokers. It uses Postgres
stored procedures for coordination and row-level locking for worker claims.

Patina already has half the pieces: Mother coordinates children, children
communicate through events, DuckDB stores structured state. What's missing is
explicit checkpoint/replay/retry semantics that make workflows durable.

Can we build this on DuckDB instead of Postgres? What do we gain, what do we
lose, and where does it live in Patina's architecture?

## Absurd's Core Primitives

From studying the main and partition-support branches:

| Primitive | What it does | Postgres mechanism |
|---|---|---|
| Task | Unit of work dispatched to a queue | Row in `t_<queue>` |
| Run | One attempt to execute a task | Row in `r_<queue>` |
| Checkpoint | Saved step result (memoized) | Row in `c_<queue>` (task_id + step name) |
| Event | External signal that can wake a suspended task | Row in `e_<queue>`, first-write-wins |
| Wait | Task suspended until a named event arrives | Row in `w_<queue>` |
| Claim | Worker lease on a run | `SELECT ... FOR UPDATE SKIP LOCKED` |
| Partition | Weekly time bucket for high-volume queues | Postgres declarative range partitioning on UUIDv7 |

## What Maps to DuckDB

### Direct mappings (no friction)

- **Tasks, runs, checkpoints, events, waits** — DuckDB tables with JSON
  columns. DuckDB's JSON support handles the same payloads Absurd stores in
  `jsonb`. Schema is identical.
- **Time partitioning** — Absurd's partition-support branch uses weekly
  buckets keyed on UUIDv7 time extraction. DuckDB can partition via
  Hive-style directory layout with weekly parquet files. Partition
  creation/retirement is just file management — no DDL, no `DETACH
  PARTITION CONCURRENTLY`.
- **Cleanup/retention** — Absurd has per-queue TTL + cleanup limits + detach
  policies. With parquet partitions, retention is `rm` on old files. No
  vacuum, no bloat.
- **Policy per queue** — Absurd stores policy in `absurd.queues` (lookahead,
  lookback, TTL, detach mode). Maps directly to a config table or to
  `pando.toml` queue sections.

### Different solutions needed

- **Worker claims / `SKIP LOCKED`** — DuckDB is single-writer, embedded. No
  concurrent transactions, no row-level locking. But Mother is the single
  coordinator — she doesn't need `SKIP LOCKED` because she assigns work to
  children directly. Claim semantics become Mother's dispatch logic, not
  database locking.
- **`LISTEN/NOTIFY`** — Absurd uses Postgres channels for event wakeup.
  Mother already has an event bus for child communication. Event emit/await
  maps to Mother's existing event dispatch, with DuckDB as the persistence
  layer (write event, then notify).
- **Stored procedures** — Absurd's coordination logic lives in plpgsql
  functions. In Patina, this logic lives in Mother (Rust) or in a dedicated
  durable-execution child. DuckDB is the state store, not the compute layer.
- **`pg_cron`** — Absurd uses it for automated partition provisioning, cleanup,
  and detach. Mother *is* the scheduler. A periodic task in Mother's event
  loop replaces all three cron jobs.

## DuckDB's Single-Writer Advantage

Absurd's partition branch goes to significant effort to handle concurrent
writers safely — `DETACH PARTITION CONCURRENTLY`, careful locking, race
conditions in `spawn_task` idempotency. DuckDB's single-writer model
eliminates all of this:

- **No partition detach races** — Mother is the only writer. She stops
  writing to a partition, then deletes or archives the file.
- **No claim contention** — Mother assigns tasks to children. No need for
  `SKIP LOCKED` because there's no contention.
- **No idempotency races** — `spawn_task` in Absurd guards against
  concurrent inserts with the same idempotency key. With one writer, this
  is a simple check-then-insert.
- **Partition rotation is file rotation** — closing one parquet file and
  opening the next is atomic from Mother's perspective.

The limitation: DuckDB can't serve multiple independent workers pulling from
the same queue. But Patina doesn't need that — Mother is the coordinator,
children are the workers, and Mother assigns work. This is pull-from-Mother,
not pull-from-database.

## Where Does It Live?

Three options, not mutually exclusive:

### Option A: Mother internal infrastructure

Durable execution as Mother's own reliability layer. Mother uses it to make
child orchestration crash-safe:

- Child dispatch becomes a task with checkpointed steps
- If Mother restarts, she replays from last checkpoint
- Event stream processing gets exactly-once semantics
- Pando composition wiring becomes durable (pipeline pandos survive crashes)

This is invisible to pandos. They get reliability for free.

### Option B: Patina toy (`patina:durable`)

Expose durable execution as a toy that children can use:

```wit
interface durable {
    spawn: func(queue: string, task-name: string, params: string) -> task-id;
    checkpoint: func(step: string, state: string) -> option<string>;
    await-event: func(event-name: string) -> string;
    emit-event: func(event-name: string, payload: string) -> ();
}
```

Children that need durable workflows import the toy. Mother provides the
host implementation backed by DuckDB. This is the Absurd SDK model — but
the SDK is a WIT interface, not a language library.

### Option C: Durable execution pando

A pando that exposes durable workflows as a user-facing capability:

- `patina workflow list` — show running workflows
- `patina workflow spawn <queue> <task>` — kick off a workflow
- `patina workflow inspect <task-id>` — show checkpoints and state
- `patina workflow retry <task-id>` — retry a failed task

This builds on Option B (the toy) and adds CLI surface.

### Likely path

Start with **A** (Mother internal) to prove the checkpoint/replay model on
DuckDB. Then extract **B** (the toy) once the primitives are stable. **C**
comes last, after the pando platform exists to host it.

## Partition Strategy

Two approaches for time-bucketed storage:

### Hive-style parquet directories

```
~/.patina/data/queues/<queue>/
  tasks/year=2026/week=14/data.parquet
  tasks/year=2026/week=15/data.parquet
  runs/year=2026/week=14/data.parquet
  ...
```

DuckDB reads these with `read_parquet('tasks/**/*.parquet', hive_partitioning=true)`.
Retention is `rm -rf year=2026/week=10/`. Mother manages the directory tree.

### DuckDB-native tables with periodic export

Keep live data in DuckDB tables. Periodically export completed/old partitions
to parquet and drop from the live tables. DuckDB's `COPY ... TO` with
partitioning handles the export.

**Tradeoff:** Hive-style is simpler for retention but slower for queries that
span many partitions. Native tables are faster for live queries but need
explicit export/cleanup logic.

Likely answer: **native tables for live data, parquet export for archive.**
Same pattern as Absurd's detach — active partitions are hot, old partitions
become cold storage.

## Open Questions

1. **Checkpoint granularity** — Absurd checkpoints per step name per task.
   Is that the right granularity for children, or do children need
   finer-grained checkpointing (per-record in a stream)?

2. **Replay semantics** — Absurd replays by re-running the task function and
   loading cached step results. For children, replay means re-instantiating
   the WASM module and feeding it cached state. How does this interact with
   WASI capabilities that have side effects (filesystem writes, HTTP calls)?

3. **Queue-per-pando or shared queues?** — Absurd creates tables per queue.
   Should each pando get its own queue (isolation) or share queues
   (simpler management)?

4. **Event deduplication across restarts** — Absurd's first-write-wins
   events are simple in Postgres (unique constraint). In DuckDB with
   parquet files, dedup needs either a live table for events or a bloom
   filter on archived partitions.

5. **Does this subsume `patina:task`?** — The existing task toy has overlap
   with durable execution primitives. Explore whether task becomes a thin
   wrapper around the durable execution layer or remains separate.

## References

- earendil-works/absurd (main) — core durable execution model
- earendil-works/absurd (partition-support) — time-partitioned queues,
  policy-driven retention, pg_cron automation
- Announcement post: lucumr.pocoo.org/2025/11/3/absurd-workflows/
