---
type: belief
id: errors-at-transactions-are-not-ignorable
persona: architect
facets: [rust, error-handling, database, correctness]
entrenchment: high
status: active
endorsed: true
extracted: 2026-03-01
revised: 2026-03-01
---

# errors-at-transactions-are-not-ignorable

`let _ = conn.execute("ROLLBACK", [])` is a correctness bug, not a stylistic choice. If ROLLBACK fails, the connection's transaction state is undefined — subsequent operations may silently commit partial writes or fail in unpredictable ways. Transaction boundary errors must be propagated or logged with the original error context.

## Statement

`let _ = conn.execute("ROLLBACK", [])` is a correctness bug, not a stylistic choice. If ROLLBACK fails, the connection's transaction state is undefined — subsequent operations may silently commit partial writes or fail in unpredictable ways. Transaction boundary errors must be propagated or logged with the original error context.

## Evidence

- [[session-20260301-165723]]: Structural audit found 12 instances of `let _ = conn.execute("ROLLBACK", [])` in `mother/graph.rs`, covering every belief sync transaction. If any ROLLBACK fails, the original error is returned but the connection state is undefined. (weight: 0.95)
- [[session-20260301-165723]]: Additional 68 `let _ =` patterns found across codebase discarding Results, including file operations in `forge/sync/internal.rs` (PID file writes) and `repo/internal.rs`. (weight: 0.8)

## Supports

- [[question-mark-on-option-is-silent-swallower]] — same family of silent error erasure

## Attacks

<!-- None known -->

## Attacked-By

- "What can you do if ROLLBACK fails?" — at minimum, log the compound error (original + rollback failure) so the state inconsistency is visible in diagnostics

## Applied-In

<!-- Not yet applied — requires spec work -->

## Revision Log

- 2026-03-01: Created from structural audit findings
