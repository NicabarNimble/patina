---
type: feat
id: spec-remote-ingestion
status: draft
created: 2026-02-17
sessions:
  origin: 20260217-101900
related:
  - layer/surface/build/feat/patina-polymorphic-extraction/SPEC.md
  - layer/surface/build/feat/mother-design/SPEC.md
beliefs:
  - work-triages-specs
  - mother-is-the-daemon
  - specs-push-discoveries-outbound
---

# feat: Remote Spec Ingestion — Mother-Native Spec Graph

> Elevate specs from local markdown into a remotely queryable system.
> Mother becomes the authority for spec metadata, while projects stay git-tracked.
> This enables remote agents (and future CLI/UX) to collaborate without waiting
> for `patina scrape layer` or manual status sync.

## Problem

Specs currently live only as files under `layer/surface/build/`. The `patina spec`
CLI reads/writes them directly and relies on the layer scraper to populate the
`patterns` + `spec_deps` tables. Pain points:

- **Stale metadata.** Blockers, targets, and titles stay outdated until the user
  runs `patina scrape layer`, making `patina spec ready` fail on fresh checkouts.
- **No remote visibility.** Mother cannot list or mutate spec state; anything
  outside the current repo cannot see ready/blocked queues.
- **Zero provenance.** There's no audit trail for status flips, checklist edits,
  or test evidence, so agents can't safely participate.

## Scope

This SPEC covers the data plane that makes specs remotely accessible while
remaining git-driven. Execution happens in three phases, each with explicit
rollback switches.

### Phase A — Project Watcher CLI

1. New command `patina spec sync` parses SPEC.md frontmatter + body (reuse
   `patina::spec::parse_spec_file`).
2. Writes normalized rows into `.patina/local/data/spec_cache.db` with tables:
   `specs(id, title, status, target, owner, updated_at)`,
   `spec_blockers(spec_id, depends_on)`, `spec_checklist(spec_id, item_id, text)`.
3. Adds `owner`, `checklist`, and `tests` sections to the SPEC template. When a
   field is missing, the sync command injects defaults but leaves files unchanged.
4. Rollback: deleting `spec_cache.db` reverts to current behavior; CLI refuses to
  read cached data unless checksum matches HEAD.

### Phase B — Mother Child: `spec-registry`

1. Scaffold a mother-child plugin that reads the project cache and uploads its
   contents to Mother via HTTP (same pattern as `mother graph sync`).
2. Data lands in `~/.patina/mother/specs.db` (new SQLite) with identical tables.
3. Mother exposes RPCs `list_specs`, `get_spec(id)`, `update_status`,
   `record_checklist`. Authentication piggybacks on existing mother daemon auth.
4. Rollback: disable the child in `~/.patina/mother/children.toml`; CLI falls
   back to local cache.

### Phase C — CLI + MCP Integration

1. `patina spec ready/list/blocked` first query Mother; if unreachable, they fall
   back to local cache and print a warning banner.
2. The MCP server registers a `specs` tool backed by Mother, enabling agents to
   query status remotely.
3. Document the new lifecycle in `layer/core/spec-driven-design.md`: specs remain
   git truth, but metadata is mirrored in Mother for orchestration.
4. Rollback: environment variable `PATINA_SPEC_OFFLINE=1` forces legacy mode.

## Non-Goals

- Enforcing checklist semantics (handled by follow-up SPEC).
- Automatic conflict resolution. If Mother has newer metadata than git, the CLI
  prompts the human; resolution flow lives in a separate fix spec.
- Persisting spec bodies outside git — only metadata is mirrored.

## Exit Criteria

1. `patina spec ready` shows identical results before/after running `spec sync`
   or querying Mother (minus warning banners).
2. Mother CLI (`patina mother spec list`) lists every spec with status/target.
3. MCP `specs.list` tool returns JSON objects with `id,status,target,owner`.
4. Disabling the mother child (or deleting `spec_cache.db`) reverts behavior
   without breaking existing commands.
5. `cargo test --package patina --lib spec` covers the new sync command and
   mother RPC glue.
