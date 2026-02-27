---
type: belief
id: question-mark-on-option-is-silent-swallower
persona: architect
facets: [rust, code-review, error-handling]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-02-27
revised: 2026-02-27
---

# question-mark-on-option-is-silent-swallower

The ? operator on Option in a function returning Option silently returns None — it swallows the entire function, not just the missing value. Code review should grep for this pattern.

## Statement

The ? operator on Option in a function returning Option silently returns None — it swallows the entire function, not just the missing value. Code review should grep for this pattern.

## Evidence

- [[session-20260227-095804]]: Found twice in scry logging: CLI path (gap 7, [[commit-471c39d0]]) and MCP path ([[commit-2a0fbf31]]). Both times caused events to be silently dropped when no session was active. (weight: 0.95)
- [[session-20260227-093437]]: Original implementation session noted "The `?` operator on `Option` in a function returning `Option` is a silent swallower — it returns `None` from the function, not an error." (weight: 0.8)

## Supports

- [[events-are-autobiography-not-telemetry]] — silent event loss violates the autobiography principle

## Attacks

<!-- Add beliefs this defeats -->

## Attacked-By

<!-- Add beliefs that challenge this -->

## Applied-In

- `src/commands/scry/internal/logging.rs:88` — removed `?` from `get_active_session_id()` in `log_scry_query()` and `log_scry_query_with_routing()`
- `src/mcp/server/scry.rs:371` — removed `?` from `get_active_session_id()` in `log_mcp_query()`

## Revision Log

- 2026-02-27: Created — metrics computed by `patina scrape`
