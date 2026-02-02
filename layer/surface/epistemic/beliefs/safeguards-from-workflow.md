---
type: belief
id: safeguards-from-workflow
persona: architect
facets: [workflow, safety, automation]
confidence:
  score: 0.90
entrenchment: medium
status: active
extracted: 2026-01-26
revised: 2026-01-26
---

# safeguards-from-workflow

Safeguards should be designed from actual workflow patterns, not theoretical best practices. Analyze real usage before defining checks.

## Statement

Before implementing safety checks or guards, analyze the actual workflow patterns from history (git logs, usage patterns, real behavior). Theoretical "best practices" often conflict with how work actually gets done. A safeguard that blocks normal workflow will be bypassed or disabled, defeating its purpose.

## Evidence

- session-20260126-074256: Analyzed git history before designing version safeguards. Found user is typically 30 commits ahead of remote - blocking on "ahead" would break normal workflow. (weight: 0.95)
- [[session-20260126-074256]]: Typical CI advice says "block if ahead of remote" but this user's pattern is commit-often-push-rarely. Theoretical advice was wrong for this workflow. (weight: 0.9)
- [[session-20260126-074256]]: Version safeguards designed from evidence: dirty tree (block), behind remote (block), ahead of remote (allow - it's normal). (weight: 0.9)

## Verification

```verify type="assay" label="Safeguard function exists in version command" expect=">= 1"
functions --pattern "safeguard"
```

```verify type="assay" label="Safeguard function is called" expect=">= 1"
callers --pattern "safeguard"
```

## Supports

- versioning-inference: Both beliefs emphasize deriving behavior from existing state rather than explicit configuration

## Attacks

<!-- Add beliefs this defeats -->

## Attacked-By

- Could argue theoretical best practices exist for good reasons (but they assume different workflows)

## Applied-In

- `patina version milestone` safeguard checks
- Decision to allow "ahead of remote" while blocking "behind remote"

## Revision Log

- 2026-01-26: Created (confidence: 0.90)
