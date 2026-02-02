---
type: belief
id: versioning-inference
persona: architect
facets: [architecture, versioning, project-config]
confidence:
  score: 0.80
  signals:
    evidence: 0.85
    source_reliability: 0.85
    recency: 0.95
    survival: 0.50
    user_endorsement: 0.85
entrenchment: medium
status: active
extracted: 2026-01-26
revised: 2026-01-26
---

# versioning-inference

Infer versioning behavior from upstream config. Don't add explicit flags when existing config already implies the answer.

## Statement

Whether `patina version milestone` should update Cargo.toml is inferred from `[upstream].remote` in config:
- `remote = "origin"` or no upstream section → owned repo → versioning enabled
- `remote = "upstream"` → fork/contrib → versioning disabled (upstream controls Cargo.toml)

This avoids redundant config flags. The upstream relationship already tells us who controls versions.

## Evidence

- session-20260126-074256: Discussed explicit `versioning.enabled` flag vs inference. User chose inference: "i think we should infer right?" (weight: 0.9)
- [[session-20260126-074256]]: Existing `[upstream]` config already distinguishes owned vs fork repos (weight: 0.85)
- [[session-20260126-074256]]: Implemented in [[src/project/internal.rs]] `is_versioning_enabled()` - checks `upstream.remote == "origin"` (weight: 0.9)

## Verification

```verify type="assay" label="is_versioning_enabled function exists" expect=">= 1"
functions --pattern "is_versioning_enabled"
```

```verify type="assay" label="is_versioning_enabled has callers" expect=">= 1"
callers --pattern "is_versioning_enabled"
```

## Supports

- milestones-in-specs: For forks, milestones track YOUR contribution goals even though Cargo.toml is upstream-controlled

## Attacks

<!-- Add beliefs this defeats -->

## Attacked-By

- Could argue explicit config is clearer than inference (but adds redundancy)

## Applied-In

- `src/project/internal.rs::is_versioning_enabled()`
- `patina version milestone` - skips Cargo.toml update for forks

## Revision Log

- 2026-01-26: Created (confidence: 0.80)
