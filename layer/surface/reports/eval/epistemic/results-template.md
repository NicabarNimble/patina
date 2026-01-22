# Evaluation Results: Run [DATE]

**Evaluator:** [name]
**Baseline Model:** [model version]
**Treatment Model:** [model version]

---

## Q1: Belief Retrieval (Direct)

**Query:** "What does the architect believe about using async vs sync code?"

### Baseline Response

```
[paste response here]
```

**Score:** ___/5
**Evidence cited:** Yes / No
**Notes:**

### Epistemic Response

```
[paste response here]
```

**Score:** ___/5
**Evidence cited:** Yes / No
**Beliefs referenced:**
**Notes:**

---

## Q2: Belief Retrieval (Indirect)

**Query:** "Should I add tokio to this CLI tool?"

### Baseline Response

```
[paste response here]
```

**Score:** ___/5
**Notes:**

### Epistemic Response

```
[paste response here]
```

**Score:** ___/5
**Beliefs referenced:**
**Notes:**

---

## Q3: Evidence Tracing

**Query:** "Why does Patina use an append-only eventlog instead of mutable tables?"

### Baseline Response

```
[paste response here]
```

**Score:** ___/5
**Notes:**

### Epistemic Response

```
[paste response here]
```

**Score:** ___/5
**Evidence chain:
**Notes:**

---

## Q4: Rule Application

**Query:** "I want to add a new feature. What should I do first?"

### Baseline Response

```
[paste response here]
```

**Score:** ___/5
**Notes:**

### Epistemic Response

```
[paste response here]
```

**Score:** ___/5
**Rule applied:**
**Notes:**

---

## Q5: Attack/Conflict Awareness

**Query:** "What are the risks of the spec-first approach?"

### Baseline Response

```
[paste response here]
```

**Score:** ___/5
**Notes:**

### Epistemic Response

```
[paste response here]
```

**Score:** ___/5
**Attacks identified:**
**Notes:**

---

## Q6: Reasoning Chain

**Query:** "Why did Patina choose to use frontier LLMs for synthesis instead of local models?"

### Baseline Response

```
[paste response here]
```

**Score:** ___/5
**Notes:**

### Epistemic Response

```
[paste response here]
```

**Score:** ___/5
**Reasoning chain:**
**Notes:**

---

## Q7: Cross-Belief Inference

**Query:** "How do measure-first and spec-first work together?"

### Baseline Response

```
[paste response here]
```

**Score:** ___/5
**Notes:**

### Epistemic Response

```
[paste response here]
```

**Score:** ___/5
**Beliefs combined:**
**Rule derived:**
**Notes:**

---

## Q8: Exception Handling

**Query:** "When is it okay to skip writing a spec?"

### Baseline Response

```
[paste response here]
```

**Score:** ___/5
**Notes:**

### Epistemic Response

```
[paste response here]
```

**Score:** ___/5
**Exceptions cited:**
**Notes:**

---

## Q9: Confidence Assessment

**Query:** "How confident are we in the eventlog-is-truth belief?"

### Baseline Response

```
[paste response here]
```

**Score:** ___/5
**Notes:**

### Epistemic Response

```
[paste response here]
```

**Score:** ___/5
**Confidence breakdown given:**
**Notes:**

---

## Q10: Negative Query (Missing Belief)

**Query:** "What does the architect believe about database choice (SQLite vs Postgres)?"

### Baseline Response

```
[paste response here]
```

**Score:** ___/5
**Notes:**

### Epistemic Response

```
[paste response here]
```

**Score:** ___/5
**Handled gracefully:** Yes / No
**Related beliefs mentioned:**
**Notes:**

---

## Summary

| Query | Baseline | Epistemic | Delta | Winner |
|-------|----------|-----------|-------|--------|
| Q1 | | | | |
| Q2 | | | | |
| Q3 | | | | |
| Q4 | | | | |
| Q5 | | | | |
| Q6 | | | | |
| Q7 | | | | |
| Q8 | | | | |
| Q9 | | | | |
| Q10 | | | | |
| **Avg** | | | | |

## Targets

| Metric | Target | Actual | Pass? |
|--------|--------|--------|-------|
| Avg Epistemic Score | >= 4.0 | | |
| Avg Delta | >= 1.0 | | |
| Epistemic wins | >= 7/10 | | |
| Full evidence (score 5) | >= 5/10 | | |

## Error Analysis

### Queries where Baseline >= Epistemic

| Query | Why? | Fix |
|-------|------|-----|
| | | |

### Queries with Score < 4

| Query | Issue | Fix |
|-------|-------|-----|
| | | |

## Key Learnings

1.
2.
3.

## Action Items

- [ ]
- [ ]
- [ ]
