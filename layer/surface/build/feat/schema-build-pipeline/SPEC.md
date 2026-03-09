---
type: feat
id: schema-build-pipeline
status: ready
created: 2026-03-09
sessions:
  origin: 20260309-090701
related:
- connector-owns-tables
exit_criteria:
- id: broker-rejects-facts-when-installed-schema-is-missing-fail-closed
  text: Broker rejects facts when installed schema is missing (fail closed)
  checked: false
- id: broker-rejects-facts-with-fact-type-not-declared-in-installed-schema
  text: Broker rejects facts with fact_type not declared in installed schema
  checked: false
- id: ci-check-validates-canonical-schema-matches-installed-copy-full-directory-not-just-toml
  text: CI check validates canonical schema matches installed copy (full directory, not just TOML)
  checked: false
- id: patina-schema-build-name-orchestrates-validate-install-with-optional-generate
  text: patina schema build <name> orchestrates validate→install, with optional generate
  checked: false
- id: src-generated-schemas-removed-or-regenerated-from-installed-schemas-not-forge-only
  text: src/generated/schemas/ removed or regenerated from installed schemas (not forge-only)
  checked: false
---
# feat: Schema Build Pipeline — Single Source, Runtime Validation, CI Enforcement

> wit/schema/<name>/ is the canonical schema source but there is no validation pipeline: broker doesn't check fact_type against installed schemas, no CI enforces canonical-to-installed drift, and there is no orchestration command for the validate→install→generate flow.

## Problem

After connector-owns-tables, `wit/schema/<name>/schema.toml` is the canonical
schema source and `.patina/schemas/<name>/` is the installed runtime copy. But
the pipeline between them has gaps:

1. **No runtime fact-type validation.** `src/broker/routing.rs:50` checks that
   the schema *name* is declared in child.toml but not that the `fact_type`
   exists in the installed schema. A child emitting `github.typo` passes
   routing silently.

2. **No CI drift enforcement.** Nothing prevents the canonical source from
   diverging from the installed copy. The connector-local duplicate drifted
   exactly this way (fixed in ebdc7bcb).

3. **No orchestration command.** Today the flow is manual: `schema install`
   then `schema generate`. No single command validates, installs, and
   regenerates.

4. **Dead generated code.** `src/generated/schemas/` contains forge-only
   types that are not imported anywhere.

## Solution

Phased, in priority order:

### Phase 1: Broker fact-type validation
- Fail closed: if schema is declared in manifest but not installed, reject facts.
- Validate `fact_type` against installed schema `facts[].event_type`.
- One validation entry point: `routing::validate_fact()` with internal caching.
- Files: `src/broker/routing.rs`, `src/broker/mod.rs`

### Phase 2: CI drift checks
- Add to `pre-push-checks.sh`:
  - Diff entire installed schema directory against canonical (TOML + WIT)
  - Connector manifest `package` version matches canonical `schema.package`
- Files: `resources/git/pre-push-checks.sh`

### Phase 3: `patina schema build <name>`
- Thin orchestration wrapper: validate → install, with optional generate
- Replaces manual two-step flow
- Files: `src/commands/schema/mod.rs`, `src/commands/schema/internal.rs`

### Phase 4: Generated schema cleanup
- Confirm `src/generated/schemas/` is dead code (no imports)
- Either delete it or regenerate from installed schemas (github + forge)
- Files: `src/generated/schemas/`

## Exit Criteria

1. Broker rejects facts when installed schema is missing (fail closed)
2. Broker rejects facts with fact_type not declared in installed schema
3. CI check validates canonical schema matches installed copy (full directory)
4. `patina schema build <name>` orchestrates validate→install, with optional generate
5. `src/generated/schemas/` removed or regenerated from installed schemas (not forge-only)

## Non-Goals

- New top-level `schemas/` directory (keep `wit/schema/<name>/`)
- Schema versioning or migration (pre-v1, single-user)
- Auto-install on `mother run` (explicit install is the right UX for now)
