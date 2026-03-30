---
type: refactor
id: mother-database-consolidation
status: draft
created: 2026-03-30
sessions:
  origin: 20260330-064829-699906000
related:
- mother/src/registry.rs,src/eventlog.rs,mother/src/state.rs,src/child/internal/host_support.rs
exit_criteria: []
---
# refactor: Mother owns databases, projects stay plain text

> Consolidate database ownership in Mother. Projects keep only git-tracked layer/ and minimal .patina/ config. Mother holds per-project events, child runtime state, and scrape projections. Working-copy-specific caches (embeddings, FTS5) remain project-local as rebuildable derived state.

## Problem

## Goal

## Status

## Non-Goals

## Current State

## Target State

## Solution

## Implementation Order

## Resolved Decisions

## Verification

## Exit Criteria

## Build Readiness
