# Topic 2: Session Extraction Quality Analysis

**Date**: 2025-11-17
**Session**: 20251117-132649

## Observation Sources Comparison

### session_distillation (60 observations, reliability 0.85)
**Characteristics**:
- Single-word or short phrases ("neuro-symbolic-persona", "domain-buckets")
- Tag-like, minimal context
- Technologies: "tool: brief description" format
- No actionable insight or reasoning

**Examples**:
- "neuro-symbolic-persona" (pattern)
- "tmpfs-for-secrets" (pattern)
- "github-actions: CI/CD automation" (technology)
- "markdown-is-generated-output: canonical data in facts.db + rules.pl" (decision)

**Assessment**: Low quality, shallow extraction

### session (40 observations, reliability 0.85-1.0)
**Characteristics**:
- Full sentences with context
- Rich explanations and reasoning
- Actionable insights
- Clear decision rationale

**Examples**:
- "Build core value proposition (Ingest → Structure → Retrieve) before optimizing for performance" (decision, 0.95)
- "SQLite Connection uses RefCell internally and is not Sync - cannot be shared across threads with Arc<RwLock>" (challenge, 1.0)
- "Ask 'Are we solving the wrong problem?' when hitting unexpected technical constraints - may indicate invented requirements" (pattern, 0.9)

**Assessment**: High quality, manual curation

## Quality Difference

| Metric | session_distillation | session (manual) |
|--------|---------------------|------------------|
| Average length | 3-5 words | 15-25 words |
| Context | None | Rich |
| Actionable | No | Yes |
| Reliability | 0.85 | 0.85-1.0 |
| Extraction | Automated (legacy) | Manual curation |

## Hypothesis

**session_distillation** appears to be an early automated extraction experiment:
- Extracted keywords/tags from sessions
- Minimal processing (just pattern names)
- Low information density
- Filtered out by strict threshold (> 0.85)

**session** is manual curation from Topic 0:
- Hand-written observations from sessions
- Full context and reasoning
- High information density
- Included in filtered results (reliability 0.9-1.0)

## Next Steps

1. Fix filter threshold (> 0.85 → >= 0.85) to include session_distillation
2. Test retrieval quality with session_distillation included
3. Determine if session_distillation adds value or creates noise
4. Recommend purge or upgrade reliability threshold
