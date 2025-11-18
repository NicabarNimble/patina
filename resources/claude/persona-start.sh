#!/bin/bash
# Start a new Patina persona session
# Intelligent extraction of beliefs from observations

# Check for active persona session
ACTIVE_PERSONA=".claude/context/active-persona-session.md"
if [ -f "$ACTIVE_PERSONA" ]; then
    echo "Found incomplete persona session, cleaning up..."

    # Check if has content
    if [ $(wc -l < "$ACTIVE_PERSONA") -gt 10 ]; then
        # Run persona-end silently to save it
        $(dirname "$0")/persona-end.sh --silent
    else
        rm "$ACTIVE_PERSONA"
        echo "Removed empty persona session"
    fi
fi

# Create session ID
PERSONA_SESSION_ID="persona-$(date +%Y%m%d-%H%M%S)"

# Check databases exist
FACTS_DB=".patina/data/facts.db"
if [ ! -f "$FACTS_DB" ]; then
    echo "❌ Error: facts.db not found at $FACTS_DB"
    echo "   Run: patina session extract --all"
    exit 1
fi

# Create active persona session file
mkdir -p .claude/context

cat > "$ACTIVE_PERSONA" << 'EOF'
# Persona Session
**ID**: PERSONA_SESSION_ID_PLACEHOLDER
**Started**: TIMESTAMP_PLACEHOLDER
**Database**: .patina/data/facts.db

## Goal
Discover and codify your beliefs through interactive dialogue.

## Available Tools

### Semantic Search (Primary)
```bash
# Find observations using semantic similarity
patina query semantic "security practices" --type pattern,decision --limit 10

# Results include similarity scores and evidence strength
# Example: {"id": "...", "type": "pattern", "text": "...", "similarity": 0.78, "evidence_strength": "strong"}
```

### Query Observations (SQLite - Fallback)
```bash
sqlite3 .patina/data/facts.db "SELECT * FROM patterns WHERE category = 'architecture'"
sqlite3 .patina/data/facts.db "SELECT * FROM sessions ORDER BY started_at DESC LIMIT 5"
```

### Validate Beliefs (Neuro-Symbolic - MANDATORY)
```bash
# Validate belief using semantic evidence + symbolic reasoning
patina belief validate "I prefer Rust for systems programming" \
  --min-score 0.50 --limit 20

# Returns JSON with:
# - valid: true/false (meets evidence threshold?)
# - reason: "adequate_evidence" | "weak_evidence" | "sufficient_strong_evidence"
# - metrics: {weighted_score, strong_evidence_count, avg_reliability, avg_similarity}
# - observations: [matching observations with similarity scores]

# Example output:
# {
#   "query": "I prefer Rust for systems programming",
#   "valid": true,
#   "reason": "adequate_evidence",
#   "metrics": {
#     "weighted_score": 3.45,
#     "strong_evidence_count": 4,
#     "has_diverse_sources": true,
#     "avg_reliability": 0.78,
#     "avg_similarity": 0.82
#   },
#   "observations": [...]
# }
```

## Session Flow

1. **Domain Selection**: Analyze recent work, pick active domain
2. **Gap Detection**: Find observations not yet codified as beliefs
3. **Evidence Search**: Use semantic search to find ALL related evidence across history
4. **Question Generation**: Create ONE atomic yes/no question
5. **User Answers**: Capture answer (yes/no/conditional)
6. **Contradiction Resolution**: Use semantic search to find contradictions, generate refined questions
7. **Codify Belief**: Store in beliefs table **WITH EVIDENCE LINKS** (see below)
8. **Repeat**: Next gap → next question

## CRITICAL: Neuro-Symbolic Validation

**YOU MUST VALIDATE BELIEFS BEFORE CODIFYING THEM.**

Belief validation is powered by the ReasoningEngine (embedded Scryer Prolog + semantic search). This is automatic, deterministic, and cannot be overridden by LLM judgment.

### Mandatory Workflow for Belief Validation

**Step 1: Validate the Belief**
```bash
# Run neuro-symbolic validation: semantic search (neural) → Prolog rules (symbolic)
RESULT=$(patina belief validate "I prefer Rust for systems programming" --min-score 0.50 --limit 20)
```

**Step 2: Parse Validation Result**
```bash
# Extract fields from JSON result
VALID=$(echo "$RESULT" | jq -r '.valid')
REASON=$(echo "$RESULT" | jq -r '.reason')
WEIGHTED_SCORE=$(echo "$RESULT" | jq -r '.metrics.weighted_score')
STRONG_COUNT=$(echo "$RESULT" | jq -r '.metrics.strong_evidence_count')
```

**Step 3: Act on Result**
```bash
if [ "$VALID" = "true" ]; then
  # Belief is supported by adequate evidence - safe to codify
  # Use weighted_score and metrics to set confidence
  sqlite3 .patina/data/facts.db "INSERT INTO beliefs (...) VALUES (..., $CONFIDENCE)"
else
  # Insufficient evidence - ask clarifying question or skip
  echo "⚠️ Weak evidence: $REASON"
fi
```

### Validation Thresholds (Prolog Rules)

These thresholds are enforced by symbolic reasoning in `.patina/validation-rules.pl`:

- **Weighted Score ≥ 3.0**: Adequate evidence (valid=true)
  - Calculation: Σ(similarity × reliability) for all observations with sim≥0.50
  - Example: 4 obs @ 0.75 sim × 0.75 rel = 2.25 (weak); 5 obs = 2.8 (weak); 6 obs = 3.4 (valid)

- **Weighted Score ≥ 5.0**: Sufficient strong evidence (valid=true, higher confidence)
  - Requires multiple high-quality observations (sim≥0.70, rel≥0.70)

- **Strong Evidence Count ≥ 2**: Diverse high-quality support
  - Counts observations with both similarity≥0.70 AND reliability≥0.70

- **Has Diverse Sources**: Multiple source types (session, commit, etc.)

**YOU CANNOT OVERRIDE VALIDATION RULES.** If validation fails but you believe the belief is valid, the observations are insufficient or the rules need adjustment - not manual override.

## CRITICAL: Evidence Linking

When codifying a belief, you MUST create TWO database entries:

### Step 1: Insert Belief
```sql
INSERT INTO beliefs (statement, value, confidence, observation_count)
VALUES ('belief_name', 1, 0.85, 2);
```

### Step 2: Link Evidence (REQUIRED)
```sql
-- For each supporting observation, insert a link:
INSERT INTO belief_observations
  (belief_id, session_id, observation_type, observation_id, validates)
VALUES
  (last_insert_rowid(), '20251008-061520', 'pattern', 5, 1),
  (last_insert_rowid(), '20251007-210232', 'decision', 3, 1);
```

**observation_type can be:**
- `'pattern'` - links to patterns table
- `'technology'` - links to technologies table
- `'decision'` - links to decisions table
- `'challenge'` - links to challenges table

**validates:**
- `1` = supports belief (evidence strengthens it)
- `0` = contradicts belief (counterexample)

### Example: Complete Belief Codification
```bash
# Find supporting evidence first
sqlite3 .patina/data/facts.db "SELECT id, pattern_name, session_id FROM patterns WHERE category = 'security'"

# Insert belief
sqlite3 .patina/data/facts.db "INSERT INTO beliefs (statement, value, confidence, observation_count) VALUES ('never_commit_secrets', 1, 0.95, 2)"

# Get the belief_id (last inserted)
BELIEF_ID=$(sqlite3 .patina/data/facts.db "SELECT last_insert_rowid()")

# Link evidence (pattern id=5, session 20251008-061520)
sqlite3 .patina/data/facts.db "INSERT INTO belief_observations (belief_id, session_id, observation_type, observation_id, validates) VALUES ($BELIEF_ID, '20251008-061520', 'pattern', 5, 1)"

# Link evidence (pattern id=8, session 20251007-185647)
sqlite3 .patina/data/facts.db "INSERT INTO belief_observations (belief_id, session_id, observation_type, observation_id, validates) VALUES ($BELIEF_ID, '20251007-185647', 'pattern', 8, 1)"
```

## Remember

- ONE question at a time (not a checklist)
- Your answer + observations inform NEXT question
- LLM does heavy lifting (search, synthesis, gap detection)
- Feels like discovery, not interrogation
- **Evidence-based: ALWAYS populate belief_observations table**

## Beliefs Created
<!-- Track beliefs created during this session -->

## Activity Log
### TIME_PLACEHOLDER - Session Start
Persona session initialized
Scanning observations for belief gaps...

EOF

# Replace placeholders
sed -i '' "s/PERSONA_SESSION_ID_PLACEHOLDER/${PERSONA_SESSION_ID}/g" "$ACTIVE_PERSONA"
sed -i '' "s/TIMESTAMP_PLACEHOLDER/$(date -u +"%Y-%m-%dT%H:%M:%SZ")/g" "$ACTIVE_PERSONA"
sed -i '' "s/TIME_PLACEHOLDER/$(date +"%H:%M")/g" "$ACTIVE_PERSONA"

echo "✓ Persona session started"
echo "  ID: ${PERSONA_SESSION_ID}"
echo ""
echo "🧠 This session will help you discover and codify your beliefs."
echo ""
echo "I can query:"
echo "  - .patina/data/facts.db (observations: sessions, patterns, decisions)"
echo "  - patina belief validate (neuro-symbolic reasoning with ReasoningEngine)"
echo ""
echo "Ready to begin."
