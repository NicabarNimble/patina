# Belief Validation System

**Status**: Draft
**Type**: Feature
**Created**: 2026-01-21
**Author**: Session collaboration

## Problem Statement

The current epistemic belief system captures valuable project knowledge but relies on **aspirational signals** rather than **verifiable evidence**. Confidence scores are manually assigned by the LLM without validation, evidence links aren't verified to exist, and there's no computation to ensure consistency.

### Current State

```
LLM notices pattern → LLM picks confidence (0.85?) → create-belief.sh validates FORMAT only
```

The create-belief.sh script fakes signal values:
```bash
EVIDENCE_SIGNAL=$(printf "%.2f" $(echo "$CONFIDENCE + 0.05" | bc))
SURVIVAL_SIGNAL="0.50"  # Hardcoded
ENDORSEMENT_SIGNAL="0.50"  # Hardcoded
```

### Desired State

```
LLM proposes belief → System verifies evidence → System computes confidence → Human reviews
```

Confidence becomes traceable: "This belief has confidence 0.82 because it has 4 verified evidence links with avg similarity 0.78 and 2 supporting beliefs."

## Prior Art

### Prolog Neuro-Symbolic System (Nov 2025)

An earlier implementation attempted formal validation using embedded Scryer Prolog. Key files:
- `src/reasoning/engine.rs` - ReasoningEngine (447 lines)
- `src/reasoning/confidence-rules.pl` - Confidence calculation rules
- `src/reasoning/validation-rules.pl` - Belief validation logic

**What it got right:**
- Computed confidence from evidence count: `0.5 + (evidence_count * 0.15)`
- Strong evidence criteria: similarity >= 0.70 AND reliability >= 0.70
- Diverse sources requirement: 2+ source types
- Temporal decay after 180 days
- Contradiction detection

**Why it wasn't adopted:**
- Parallel architecture (separate storage) rather than integration
- Added Prolog dependency for minimal benefit
- Didn't leverage existing infrastructure (eventlog, scry, git)

### Current Infrastructure We Can Leverage

| Resource | Location | What It Provides |
|----------|----------|------------------|
| Sessions | `layer/sessions/*.md` | 509 files of evidence |
| Git tags | `session-*-start/end` | 800 session boundary markers |
| Scry | `patina scry` | Semantic search over all content |
| Patterns table | `.patina/local/data/patina.db` | Indexed beliefs and patterns |
| Wikilinks | `[[session-YYYYMMDD]]` | Graph of evidence relationships |

## Design

### Core Principle: Compute, Don't Assume

Every confidence signal should be **derived from verifiable data**, not assigned by judgment.

### Verification Layers

#### Layer 1: Link Verification

Verify that evidence links resolve to real files:

```bash
# Evidence link: [[session-20250804-073015]]
# Verify: layer/sessions/20250804-073015.md exists
```

**Signal produced:** `link_integrity_score = verified_links / claimed_links`

#### Layer 2: Content Verification

Verify that quoted evidence appears in the source:

```bash
# Claim: "Patina's workload is inherently synchronous" (from session-20250804)
# Verify: grep -qi "synchronous" layer/sessions/20250804-073015.md
```

**Signal produced:** `content_match_score = quotes_verified / quotes_claimed`

#### Layer 3: Semantic Verification (Scry)

Use retrieval to find supporting and contradicting evidence:

```bash
patina scry "$BELIEF_STATEMENT" --limit 10
# Results with score > 0.7 = strong support
# Results 0.5-0.7 = medium support
```

**Signals produced:**
- `semantic_support_score` = avg similarity of supporting results
- `contradiction_score` = max similarity to known contradicting beliefs

#### Layer 4: Graph Verification

Analyze the belief graph structure:

```
in_degree: How many beliefs cite this one in Supports
out_degree: How many evidence items this belief cites
attack_survival: Attacks marked as defeated
cluster_coherence: Do supporting beliefs form a consistent cluster
```

**Signals produced:**
- `graph_support_score = min(1.0, in_degree / 3)`
- `entrenchment_score` = function of attack_survival and in_degree

#### Layer 5: Temporal Verification

Track belief age and activity:

```bash
# When was belief created?
git log --format=%ai -- layer/surface/epistemic/beliefs/sync-first.md

# When was it last referenced in a session?
grep -l "sync-first" layer/sessions/*.md | xargs ls -t | head -1
```

**Signals produced:**
- `age_days` = days since belief creation
- `last_referenced_days` = days since last session mention
- `recency_score = max(0.5, 1.0 - (age_days / 365) * 0.2)`

### Computed Confidence Formula

```python
def compute_confidence(belief: Belief) -> float:
    # Layer 1: Link integrity (required - fail if < 1.0)
    link_score = verified_links / total_links
    if link_score < 1.0:
        return 0.0  # Broken links = invalid belief

    # Layer 2: Content verification
    content_score = quotes_verified / quotes_claimed

    # Layer 3: Semantic support
    scry_results = scry(belief.statement, limit=10)
    semantic_score = avg([r.score for r in scry_results if r.score > 0.5])

    # Layer 4: Graph support
    graph_score = min(1.0, belief.in_degree / 3)

    # Layer 5: Temporal decay
    age_days = (now - belief.created).days
    decay = max(0.5, 1.0 - (age_days / 365) * 0.2)

    # Weighted combination
    raw = (
        content_score * 0.25 +      # Evidence says what we claim
        semantic_score * 0.35 +      # Scry finds support
        graph_score * 0.25 +         # Other beliefs support this
        0.15                         # Base confidence for existing
    )

    return round(raw * decay, 2)
```

### Confidence Thresholds

| Range | Label | Interpretation |
|-------|-------|----------------|
| 0.90+ | Very High | Multiple verified sources, graph support, recent activity |
| 0.75-0.90 | High | Strong evidence, some graph support |
| 0.60-0.75 | Medium | Adequate evidence, new or limited support |
| 0.40-0.60 | Low | Weak evidence, needs validation |
| < 0.40 | Very Low | Insufficient evidence, consider archiving |

### Contradiction Handling

When creating a new belief:

1. Search for potential conflicts: `patina scry "contradicts: $STATEMENT"`
2. If high-similarity results found (> 0.7), flag for review
3. If contradiction confirmed:
   - One belief attacks the other
   - Lower-entrenchment belief may be scoped or defeated
   - Require explicit resolution before acceptance

## Implementation

### Phase 1: Validation Script (Audit Tool)

Add `validate-belief.sh` to verify existing beliefs without changing creation flow:

```bash
#!/bin/bash
# .claude/skills/epistemic-beliefs/scripts/validate-belief.sh

BELIEF=$1
ERRORS=0
WARNINGS=0

echo "=== Validating: $(basename $BELIEF) ==="

# Layer 1: Link verification
echo ""
echo "## Link Verification"
total_links=0
verified_links=0
while read link; do
    total_links=$((total_links + 1))
    file="layer/sessions/${link#session-}.md"
    if [ -f "$file" ]; then
        echo "  ✓ $link"
        verified_links=$((verified_links + 1))
    else
        echo "  ✗ $link MISSING"
        ERRORS=$((ERRORS + 1))
    fi
done < <(grep -oE 'session-[0-9]{8}-[0-9]{6}' "$BELIEF" | sort -u)

if [ $total_links -gt 0 ]; then
    link_score=$(echo "scale=2; $verified_links / $total_links" | bc)
    echo "  Link integrity: $link_score"
fi

# Layer 3: Semantic verification
echo ""
echo "## Semantic Verification"
statement=$(sed -n '/^## Statement/,/^## /p' "$BELIEF" | grep -v "^##" | head -3 | tr '\n' ' ')
if [ -n "$statement" ]; then
    echo "  Searching for: ${statement:0:60}..."
    patina scry "$statement" --limit 3 2>/dev/null | head -10
fi

# Layer 5: Temporal check
echo ""
echo "## Temporal Check"
created=$(grep "^extracted:" "$BELIEF" | cut -d: -f2 | tr -d ' ')
if [ -n "$created" ]; then
    age_days=$(( ($(date +%s) - $(date -j -f "%Y-%m-%d" "$created" +%s 2>/dev/null || echo $(date +%s))) / 86400 ))
    echo "  Age: $age_days days"
    if [ $age_days -gt 180 ]; then
        echo "  ⚠️  Belief older than 180 days - consider revalidation"
        WARNINGS=$((WARNINGS + 1))
    fi
fi

# Summary
echo ""
echo "=== Summary ==="
echo "  Errors: $ERRORS"
echo "  Warnings: $WARNINGS"
[ $ERRORS -eq 0 ] && echo "  Status: VALID" || echo "  Status: INVALID"

exit $ERRORS
```

### Phase 2: Computed Signals

Modify `create-belief.sh` to compute signals instead of faking them:

```bash
# Replace hardcoded signals with computed values
compute_evidence_signal() {
    local belief_file=$1
    local total=0
    local verified=0

    while read link; do
        total=$((total + 1))
        file="layer/sessions/${link#session-}.md"
        [ -f "$file" ] && verified=$((verified + 1))
    done < <(grep -oE 'session-[0-9]{8}-[0-9]{6}' "$belief_file" | sort -u)

    [ $total -eq 0 ] && echo "0.50" || echo "scale=2; $verified / $total" | bc
}

compute_recency_signal() {
    # New beliefs start at 0.95, decay based on evidence age
    echo "0.95"
}

compute_survival_signal() {
    # New beliefs start at 0.50 (unproven)
    echo "0.50"
}
```

### Phase 3: Pre-Creation Validation

Add guardrails before belief creation:

```bash
# In create-belief.sh, before writing file:

echo "=== Pre-creation validation ==="

# Check evidence exists
scry_results=$(patina scry "$STATEMENT" --limit 5 2>/dev/null)
strong_evidence=$(echo "$scry_results" | grep -c "score: 0\.[7-9]")

if [ "$strong_evidence" -lt 2 ]; then
    echo "⚠️  Weak evidence: only $strong_evidence strong results found"
    echo "    Recommended: find more supporting evidence before creating belief"
    read -p "    Continue anyway? [y/N] " confirm
    [ "$confirm" != "y" ] && exit 1
fi

# Check for contradictions
contradictions=$(patina scry "beliefs: $STATEMENT" --limit 5 2>/dev/null | grep -c "attacks\|contradicts")
if [ "$contradictions" -gt 0 ]; then
    echo "⚠️  Potential contradictions found"
    echo "$contradictions"
    read -p "    Review and continue? [y/N] " confirm
    [ "$confirm" != "y" ] && exit 1
fi

echo "✓ Pre-creation validation passed"
```

### Phase 4: Periodic Audit

Add belief health check command:

```bash
# .claude/skills/epistemic-beliefs/scripts/audit-beliefs.sh

echo "=== Epistemic Belief Audit ==="
echo "Date: $(date)"
echo ""

total=0
valid=0
stale=0
broken=0

for belief in layer/surface/epistemic/beliefs/*.md; do
    total=$((total + 1))
    result=$(./validate-belief.sh "$belief" 2>/dev/null)

    if echo "$result" | grep -q "Status: VALID"; then
        valid=$((valid + 1))
    else
        broken=$((broken + 1))
        echo "BROKEN: $(basename $belief)"
    fi

    if echo "$result" | grep -q "older than 180 days"; then
        stale=$((stale + 1))
    fi
done

echo ""
echo "=== Summary ==="
echo "Total beliefs: $total"
echo "Valid: $valid"
echo "Broken links: $broken"
echo "Stale (>180d): $stale"
echo ""
echo "Health score: $(echo "scale=2; $valid / $total * 100" | bc)%"
```

### Phase 5: Integration with Scry (Optional)

Surface belief confidence in search results:

```
patina scry "async vs sync" --include-beliefs

Results:
1. [belief] sync-first (confidence: 0.88, entrenchment: high)
   "Prefer synchronous, blocking code over async in Patina"

2. [session] 20250804-073015 (score: 0.82)
   "Realized Patina's workload is inherently synchronous..."
```

## Success Metrics

| Metric | Current | Target |
|--------|---------|--------|
| Beliefs with verified links | Unknown | 100% |
| Beliefs with computed confidence | 0% | 100% |
| Average content match score | Unknown | > 0.8 |
| Stale beliefs (>180d unreviewed) | Unknown | < 20% |
| Beliefs with contradiction flags | 0 | All conflicts flagged |

## Migration Path

1. **Audit existing beliefs** - Run validate-belief.sh on all 14 beliefs
2. **Fix broken links** - Update any that don't resolve
3. **Compute initial signals** - Replace fake signals with computed values
4. **Enable pre-creation validation** - Add guardrails to create-belief.sh
5. **Schedule periodic audits** - Monthly belief health check

## Open Questions

1. **Should invalid beliefs block creation or just warn?**
   - Recommendation: Warn with override option

2. **How to handle evidence in commits vs sessions?**
   - Could extend to verify commit SHAs exist in git history

3. **Should confidence auto-update or require explicit refresh?**
   - Recommendation: Compute on read, cache with TTL

4. **What's the minimum evidence threshold for acceptance?**
   - Prolog used: score >= 3.0, strong_count >= 2
   - Recommendation: At least 2 verified links with content matches

## References

- `src/reasoning/engine.rs` - Prior Prolog implementation (dead code)
- `src/reasoning/confidence-rules.pl` - Confidence calculation rules
- `src/reasoning/validation-rules.pl` - Validation logic
- `.claude/skills/epistemic-beliefs/` - Current belief creation skill
- `layer/surface/epistemic/VALIDATION.md` - Manual validation guide
