---
type: belief
id: decouple-before-extract
persona: architect
facets: [architecture, plugins, refactoring]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-02-14
revised: 2026-02-14
---

# decouple-before-extract

Coupled modules must be decoupled before independent extraction — when two modules have different extraction timelines but tight coupling, decoupling first (each command does one job) makes both independently extractable.

## Statement

Coupled modules must be decoupled before independent extraction — when two modules have different extraction timelines but tight coupling, decoupling first (each command does one job) makes both independently extractable.

## Evidence

- [[session-20260214-110957]]: [[plugin-extraction-map]] Section 10 — spec→release coupling in src/commands/spec/internal.rs (lines 428-457) blocks independent extraction of either module; code-level decoupling plan documented (weight: 0.9)

## Supports

- [[unix-philosophy]] — one tool, one job; decoupling enforces single responsibility before extraction makes the boundary permanent
- [[graceful-extraction]] — formats must stabilize before extraction; coupled modules can't stabilize independently

## Attacks

<!-- None identified -->

## Attacked-By

- "Extract together" alternative: extract both coupled modules as a single plugin, preserving internal coupling. Counter: violates [[separate-worlds-for-isolation]] — spec (lifecycle) and release (versioning) are different responsibilities that should be independently replaceable by the community.
- YAGNI: decoupling costs effort now for extraction that may be far off. Counter: the decoupling is valuable regardless — `spec status complete` silently bumping versions is a confusing side-effect that violates unix-philosophy.

## Applied-In

- [[plugin-extraction-map]] Section 10 — spec→release coupling documented with line-level code references and decoupling plan
- `src/commands/spec/internal.rs:428-457` — the coupling site: `update_spec_status()` calls `ReleaseStrategy::from_project()` → `preflight()` → `execute()`

## Revision Log

- 2026-02-14: Created — metrics computed by `patina scrape`
