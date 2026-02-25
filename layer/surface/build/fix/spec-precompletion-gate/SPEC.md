---
type: fix
id: spec-precompletion-gate
status: active
created: 2026-02-25
blocked_by:
- spec-structured-exit-criteria
sessions:
  origin: 20260224-212321
beliefs:
- spec-is-contract
- compiler-enforced-safety
- safeguards-from-workflow
exit_criteria: []
---
# fix: Add spec check pre-completion validation

> No programmatic gate between self-certification and release; spec.complete trusts the LLM

## Problem

`spec.complete` checks `status == "active"` and triggers an irreversible
release + archive. There is no validation that exit criteria are met,
that verification commands pass, or that the spec's claims match reality.

Session 20260206-060219 found this in practice: manual audits discovered
specs completed with unchecked exit criteria — "done but not closed",
"measurement blocking implementation", "accumulated scope never shipped."

The system has 21 beliefs about spec governance, all normative. Zero
programmatic enforcement. Completion is self-certification.

## Root Cause

`complete_spec_value()` in `mutations.rs` validates status and nothing
else. No function exists to inspect exit criteria state. The spec system
was built governance-first (state machine) without verification gates.

## Fix

Add `patina spec check <id>` command:

1. Parse `exit_criteria` from frontmatter (requires spec-structured-exit-criteria)
2. Report: total criteria, checked count, unchecked list
3. Run `verify` commands for criteria that have them (optional, --verify flag)
4. Return pass/fail with details

Integration with `spec.complete`:
- `complete_spec_value()` calls `check_spec()` before proceeding
- If any exit criteria are unchecked, fail with list of what's missing
- Add `--force` flag to bypass (for legitimate "close without full completion" cases)

```
$ patina spec check my-spec
Exit criteria: 3/4 complete
  ✓ rollback-db
  ✓ rollback-abandon
  ✗ simulated-failure — not checked
  ✓ existing-behavior

Cannot complete: 1 unchecked criterion
```

## Key Files

```
src/commands/spec/internal/mutations.rs  — complete_spec_value (add gate)
src/commands/spec/internal/queries.rs    — new check_spec_value()
src/commands/spec/mod.rs                 — new Check subcommand
src/mcp/server.rs                        — new spec.check tool
```

## Exit Criteria

- [ ] `patina spec check <id>` reports exit criteria status
- [ ] `spec.check` MCP tool returns structured pass/fail
- [ ] `spec.complete` fails if unchecked exit criteria exist
- [ ] `spec.complete --force` bypasses the gate
- [ ] Specs without exit_criteria field pass by default (backward compatible)
