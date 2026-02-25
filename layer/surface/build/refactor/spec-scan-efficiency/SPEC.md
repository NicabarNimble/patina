---
type: refactor
id: spec-scan-efficiency
status: ready
created: 2026-02-25
sessions:
  origin: 20260225-104204
related:
- spec-query-filesystem-truth
exit_criteria: []
---
# refactor: Eliminate redundant filesystem scans in spec queries

> scan_disk_specs and spec_age_days_from_list re-read files per call — O(n) file reads in display loops

## Current State

The spec module performs redundant filesystem reads in several hot paths:

**Problem 1: `spec_age_days_from_list()` (queue.rs:180)**
Called in loops inside `show_ready_specs()` and `show_spec_list()` for every paused/blocked spec. Each call does: `find_spec()` → filesystem scan → `read_to_string()` → `parse_spec_file()` — just to extract `paused_date` or `blocked_date`. For N paused/blocked specs, this is N full file reads and parses that could have been captured during the initial `scan_disk_specs()` pass.

**Problem 2: `scan_disk_specs()` called multiple times per request**
`show_ready_specs()` calls `get_all_specs()` (which calls `scan_disk_specs()`) and also calls `get_blocked_specs()` (which queries DB, but the enhanced view also calls `get_all_specs` for drafts). A single `patina spec ready` can trigger 2+ full directory walks.

**Problem 3: `find_spec()` filesystem fallback (archive.rs:279)**
When DB misses, `find_spec()` calls `scan_disk_specs()` then also calls `find_spec_file_on_disk()` — two separate recursive walks of the same directory tree.

**Flagged by:** Jon Gjengset (unnecessary re-reads — parse once, pass the data), Rich Sutton (the simpler approach would be one scan that returns rich data).

## Target State

1. `scan_disk_specs()` parses and returns date fields (paused_date, blocked_date) so callers don't need to re-read
2. Display functions receive pre-scanned data instead of re-scanning per spec
3. `find_spec()` filesystem path uses the same scan result instead of walking twice

## Steps

1. Extend `SpecInfo` to include `paused_date: Option<String>` and `blocked_date: Option<String>` — parsed during `scan_disk_specs()`
2. Change `spec_age_days_from_list()` to compute from the `SpecInfo` fields directly (no file read)
3. Refactor `show_ready_specs()` to call `get_all_specs()` once and derive all subsets (ready, active, paused, drafts, blocked) from that single result
4. In `find_spec()` filesystem path, combine `scan_disk_specs` and `find_spec_file_on_disk` into one walk

## Key Files

```
src/commands/spec/internal/queries.rs  — extend SpecInfo, refactor show_ready_specs
src/commands/spec/internal/queue.rs    — simplify spec_age_days_from_list
src/commands/spec/internal/archive.rs  — consolidate find_spec filesystem path
```

## Exit Criteria

- [ ] `spec_age_days_from_list()` reads from SpecInfo fields, not filesystem
- [ ] `show_ready_specs()` performs exactly one `get_all_specs()` call
- [ ] `find_spec()` filesystem fallback does one walk, not two
- [ ] No behavioral change — same output for all commands
