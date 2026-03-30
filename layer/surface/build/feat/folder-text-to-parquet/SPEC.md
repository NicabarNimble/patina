---
type: feat
id: folder-text-to-parquet
status: active
created: 2026-03-27
parent: child-construction-canon
sessions:
  origin: 20260327-104954-066673000
blocked_by: []
beliefs:
  - "[[children-have-agency-toys-are-capabilities]]"
  - "[[observation-at-the-boundary]]"
  - "[[wasi-is-foundation-not-option]]"
related:
  - sdk/patina-sdk/
  - children/
  - wit/knowledge-child/
  - layer/surface/build/feat/child-construction-canon/
exit_criteria:
  - id: ftp1-composite-prototype
    text: "A single composite child proves the full flow: watch folder → parse text → write parquet → register in catalog."
    checked: true
  - id: ftp2a-first-split-proven
    text: "Two children compose via file.found events. Discovery (file-system-monitor) and processing (folder-text-to-parquet) separated with working subscribe/ack/offset."
    checked: true
  - id: ftp2-six-children-built
    text: "Composite split into 6 focused children: file-system-monitor, content-extractor, schema-enforcer, dedup-filter, record-writer, lakehouse-catalog."
    checked: true
  - id: ftp3-children-compose
    text: "All 6 children compose via events into a working pipeline on a deterministic test fixture."
    checked: true
  - id: ftp4-mother-metrics
    text: "Mother-tier metrics collected automatically for every child without child implementation."
    checked: true
  - id: ftp5-acceptance-gates
    text: "All acceptance gates pass with real measurements."
    checked: true
  - id: ftp6-schema-evolution
    text: "Lakehouse catalog handles at least one schema evolution (add nullable column) without breaking existing data."
    checked: true
  - id: ftp7-storage-portable
    text: "Parquet output works on local folder. Storage location is configurable (designed for S3/similar later)."
    checked: true
  - id: ftp8-recipe-validated
    text: "Objective recipe from parent canon filled in with concrete values from implementation."
    checked: true
---
# feat: folder-text-to-parquet

## Problem

The child construction canon defines rules and a registry model but has no implementation. This is the first MVP — it builds the 6 core reusable children by composing them into a data pipeline objective.

## Goal

Build a working pipeline: watch a folder for text files, parse them into structured records, write encrypted parquet files managed by a lakehouse catalog. In doing so, build 6 reusable children that compose into any data ingestion objective.

## Non-Goals

- Optimizing for production-scale throughput in this phase.
- Supporting non-text file formats (content-extractor is designed for it, but only text is tested here).
- Building S3 storage backend (designed for it, local folder only in this MVP).
- Building Iceberg/Delta support (DuckLake catalog now, designed for lakehouse portability).

## Children Built

### 1. `file-system-monitor`

**Capability:** Watch a configured folder, detect new/changed files, emit file-found events.

**Toys:** `wasi:filesystem` (read-only scoped preopen), `wasi:messaging/producer` (emit events), `wasi:logging`, `patina:measure`

**How it works:** Handles `scan` action with a `folder_path` payload. Flat-scans the folder, skips hidden/symlinks/empty/non-matching files, publishes `file.found` events for each valid file with source_path, source_hash, source_size_bytes. Reports `files_discovered` metric. Stateless — no keyvalue tracking of known files in current implementation (future: add state for change detection across scans).

**Reuse:** log-monitoring, document-indexing, code-analysis, audit-trail — anything reacting to file changes.

### 2. `content-extractor`

**Capability:** Given a file reference (from event), read the file, extract structured records with provenance metadata.

**Toys:** `wasi:filesystem` (read-only scoped), `patina:events-stream` (subscribe to file events), `wasi:messaging/producer` (publish record events), `wasi:logging`

**How it works:** Subscribes to `file.found` events. Reads the file. Produces structured records with: `source_path`, `source_hash`, `source_modified_at`, `source_size_bytes`, `content`, `content_hash`, `content_type`, `encoding`, `line_count`. Publishes `record.extracted` events. Parsing strategy is configurable by manifest (text, markdown, CSV — only text in this MVP).

**Reuse:** email-to-lake, api-sync, RSS-to-lake, webhook-to-lake — any ingestion from external sources.

### 3. `schema-enforcer`

**Capability:** Validate records against a declared schema before allowing them downstream.

**Toys:** `patina:events-stream` (subscribe to record events), `wasi:messaging/producer` (publish validated events or rejection events), `wasi:logging`

**How it works:** Subscribes to `record.extracted` events. Validates each record against the declared schema (required columns, types, non-null constraints). Valid records are republished as `record.validated`. Invalid records are published as `record.rejected` with reason. Schema is declared in manifest or loaded from lakehouse catalog.

**Reuse:** any pipeline that writes structured data — the gatekeeper before persistence.

### 4. `dedup-filter`

**Capability:** Detect and reject duplicate records by content hash.

**Toys:** `patina:events-stream` (subscribe), `wasi:keyvalue` (store seen hashes), `wasi:messaging/producer` (publish), `wasi:logging`

**How it works:** Subscribes to `record.validated` events. Checks `content_hash` against keyvalue store of seen hashes. New records are published as `record.ready`. Duplicates are published as `record.duplicate` (for observability). Dedup window/strategy configurable by manifest.

**Reuse:** any ingestion pipeline with potential duplicates — email, API polling, feed monitoring.

### 5. `record-writer`

**Capability:** Batch records into parquet files with partitioning and encryption.

**Toys:** `patina:events-stream` (subscribe to ready records), `wasi:keyvalue` (batch buffer), `wasi:logging`, `patina:measure` (domain metrics: records_written, batch_size, write_latency)

**How it works:** Subscribes to `record.ready` events. Accumulates records into batches. When batch threshold is reached (count or time), writes a parquet file with configured partitioning (e.g., `ingested_date=YYYY-MM-DD/source_folder=X/`). Publishes `file.written` event with file path and record count. Encryption applied at write time (storage-layer for now, parquet modular encryption tracked for future).

**Output storage:** writes to configured location (local folder in this MVP, designed for S3/similar via storage backend abstraction).

**Reuse:** any system persisting structured records to columnar storage.

### 6. `lakehouse-catalog`

**Capability:** Manage tables over parquet files — register files, track schema, handle evolution.

**Toys:** `wasi:sql` (DuckDB/DuckLake catalog), `wasi:keyvalue` (metadata cache), `wasi:logging`

**How it works:** Subscribes to `file.written` events. Registers the new parquet file into the table catalog. Tracks schema version. Handles schema evolution (add nullable column, version bump). DuckLake now, designed for Iceberg/Delta migration (parquet files are standard, only catalog swaps).

**Schema design:**

Provenance layer (unencrypted):

| Column | Type | Purpose |
|---|---|---|
| `record_id` | `UUID` | Unique per record |
| `source_path` | `STRING` | Original file path |
| `source_hash` | `STRING` | SHA-256 of source file |
| `source_modified_at` | `TIMESTAMP WITH TIME ZONE` | Source file last modified |
| `source_size_bytes` | `BIGINT` | Source file size |
| `ingested_at` | `TIMESTAMP WITH TIME ZONE` | When record was written |
| `batch_id` | `STRING` | Ingest run ID (checkpoint recovery) |
| `schema_version` | `INTEGER` | Schema version number |

Content layer (encrypted at storage level):

| Column | Type | Purpose |
|---|---|---|
| `content` | `STRING` | Text content |
| `content_hash` | `STRING` | SHA-256 of content |
| `content_type` | `STRING` | MIME type |
| `encoding` | `STRING` | Character encoding |
| `line_count` | `INTEGER` | Lines in source |

Schema evolution rules:
- New columns always nullable
- Never remove columns — deprecate and null-fill
- Never change column types — add new column
- `schema_version` increments on every change

**Reuse:** any system writing parquet — the catalog is the lakehouse layer.

## Composition

```
file-system-monitor
    → [file.found] →
content-extractor
    → [record.extracted] →
schema-enforcer
    → [record.validated] →
dedup-filter
    → [record.ready] →
record-writer
    → [file.written] →
lakehouse-catalog
```

All composition via `wasi:messaging/producer` (publish) and `patina:events-stream` (subscribe/ack). All children are action-driven — Mother calls `handle()` with the appropriate action. file-system-monitor receives `scan`, downstream children receive their respective consume actions.

## Acceptance Gates

- Throughput >= declared baseline over declared run window. *(Mother-observed)*
- Error rate <= declared threshold. *(Mother-observed)*
- Checkpoint restart correctness = post-crash replay produces identical output. *(Mother-observed)*
- Duplicate rate = 0 on deterministic rerun fixture. *(child-declared via `measure`)*
- Provenance completeness = all provenance columns present and non-null. *(child-declared via `measure`)*
- Schema evolution = add-column migration produces no data loss. *(integration test)*

## Approach

Each step builds the real system. Unknowns from the parent canon are resolved as they're encountered — not pre-validated. If a step surfaces a problem, document it, propose adaptation, get user approval, update this spec.

1. **Build the composite prototype.** One child: watch folder, parse text, write parquet, register in catalog.
   - *Unknowns hit here:* Does parquet-rs compile to `wasm32-wasip2`? Does `wasi:filesystem` write work with scoped preopens? If either breaks, resolve in the real system before proceeding.

2. **Observe and identify seams.** Run the composite child. Check Mother-tier metrics — do they exist? Are they useful?
   - *Unknown hit here:* Does Mother automatic observation produce useful data? If not, determine what instrumentation is needed and add it.

3. **First split — two children.** Break the composite into two children connected by events. Most natural seam: discovery vs processing.
   - *Unknown hit here:* Does two-child event composition work? What's the latency? If the model has problems, solve them in the real pipeline before splitting further.

4. **Continue splitting to focused children.** Based on what the first split taught, split further toward the 6-child target. Each split is real — the pipeline keeps working at every stage.

5. **Validate composition, schema evolution, acceptance gates.** All on real data, real children, real measurements.

6. **Fill in the canon's recipe format** with concrete values learned from building. Update parent spec with user approval.

## Verification

```bash
patina spec check folder-text-to-parquet --json
cargo check --workspace -q
cargo test -q --workspace
```

## Concrete First Action

Scaffold `children/folder-text-to-parquet/` from the template. The composite child does the full flow in one child.

### Manifest (`child.toml`)

```toml
[child]
name = "folder-text-to-parquet"
kind = "knowledge-child"
role = "app"

[needs]
toys = ["log", "state", "events", "measure"]

[needs.metrics]
files_discovered = { type = "counter", labels = ["source_folder"] }
records_written = { type = "counter", labels = [] }
write_latency_ms = { type = "gauge", labels = [] }
```

Note: `wasi:filesystem` for folder scanning is the first unknown. If preopens work, add `fs` to toys. If a host-side change is required to enable preopens, that is an allowed adaptation — document it, propose it, get user approval. That's not scope creep, it's the unknowns resolution process.

### Scan contract

- **Flat scan only** — no recursion into subdirectories.
- **Skip:** hidden files (dotfiles), symlinks, non-UTF8 files, zero-byte files (log as skipped, don't fail).
- **Include:** files matching `*.txt`, `*.md`. All other extensions skipped.
- **Config precedence:** action payload `folder_path` overrides manifest config. If neither is set, return error.

### Content rules

- **`content_type`:** extension-based only, no sniffing. `.txt` = `text/plain`, `.md` = `text/markdown`.
- **`encoding`:** assumed `utf-8`. If file fails UTF-8 decode, skip and log.
- **`source_hash`:** SHA-256 of raw file bytes.
- **`content_hash`:** SHA-256 of extracted content string.

### State contract (keyvalue)

Key format: `record:{source_hash}` (one record per key, dedup-safe).

Value: JSON object matching the schema from the "Schema design" section above. Example:
```json
{
  "record_id": "550e8400-e29b-41d4-a716-446655440000",
  "source_path": "tests/fixtures/folder-text-to-parquet/notes.txt",
  "source_hash": "a1b2c3...",
  "source_modified_at": "2026-03-28T10:00:00Z",
  "source_size_bytes": 1234,
  "content": "file content here",
  "content_hash": "d4e5f6...",
  "content_type": "text/plain",
  "encoding": "utf-8",
  "line_count": 42,
  "ingested_at": "2026-03-28T14:30:00Z",
  "batch_id": "scan-20260328-143000",
  "schema_version": 1
}
```

### Event contract

Topic: `file.found`

Payload: JSON object:
```json
{
  "source_path": "tests/fixtures/folder-text-to-parquet/notes.txt",
  "source_hash": "a1b2c3...",
  "source_size_bytes": 1234,
  "discovered_at": "2026-03-28T14:30:00Z"
}
```

### Metrics contract

| Name | Type | Labels | When emitted |
|---|---|---|---|
| `files_discovered` | counter | `source_folder` | After each file successfully processed |
| `records_written` | counter | (none) | After each record written to state |
| `write_latency_ms` | gauge | (none) | Time to write one record to state |

### Test fixture

Create `tests/fixtures/folder-text-to-parquet/` with deterministic content:
- `hello.txt` — simple text, 3 lines
- `notes.md` — markdown with headers, 10 lines
- `readme.txt` — longer text, 20+ lines
- `duplicate-of-hello.txt` — exact same content as `hello.txt` (same content_hash, different source_path)
- `empty.txt` — zero bytes (should be skipped)
- `.hidden` — dotfile (should be skipped)
- `image.png` — wrong extension (should be skipped)

### Test assertion guidance

- **Assert exact value:** `source_hash`, `content_hash`, `line_count`, `source_size_bytes`, `content_type`, `encoding`, `schema_version`, `content`
- **Assert present and valid type:** `source_modified_at` (timestamp), `ingested_at` (timestamp), `record_id` (UUID format), `batch_id` (string, non-empty)

### Success criteria

The composite child:
1. Loads in Mother (instantiation succeeds)
2. Handles `scan` action with the fixture folder path
3. Produces structured records with all provenance columns
4. Writes records to keyvalue state with `record:{source_hash}` keys
5. Publishes `file.found` events for each discovered file
6. Reports all 3 declared metrics via `patina:measure`
7. Skips hidden, empty, and non-matching files without failing
8. Integration test passes

This is exit criterion `ftp1-composite-prototype`. When it passes, mark it checked and stop for review before proceeding to step 2.

Everything after this — parquet format, lakehouse catalog, splitting into focused children — builds on this working foundation.

## Review cadence

Review spec alignment at each approach step completion:
- After step 1 (composite works) — review unknowns hit, update spec with user approval
- After step 3 (first split works) — review composition model, update spec
- After step 5 (all gates pass) — final spec review, fill in recipe format

## Build Readiness

Ready to start. First action: scaffold the composite child and test fixture.
