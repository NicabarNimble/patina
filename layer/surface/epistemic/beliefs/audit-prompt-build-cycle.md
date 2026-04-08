---
type: belief
id: audit-prompt-build-cycle
persona: architect
facets: [development-process, spec-driven-design, agent-orchestration]
confidence:
  score: 0.90
entrenchment: medium
status: active
extracted: 2026-04-07
revised: 2026-04-07
---

# audit-prompt-build-cycle

Specs are executed through a draft-audit-tighten-build cycle, not written once and thrown over the wall.

## Statement

The spec execution cycle is: draft spec, run audit agent, tighten from findings, run residual audit, lock decisions, write build agent prompt with concrete file targets, execute. Each pass hardens the spec against implementation churn. Skipping audit rounds produces specs that thrash during build.

## Evidence

- [[session-20260407-063612]]: [[spec-duckdb-version-pin]] — audit found crate versioning scheme gap, tightened, executed in one commit (weight: 0.95)
- [[session-20260407-063612]]: [[spec-mother-duckdb-ducklake-federation]] — two audit rounds (10 findings + 6 decisions + 5 residuals) transformed 9 vague criteria into 11 concrete criteria with HTTP schemas, failure matrix, telemetry contract (weight: 0.95)
- [[session-20260407-063612]]: pando-platform build agent prompt — modeled on [[spec-greenfield-mother-patina-data-platform]] commit pattern ([[commit-1ce8d60c]] through [[commit-f4cadfb2]]) (weight: 0.90)

## Supports

- [[spec-first]] — specs before code, but specs must be audit-hardened first
- [[spec-driven-design]] — every change traces to a spec; audit ensures the spec is traceable
- [[read-code-before-write]] — audit agents read code to find gaps the spec author missed
- [[truthful-specs]] — audit makes specs honest about what's actually specified vs hand-waved

## Attacks

- [[waterfall-spec]] (status: defeated, reason: this is iterative tightening, not big-design-upfront; each audit round is minutes, not weeks)

## Attacked-By

- [[time-pressure]] (status: active, confidence: 0.3, scope: "trivial fixes don't need full audit cycle — use judgment on spec size")

## Applied-In

- `duckdb-version-pin`: spec rewritten from blocked/wrong to 3 correct criteria, executed same session
- `mother-duckdb-ducklake-federation`: 9→11 criteria, locked 16 decisions (10+6), fixed 5 residuals before build agent prompt
- Build agent prompts reference greenfield commit sequence ([[commit-1ce8d60c]]) as the execution pattern

## Revision Log

- 2026-04-07: Created from session-20260407-063612-748374000 (confidence: 0.90)
