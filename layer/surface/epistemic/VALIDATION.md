# Epistemic Layer Validation

How to test and iterate on the belief/rule system.

---

## Quick Validation

### 1. Structure Check

```bash
# List all beliefs
ls -la layer/surface/epistemic/beliefs/

# List all rules
ls -la layer/surface/epistemic/rules/

# Check frontmatter is valid YAML
for f in layer/surface/epistemic/beliefs/*.md; do
  echo "=== $f ==="
  head -20 "$f" | grep -A 20 "^---"
done
```

### 2. Link Integrity

All wikilinks should resolve. Check for broken links:

```bash
# Extract all wikilinks from beliefs
grep -oh '\[\[[^]]*\]\]' layer/surface/epistemic/beliefs/*.md | sort -u

# Extract all wikilinks from rules
grep -oh '\[\[[^]]*\]\]' layer/surface/epistemic/rules/*.md | sort -u
```

### 3. Confidence Bounds

All confidence scores should be 0.0-1.0:

```bash
grep -r "score:" layer/surface/epistemic/ | grep -v "0\.[0-9]"
```

---

## Test Scenarios

### Scenario 1: Add a New Belief

**Test**: Add belief that SUPPORTS existing beliefs.

1. Create `beliefs/exploration-driven-development.md`
2. Link it as supporting `spec-first`
3. Verify the support relationship is bidirectional

**Expected**: No conflicts, belief joins the graph.

### Scenario 2: Add a Conflicting Belief

**Test**: Add belief that ATTACKS existing beliefs.

1. Create `beliefs/move-fast-break-things.md` with confidence 0.6
2. Mark it as attacking `spec-first`
3. Compare entrenchment: spec-first (high) vs new (low)

**Expected**:
- If entrenchment(new) < entrenchment(existing): new belief is defeated or scoped
- If entrenchment(new) > entrenchment(existing): revision required

### Scenario 3: Evidence Invalidation

**Test**: What happens when evidence is removed?

1. Imagine `session-20260115-121358` is deleted
2. `spec-first` loses primary evidence
3. Confidence should decrease

**Expected**: Confidence recalculated, may trigger revision.

### Scenario 4: Rule Derivation

**Test**: Can we derive a new rule from existing beliefs?

1. Identify belief cluster: `measure-first` + `dont-build-what-exists`
2. Derive rule: "audit existing tools before building"
3. Verify rule conditions reference beliefs correctly

**Expected**: New rule created with proper provenance.

---

## Iteration Questions

After each change, ask:

1. **Coherence**: Do beliefs form a consistent set?
2. **Coverage**: Are important decisions captured?
3. **Provenance**: Can every belief cite evidence?
4. **Utility**: Would an LLM find this useful for reasoning?

---

## Metrics to Track

| Metric | Current | Target |
|--------|---------|--------|
| Beliefs | 5 | 20+ |
| Rules | 3 | 10+ |
| Avg Confidence | 0.886 | > 0.7 |
| Defeated attacks | 5 | - |
| Active attacks | 5 | - |
| Broken links | 0 | 0 |

---

## Next Steps for Iteration

### Phase 1: Manual Population
- [ ] Extract 10 more beliefs from sessions
- [ ] Add evidence links to existing sessions
- [ ] Derive 5 more rules from belief clusters

### Phase 2: Tooling
- [ ] Script to validate frontmatter schema
- [ ] Script to build link graph
- [ ] Script to calculate confidence from signals

### Phase 3: Integration
- [ ] Connect to scry (query beliefs semantically)
- [ ] Connect to oxidize (embed beliefs)
- [ ] Connect to Mother (cross-project beliefs)

---

## Manual Testing Checklist

- [ ] Can read any belief file and understand it without context
- [ ] Evidence links point to real sessions/commits
- [ ] Support/attack relationships make logical sense
- [ ] Rules correctly reference their condition beliefs
- [ ] Exceptions in rules are reasonable
- [ ] Index is up-to-date with actual files
- [ ] Argument graph in index matches actual links
