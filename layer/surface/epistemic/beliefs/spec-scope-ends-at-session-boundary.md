---
type: belief
id: spec-scope-ends-at-session-boundary
persona: architect
facets: [process, spec-lifecycle]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-02-14
revised: 2026-02-14
---

# spec-scope-ends-at-session-boundary

Post-completion review fixes within the same session (before push) are in-scope of the original spec; once pushed or session-ended, any further changes require their own spec.

## Statement

Post-completion review fixes within the same session (before push) are in-scope of the original spec; once pushed or session-ended, any further changes require their own spec.

## Evidence

- [[session-20260214-091805]]: [[spec-complete-archives]] review fix ([[3dc7891a]]) landed after spec was completed/archived but before session end or push. Exposed the need for a clear boundary. (weight: 0.9)

## Supports

- [[read-code-before-write]] — traceability requires knowing what spec drove the change

## Attacked-By

- Overhead concern: tiny fixes (typos, review nits) may not justify a full spec. Counterpoint: a one-line spec is still a spec — the overhead is in the tracking, not the document size.

## Applied-In

- [[3dc7891a]] — review fix committed in same session as [[spec-complete-archives]] completion, before push. Acceptable under this rule.
- Future: any post-push fix to a completed spec should get its own `fix:` spec.

## Revision Log

- 2026-02-14: Created — metrics computed by `patina scrape`
