---
type: refactor
id: mother-broker-github
status: active
created: 2026-03-07
sessions:
  origin: 20260307-234302
related:
- pipe-architecture
- github-child-owns-forge
- schema-driven-projection
exit_criteria:
- id: mother-run-github
  text: '`patina mother run github` spawns github-connector, routes github.* facts to project events.db with content-hash dedup'
  checked: true
- id: github-events-searchable
  text: github.issue and github.pr events are projected into materialized views and searchable via scry/assay
  checked: true
split_from: mother-broker
---
# refactor: Mother Broker GitHub — End-to-End GitHub Connector Verification

> Verify that `patina mother run github` works end-to-end: github-connector fetches issues/PRs, broker routes facts to events.db, projection makes them searchable.

## Context

Split from [[spec-mother-broker]] after all other broker ECs were verified in session 20260307-165002. This EC was blocked by [[spec-github-connector]] (the child binary).

Parent spec content: `git show spec/mother-broker-v1-complete:layer/surface/build/refactor/mother-broker/SPEC.md`

## Verification (session 20260307-234302)

**mother-run-github:** 102 facts from `child:github-connector` in events.db (17 `github.issue` + 88 `github.pr`). All have content_hash (dedup works). Source confirmed as `child:github-connector`, not `gh` CLI.

**github-events-searchable:** Fixed projection gap in [[spec-github-child-owns-forge]] Phase 1. `project_from_events()` now handles both `forge.*` and `github.*` event types. Verified: `patina scrape` projects 17 issues + 87 PRs, `patina scry --include-issues` and `patina assay search --include-issues` return results.

**Note:** The hardcoded event type approach will be replaced by schema-driven projection ([[spec-schema-driven-projection]]) — new connectors (gitea, gitlab) should not require core code changes.

## Exit Criteria

- **mother-run-github:** `patina mother run github` routes github.* facts to events.db with content-hash dedup
- **github-events-searchable:** github events are projected and searchable via scry/assay
