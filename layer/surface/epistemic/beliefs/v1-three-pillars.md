---
type: belief
id: v1-three-pillars
persona: architect
facets: [architecture, roadmap, versioning]
confidence:
  score: 0.90
entrenchment: high
status: active
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

<!-- Add beliefs that challenge this -->

## Applied-In

- [[feat/v1-release/SPEC.md]]: Three-pillar structure and patch versioning

## Revision Log

- 2026-01-29: Created (confidence: 0.90)
