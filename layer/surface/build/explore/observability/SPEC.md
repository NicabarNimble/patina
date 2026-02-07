---
type: explore
id: observability
status: design
created: 2026-01-13
updated: 2026-02-06
tags: [observability, metrics, events, doctor]
related:
  - layer/surface/build/feat/doctor-dev/SPEC.md
---

# Explore: Command Observability

> Can command-level event logging help doctor-dev catch failure patterns?

## Core Question

Patina already logs retrieval events in the eventlog. Would logging command-level events (init, adapter, scrape, launch failures) give doctor-dev useful signal for catching problems before the user notices?

## What Exists

- **eventlog**: captures scry/retrieval events, forge events
- **session archives**: capture what happened during development sessions
- **git history**: captures what changed and when

## What's Missing

- No record of command failures (scrape errors, launch failures, adapter issues)
- No visibility into which commands run often vs never
- No pattern detection across sessions (e.g., "scrape always fails on this repo")

## Potential Value for doctor-dev

doctor-dev runs at session boundaries. If it could see:
- "scrape failed 3 times this week on repo X" → suggest fix
- "adapter refresh hasn't run in 60 days" → suggest refresh
- "launch_failed by reason" → identify common blockers

## Open Questions

- Is the eventlog the right place for command events, or a separate store?
- What's the minimum set of events that would be useful? (Not everything — just failures and key lifecycle events)
- Does doctor-dev need structured events, or would grep over session archives suffice?
- Single-user tool today — does observability only matter post-v1.0 with real users?

## References

- Extracted from spec-init-hardening (Phase 4, archived)
- doctor-dev: beads "deacon patrol" pattern at session boundaries
