# Design: CAR Cleanup (non-A)

## Principle Alignment

- [[unix-philosophy]]: remove stale surfaces and deprecated flags.
- [[session-capture]] and [[spec-driven-design]]: archive stale specs using first-class lifecycle commands, not file deletes.

## Strategy

- Do cleanup as final pass after behavior and architecture are settled.
- Keep user-facing deprecations explicit in release notes/help text.

## Verification

- `cargo check --workspace -q`
- command help snapshots for touched commands
- `patina spec list` sanity after archive operations

## Out of Scope

- Any safety fixes, inversion fixes, or dead-code gate work.
