---
type: fix
id: spec-structured-exit-criteria
status: ready
created: 2026-02-25
sessions:
  origin: 20260224-212321
related:
- spec-precompletion-gate
beliefs:
- spec-is-contract
- specs-require-zero-ambiguity
exit_criteria: []
---
# fix: Structured exit criteria in spec frontmatter

> Exit criteria are unenforced markdown checkboxes; spec.complete only checks status==active

## Problem

Exit criteria live in the SPEC.md markdown body as freeform checkbox lists.
No tooling can parse them, count them, or verify their state. Session
20260206-060219 found multiple specs completed with unchecked exit
criteria — requiring manual audits to catch. The LLM self-certifies
completion with no structural verification.

## Root Cause

Exit criteria were designed as human-readable documentation, not
machine-readable contracts. The spec frontmatter (`SpecFrontmatter` in
`src/spec.rs`) has 22 structured fields for metadata but exit criteria
are buried in the unstructured body text. `spec.complete` validates
`status == "active"` and nothing else.

## Fix

Add an `exit_criteria` field to `SpecFrontmatter`:

```yaml
exit_criteria:
  - id: rollback-db
    text: "complete_spec_value rolls back DB status on failure"
    checked: false
  - id: simulated-failure
    text: "Simulated failure leaves DB status unchanged"
    checked: false
    verify: "patina spec complete <id> with dirty tree; check DB"
```

Each criterion has:
- `id` — stable reference for programmatic use
- `text` — human description
- `checked` — boolean, updated when criterion is met
- `verify` (optional) — command or instruction to validate

Parse this field in `parse_spec_file`. Serialize it back via
`serialize_spec_file`. Existing markdown checkboxes in the body remain
for human readability but are no longer the source of truth.

## Key Files

```
src/spec.rs                              — SpecFrontmatter struct, parse/serialize
src/commands/spec/internal/mutations.rs  — mutate_spec (frontmatter writes)
src/commands/spec/internal/create.rs     — body templates
```

## Exit Criteria

- [ ] `SpecFrontmatter` has `exit_criteria: Vec<ExitCriterion>` field
- [ ] `ExitCriterion` struct: id, text, checked, verify (optional)
- [ ] `parse_spec_file` round-trips exit_criteria without loss
- [ ] `spec create` body template includes `exit_criteria:` in frontmatter
- [ ] Existing specs without the field parse without error (backward compatible)
