---
type: refactor
id: greenfield-mother-clean-continued
status: draft
created: 2026-03-25
sessions:
  origin: 20260325-062526-943013000
exit_criteria: []
---
# refactor: Greenfield Mother: separate internal services from child registry

> Mother's code conflates internal services (secrets, sessions, health, specs, lakes) with the child abstraction. Native Rust structs masquerade as children via MotherChild trait and StaticChild markers, while the actual child registry should hold only WASM guests. This refactor draws a clean line: Mother owns internal services directly, ChildRegistry holds only KnowledgeChild WASM instances.

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
