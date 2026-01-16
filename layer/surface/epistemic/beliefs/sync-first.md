---
type: belief
id: sync-first
persona: architect
facets: [rust, architecture, simplicity]
confidence:
  score: 0.88
  signals:
    evidence: 0.92
    source_reliability: 0.90
    recency: 0.75
    survival: 0.95
    user_endorsement: 0.85
entrenchment: high
status: active
extracted: 2025-08-04
revised: 2026-01-16
---

# sync-first

Prefer synchronous, blocking code over async in Patina.

## Statement

Use synchronous, blocking code by default. Async adds complexity (infects codebase with 'static lifetimes, complicates borrow checker) without benefit when the workload is inherently synchronous (local file I/O, SQLite queries).

## Evidence

- [[session-20250804-073015]] - "Patina's workload is inherently synchronous" (weight: 0.95)
- [[session-20250804-073015]] - "Async infects codebase with 'static lifetimes" (weight: 0.90)
- [[session-20250804-073015]] - "Borrow checker works best without async runtime complexity" (weight: 0.85)
- [[session-20250730-065949]] - "Chose blocking reqwest client for simplicity in CLI context" (weight: 0.80)

## Supports

- [[simple-error-handling]]
- [[local-first]]
- [[dependable-rust]] - borrow checker works better without async

## Attacks

- [[async-by-default]] (status: defeated, reason: consider actual I/O patterns first)
- [[rqlite-architecture]] (status: defeated, reason: migrated to SQLite)

## Attacked-By

- [[high-concurrency-needed]] (status: active, confidence: 0.3, scope: "if network-heavy or many parallel connections")
- [[streaming-responses]] (status: active, confidence: 0.25, scope: "long-running streaming APIs")

## Context

The async decision came from analyzing Patina's actual workload:
- Local file I/O (reading code, sessions, patterns)
- SQLite queries (single-threaded is fine)
- No high-concurrency network requirements

Async was originally introduced for rqlite (network-based database), but when migrating to SQLite, async became unnecessary complexity.

## Applied-In

- reqwest blocking client instead of async
- rusqlite instead of async SQLite wrappers
- Standard threads for background work, not tokio tasks

## Revision Log

- 2025-08-04: Decided during SQLite migration (confidence: 0.80)
- 2025-08-04: Removed async entirely from codebase (confidence: 0.80 → 0.85)
- 2026-01-16: Added to epistemic layer, high survival (confidence: 0.85 → 0.88)
