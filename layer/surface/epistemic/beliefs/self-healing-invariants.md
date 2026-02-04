---
type: belief
id: self-healing-invariants
persona: architect
facets: [architecture, resilience]
confidence:
  score: 0.85
entrenchment: medium
status: active
extracted: 2026-01-21
revised: 2026-01-21
---

# self-healing-invariants

Prefer self-healing invariants over fail-fast guards - operations that need a precondition should ensure it exists rather than failing when it doesn't

## Statement

Prefer self-healing invariants over fail-fast guards - operations that need a precondition should ensure it exists rather than failing when it doesn't

## Evidence

- session-20260121-102727: database-identity Phase 1 chose auto-create UIDs over fail guards (weight: 0.9)

## Verification

```verify type="assay" label="migration functions exist" expect=">= 2"
functions --pattern "migration"
```

```verify type="assay" label="create_uid functions exist" expect=">= 1"
functions --pattern "create_uid"
```

## Supports

<!-- Add beliefs this supports -->

## Attacks

<!-- Add beliefs this defeats -->

## Attacked-By

<!-- Add beliefs that challenge this -->

## Applied-In

- **UID auto-creation**: `create_uid_if_missing()` called at scrape/rebuild/update entry points ensures UIDs exist rather than failing
- **Preflight stale process killer**: Auto-kills processes older than 24h rather than failing on port conflict
- **Config migration**: `load_with_migration()` silently upgrades config.json → config.toml

## Revision Log

- 2026-01-21: Created (confidence: 0.85)
