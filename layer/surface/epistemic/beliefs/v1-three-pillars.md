---
type: belief
id: v1-three-pillars
persona: architect
facets: [architecture, roadmap, versioning]
confidence:
  score: 0.90
entrenchment: high
status: defeated
extracted: 2026-01-29
revised: 2026-01-29
---

# v1-three-pillars

v1.0 requires finalizing exactly three pillars: epistemic layer (beliefs), mother (federated query), and distribution (modular binary). No architectural rewrites after 1.0.

## Statement

v1.0 requires finalizing exactly three pillars: epistemic layer (beliefs), mother (federated query), and distribution (modular binary). No architectural rewrites after 1.0.

## Evidence

- session-20260129-074742: Crystallized v1.0 focus from discussion of specs, versioning, and distribution. User confirmed these three as THE dependencies for 1.0. (weight: 0.95)
- session-20260127-085434: Distribution architecture emerged from crates.io blocker (60MB grammars). WASM + dynamic ONNX chosen. (weight: 0.85)
- [[session-20260129-074742]]: [[spec-epistemic-layer]] E0-E3 complete, 35 beliefs indexed. E4 (automation) identified as remaining work. (weight: 0.80)
- [[session-20260129-074742]]: [[spec-mother]] Federated query and persona fusion identified as remaining work. (weight: 0.80)

## Verification

```verify type="sql" label="Epistemic layer spec exists" expect=">= 1"
SELECT COUNT(*) FROM git_tracked_files WHERE file_path LIKE '%feat/epistemic-layer/SPEC.md'
```

```verify type="sql" label="Mother spec exists" expect=">= 1"
SELECT COUNT(*) FROM git_tracked_files WHERE file_path LIKE '%feat/mother/SPEC.md'
```

```verify type="sql" label="V1 release spec exists" expect=">= 1"
SELECT COUNT(*) FROM git_tracked_files WHERE file_path LIKE '%feat/v1-release/SPEC.md'
```

## Supports

- [[specs-source-of-truth]]: Specs drive the roadmap, v1.0 pillars are spec-defined
- [[phased-development-with-measurement]]: Patch versions (0.9.x) enable measured iteration

## Attacks

<!-- Add beliefs this defeats -->

## Attacked-By

- [[patina-is-beliefs-plus-action]] — The domain-agnostic pivot changed Patina's identity. v1 of a "domain-agnostic knowledge system" is a different product than v1 of a "development tool."
- [[patina-is-domain-agnostic-knowledge-system]] — All three pillar definitions predate this pivot: "epistemic layer" is now beliefs+action, "mother" is now federation+continuity+lakes, "distribution" is now plugin architecture/protocol distillation.
- [[wit-is-contract-wasm-is-one-runtime]] — The "distribution (modular binary)" pillar was about crates.io size. Plugin architecture is now a much larger concern — 4 roles, WIT contracts, core extraction.

## Defeated

- **Date**: 2026-03-04
- **Reason**: Predates the domain-agnostic pivot (created 2026-01-29). All three pillar definitions are outdated and revising would replace the entire substance. The governance principle ("define v1 gates before building") is sound but should be re-derived fresh from the current architecture when the spec landscape stabilizes.

## Applied-In

- [[feat/v1-release/SPEC.md]]: Three-pillar structure and patch versioning

## Revision Log

- 2026-01-29: Created (confidence: 0.90)
- 2026-03-04: Defeated — predates domain-agnostic pivot, all three pillar definitions outdated. New v1 definition to be derived from current architecture.
