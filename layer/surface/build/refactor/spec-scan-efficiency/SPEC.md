---
type: refactor
id: spec-scan-efficiency
status: draft
created: 2026-02-25
sessions:
  origin: 20260225-104204
exit_criteria: []
---
# refactor: Eliminate redundant filesystem scans in spec queries

> scan_disk_specs and spec_age_days_from_list re-read files per call — O(n) file reads in display loops

## Current State

## Target State

## Steps

## Exit Criteria
