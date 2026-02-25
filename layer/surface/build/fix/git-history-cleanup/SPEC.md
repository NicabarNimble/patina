---
type: fix
id: git-history-cleanup
status: draft
created: 2026-02-25
sessions:
  origin: 20260225-143514
exit_criteria: []
---
# fix: Strip historical binary blobs from git history

> 714 MB .git directory dominated by binary blobs that were committed and later deleted — ONNX models (90MB), libduckdb (72MB+), grammar build artifacts. Strip them with git filter-repo to reclaim ~50% of .git size.

## Problem

## Root Cause

## Fix

## Exit Criteria
