# Belief Validation System - Design Details

## Architecture

### Current Flow (No Validation)

```
┌─────────────┐     ┌──────────────────┐     ┌─────────────────┐
│ LLM notices │ ──▶ │ create-belief.sh │ ──▶ │ belief file     │
│ pattern     │     │ (format only)    │     │ in layer/       │
└─────────────┘     └──────────────────┘     └─────────────────┘
                           │
                           ▼
                    Fake signals:
                    evidence: CONF + 0.05
                    survival: 0.50
                    endorsement: 0.50
```

### Proposed Flow (With Validation)

```
┌─────────────┐     ┌──────────────────┐     ┌─────────────────┐
│ LLM notices │ ──▶ │ Pre-validation   │ ──▶ │ create-belief.sh│
│ pattern     │     │ (scry + links)   │     │ (compute sigs)  │
└─────────────┘     └──────────────────┘     └─────────────────┘
                           │                         │
                           ▼                         ▼
                    ┌──────────────┐         ┌─────────────────┐
                    │ Warn if weak │         │ Computed signals│
                    │ evidence     │         │ from real data  │
                    └──────────────┘         └─────────────────┘
```

## Data Model

### Verifiable Signals Schema

```yaml
confidence:
  score: 0.82                    # Computed, not assigned
  computed_at: 2026-01-21T12:00:00Z
  signals:
    # Layer 1: Link integrity
    link_integrity:
      value: 1.0
      verified: 4
      total: 4

    # Layer 2: Content matching
    content_match:
      value: 0.75
      verified: 3
      total: 4
      unverified:
        - "exact quote not found in session-20250730"

    # Layer 3: Semantic support
    semantic_support:
      value: 0.78
      query: "prefer synchronous blocking code async"
      top_results:
        - doc: "session-20250804-073015"
          score: 0.89
        - doc: "session-20250730-065949"
          score: 0.76
        - doc: "layer/core/sync-first.md"
          score: 0.71

    # Layer 4: Graph support
    graph_support:
      value: 0.67
      in_degree: 2          # beliefs that cite this
      out_degree: 4         # evidence items
      supporting_beliefs:
        - simple-error-handling
        - local-first
      attacks_survived: 2

    # Layer 5: Temporal
    temporal:
      value: 0.95
      created: 2025-08-04
      age_days: 170
      last_referenced: 2026-01-15
      days_since_reference: 6
```

### Validation Result Schema

```yaml
validation:
  status: valid | warning | invalid
  timestamp: 2026-01-21T12:00:00Z
  checks:
    - name: link_integrity
      passed: true
      details: "4/4 links verified"

    - name: content_match
      passed: true
      details: "3/4 quotes found"
      warnings:
        - "Quote not found: 'exact text' in session-20250730"

    - name: semantic_support
      passed: true
      details: "Avg score 0.78 from 5 results"

    - name: contradiction_check
      passed: true
      details: "No high-similarity contradictions found"

    - name: temporal_health
      passed: true
      details: "Age 170 days, referenced 6 days ago"
      warnings:
        - "Approaching 180-day review threshold"
```

## Implementation Details

### Link Verification Algorithm

```bash
verify_links() {
    local belief_file=$1
    local results=()

    # Extract all wikilinks
    links=$(grep -oE '\[\[[^\]]+\]\]' "$belief_file")

    for link in $links; do
        # Strip [[ and ]]
        link_content="${link:2:-2}"

        # Handle different link types
        case "$link_content" in
            session-*)
                # Session link: [[session-20250804-073015]]
                session_id="${link_content#session-}"
                file="layer/sessions/${session_id}.md"
                ;;
            commit-*)
                # Commit link: [[commit-abc123]]
                sha="${link_content#commit-}"
                # Verify commit exists
                git cat-file -t "$sha" &>/dev/null && status="valid" || status="invalid"
                ;;
            *)
                # Belief link: [[sync-first]]
                file="layer/surface/epistemic/beliefs/${link_content}.md"
                ;;
        esac

        if [ -f "$file" ]; then
            echo "verified:$link_content"
        else
            echo "missing:$link_content"
        fi
    done
}
```

### Content Verification Algorithm

```bash
verify_content() {
    local belief_file=$1
    local session_file=$2
    local quote=$3

    # Normalize quote (lowercase, collapse whitespace)
    normalized=$(echo "$quote" | tr '[:upper:]' '[:lower:]' | tr -s ' ')

    # Extract key terms (words > 4 chars)
    key_terms=$(echo "$normalized" | grep -oE '\b[a-z]{5,}\b' | head -5)

    # Check if all key terms appear in session
    found=0
    total=0
    for term in $key_terms; do
        total=$((total + 1))
        if grep -qi "$term" "$session_file"; then
            found=$((found + 1))
        fi
    done

    echo "scale=2; $found / $total" | bc
}
```

### Scry Integration

```bash
semantic_verify() {
    local statement=$1
    local min_score=${2:-0.5}

    # Query scry for supporting evidence
    results=$(patina scry "$statement" --limit 10 --min-score "$min_score" 2>/dev/null)

    # Parse results and calculate average score
    scores=$(echo "$results" | grep -oE 'score: [0-9.]+' | cut -d: -f2)

    if [ -z "$scores" ]; then
        echo "0.0"
        return
    fi

    # Calculate average
    sum=0
    count=0
    for score in $scores; do
        sum=$(echo "$sum + $score" | bc)
        count=$((count + 1))
    done

    echo "scale=2; $sum / $count" | bc
}
```

### Contradiction Detection

```bash
check_contradictions() {
    local statement=$1

    # Search for beliefs that might contradict
    # Use negation patterns
    negated=$(echo "$statement" | sed 's/prefer/avoid/g; s/always/never/g; s/should/should not/g')

    results=$(patina scry "$negated" --limit 5 2>/dev/null)

    # Check for high-similarity results (potential contradictions)
    high_sim=$(echo "$results" | grep -E 'score: 0\.[89]' | wc -l)

    if [ "$high_sim" -gt 0 ]; then
        echo "warning:$high_sim potential contradictions"
        echo "$results" | grep -E 'score: 0\.[89]'
    else
        echo "ok:no contradictions found"
    fi
}
```

## Confidence Computation

### Weight Configuration

```yaml
# .patina/config.toml
[beliefs.validation]
weights:
  content_match: 0.25      # Evidence says what we claim
  semantic_support: 0.35   # Scry finds related content
  graph_support: 0.25      # Other beliefs support this
  base_confidence: 0.15    # Existence bonus

thresholds:
  strong_evidence: 0.7     # Score for "strong" support
  medium_evidence: 0.5     # Score for "medium" support
  min_evidence_count: 2    # Minimum verified links
  max_age_days: 180        # Trigger revalidation
  decay_rate: 0.2          # Per-year confidence decay
```

### Formula Implementation

```python
def compute_belief_confidence(belief_path: str, config: dict) -> dict:
    """Compute confidence from verifiable signals."""

    weights = config['beliefs']['validation']['weights']
    thresholds = config['beliefs']['validation']['thresholds']

    # Layer 1: Link verification (pass/fail gate)
    link_result = verify_links(belief_path)
    if link_result['verified'] < link_result['total']:
        return {
            'score': 0.0,
            'status': 'invalid',
            'reason': f"Broken links: {link_result['missing']}"
        }

    # Layer 2: Content verification
    content_result = verify_content(belief_path)
    content_score = content_result['verified'] / max(1, content_result['total'])

    # Layer 3: Semantic verification
    statement = extract_statement(belief_path)
    scry_result = scry(statement, limit=10)
    semantic_score = avg([r.score for r in scry_result if r.score > thresholds['medium_evidence']])

    # Layer 4: Graph verification
    graph_result = analyze_graph(belief_path)
    graph_score = min(1.0, graph_result['in_degree'] / 3)

    # Layer 5: Temporal decay
    age_days = get_belief_age(belief_path)
    decay = max(0.5, 1.0 - (age_days / 365) * config['beliefs']['validation']['decay_rate'])

    # Weighted combination
    raw_score = (
        content_score * weights['content_match'] +
        semantic_score * weights['semantic_support'] +
        graph_score * weights['graph_support'] +
        weights['base_confidence']
    )

    final_score = round(raw_score * decay, 2)

    return {
        'score': final_score,
        'status': 'valid' if final_score >= 0.4 else 'low_confidence',
        'signals': {
            'content_match': content_score,
            'semantic_support': semantic_score,
            'graph_support': graph_score,
            'temporal_decay': decay
        },
        'computed_at': datetime.utcnow().isoformat()
    }
```

## File Changes

### Modified Files

1. **`.claude/skills/epistemic-beliefs/scripts/create-belief.sh`**
   - Add pre-creation validation
   - Replace fake signals with computed values
   - Add confirmation prompts for weak evidence

2. **`.claude/skills/epistemic-beliefs/SKILL.md`**
   - Document validation behavior
   - Update confidence guidelines to reflect computation

### New Files

1. **`.claude/skills/epistemic-beliefs/scripts/validate-belief.sh`**
   - Standalone validation for single belief
   - Returns exit code for scripting

2. **`.claude/skills/epistemic-beliefs/scripts/audit-beliefs.sh`**
   - Batch validation of all beliefs
   - Generates health report

3. **`.claude/skills/epistemic-beliefs/scripts/compute-confidence.sh`**
   - Recompute confidence for a belief
   - Updates YAML frontmatter in place

## Testing Strategy

### Unit Tests

```bash
# Test link verification
test_link_verification() {
    # Create temp belief with known links
    # Verify correct detection of valid/invalid links
}

# Test content matching
test_content_matching() {
    # Create temp belief with quotes
    # Create temp session with matching/non-matching content
    # Verify score calculation
}

# Test confidence computation
test_confidence_computation() {
    # Create belief with known signals
    # Verify formula produces expected score
}
```

### Integration Tests

```bash
# Test full validation flow
test_validation_flow() {
    # 1. Create belief with valid evidence
    # 2. Run validation
    # 3. Assert passes

    # 4. Create belief with broken link
    # 5. Run validation
    # 6. Assert fails with correct reason
}
```

### Regression Tests

```bash
# Validate all existing beliefs still pass
test_existing_beliefs() {
    for belief in layer/surface/epistemic/beliefs/*.md; do
        ./validate-belief.sh "$belief"
        assert_exit_code 0
    done
}
```

## Rollout Plan

### Week 1: Audit Tool
- [ ] Implement validate-belief.sh
- [ ] Run on all 14 existing beliefs
- [ ] Fix any broken links discovered
- [ ] Document baseline health metrics

### Week 2: Computed Signals
- [ ] Implement compute-confidence.sh
- [ ] Migrate existing beliefs to computed signals
- [ ] Verify no regression in confidence scores

### Week 3: Pre-Creation Validation
- [ ] Add guardrails to create-belief.sh
- [ ] Test with real belief creation
- [ ] Tune warning thresholds based on feedback

### Week 4: Periodic Audit
- [ ] Implement audit-beliefs.sh
- [ ] Add to session-end workflow (optional)
- [ ] Create monthly audit reminder
