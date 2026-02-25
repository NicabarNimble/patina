---
type: belief
id: adding-type-is-not-migrating-model
persona: architect
facets: [architecture, rust, refactoring]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-02-24
revised: 2026-02-24
---

# adding-type-is-not-migrating-model

Adding a thin enum alongside existing string fields is additive; changing the field type from String to the enum is a data model migration — these are different scopes and should be separate specs

## Statement

Adding a thin enum alongside existing string fields is additive; changing the field type from String to the enum is a data model migration — these are different scopes and should be separate specs

## Evidence

- [[session-20260224-195035]]: spec-create review discovered SpecType enum (additive, ~30 lines, zero churn) vs SpecFrontmatter.r#type migration (touches every consumer, serde contract, YAML files) — conflating them caused incorrect scope analysis (weight: 0.9)

## Supports

<!-- Add beliefs this supports -->

## Attacks

<!-- Add beliefs this defeats -->

## Attacked-By

<!-- Add beliefs that challenge this -->

## Applied-In

<!-- Add concrete applications -->

## Revision Log

- 2026-02-24: Created — metrics computed by `patina scrape`
