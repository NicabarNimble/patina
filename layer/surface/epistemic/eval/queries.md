# Epistemic Layer Evaluation Queries

**Purpose:** Test whether the epistemic layer improves LLM reasoning about project decisions.

**Method:**
1. Ask each query WITHOUT epistemic context (baseline)
2. Ask each query WITH epistemic belief files (treatment)
3. Score answer quality (1-5)
4. Document failures

---

## Scoring Rubric

| Score | Description |
|-------|-------------|
| 1 | Wrong or no answer |
| 2 | Vague, generic answer |
| 3 | Correct but no evidence cited |
| 4 | Correct with partial evidence |
| 5 | Correct with full evidence chain |

---

## Query Set

### Q1: Belief Retrieval (Direct)

**Query:** "What does the architect believe about using async vs sync code?"

**Expected Belief:** `sync-first` (or `smart-model-in-room` for LLM context)

**Expected Answer Elements:**
- Prefer synchronous, blocking code
- Borrow checker works better without async lifetimes
- LLMs understand sync code better

**Expected Evidence:**
- session-20260115-121358
- session-20250804 (if exists)

**Baseline Score:** ___
**Epistemic Score:** ___
**Notes:**

---

### Q2: Belief Retrieval (Indirect)

**Query:** "Should I add tokio to this CLI tool?"

**Expected Belief:** `sync-first` → implies NO

**Expected Answer Elements:**
- Probably not for a CLI
- sync-first belief suggests avoiding async
- Unless high-concurrency is needed (exception)

**Expected Evidence:**
- sync-first belief
- Rule: avoid-async-in-cli (if asked about CLI)

**Baseline Score:** ___
**Epistemic Score:** ___
**Notes:**

---

### Q3: Evidence Tracing

**Query:** "Why does Patina use an append-only eventlog instead of mutable tables?"

**Expected Belief:** `eventlog-is-truth`

**Expected Answer Elements:**
- Eventlog is source of truth
- Tables are materialized views
- Pat Helland's principle
- Non-deterministic boundaries need capture

**Expected Evidence:**
- session-20260115-121358 (L2 eventlog insight)
- session-20260114-114833 (git IS the eventlog)
- helland-paper reference

**Baseline Score:** ___
**Epistemic Score:** ___
**Notes:**

---

### Q4: Rule Application

**Query:** "I want to add a new feature. What should I do first?"

**Expected Rule:** `implement-after-measurement`

**Expected Answer Elements:**
1. Write a spec describing the problem
2. Measure the current baseline
3. Prove the gap exists with data
4. Then implement

**Expected Evidence:**
- measure-first belief
- spec-first belief
- Rule: implement-after-measurement

**Baseline Score:** ___
**Epistemic Score:** ___
**Notes:**

---

### Q5: Attack/Conflict Awareness

**Query:** "What are the risks of the spec-first approach?"

**Expected Attack:** `analysis-paralysis`

**Expected Answer Elements:**
- Analysis paralysis is a risk
- Scoped: "only when spec exceeds 1 week"
- Confidence of attack: 0.3 (low but active)

**Expected Evidence:**
- spec-first belief → Attacked-By section
- Scope qualifier

**Baseline Score:** ___
**Epistemic Score:** ___
**Notes:**

---

### Q6: Reasoning Chain

**Query:** "Why did Patina choose to use frontier LLMs for synthesis instead of local models?"

**Expected Beliefs:** `smart-model-in-room` + `dont-build-what-exists`

**Expected Answer Elements:**
- Quality gap between frontier and local is large
- Frontier LLMs already available via adapters
- Don't build infrastructure before proving patterns
- Local models deferred to Phase 6+

**Expected Evidence:**
- session-20260115-053944 (Mother architecture discussion)
- spec-surface-layer

**Baseline Score:** ___
**Epistemic Score:** ___
**Notes:**

---

### Q7: Cross-Belief Inference

**Query:** "How do measure-first and spec-first work together?"

**Expected:** Both beliefs + derived rule

**Expected Answer Elements:**
- spec-first: design before coding
- measure-first: prove problem with data
- Combined: spec → measure baseline → prove gap → implement
- Rule: implement-after-measurement derives from both

**Expected Evidence:**
- Both belief files
- Rule file showing derived-from

**Baseline Score:** ___
**Epistemic Score:** ___
**Notes:**

---

### Q8: Exception Handling

**Query:** "When is it okay to skip writing a spec?"

**Expected:** Exceptions from rules

**Expected Answer Elements:**
- Trivial fixes (< 20 lines)
- Security patches (urgent)
- Explicit user request (with acknowledgment)

**Expected Evidence:**
- Rule: implement-after-measurement → Exceptions section

**Baseline Score:** ___
**Epistemic Score:** ___
**Notes:**

---

### Q9: Confidence Assessment

**Query:** "How confident are we in the eventlog-is-truth belief?"

**Expected:** Confidence breakdown

**Expected Answer Elements:**
- Overall: 0.92
- Evidence: 0.95 (strong)
- Entrenchment: very-high
- Multiple sessions support it
- Helland academic grounding

**Expected Evidence:**
- eventlog-is-truth frontmatter
- Evidence section with weights

**Baseline Score:** ___
**Epistemic Score:** ___
**Notes:**

---

### Q10: Negative Query (Missing Belief)

**Query:** "What does the architect believe about database choice (SQLite vs Postgres)?"

**Expected:** No direct belief (tests handling of gaps)

**Expected Answer Elements:**
- No explicit belief found
- Could infer from eventlog-is-truth (SQLite supports append)
- Could infer from local-first principle
- Honest: "No explicit belief, but related beliefs suggest..."

**Expected Evidence:**
- N/A (tests graceful handling)

**Baseline Score:** ___
**Epistemic Score:** ___
**Notes:**

---

## Summary Table

| Query | Topic | Baseline | Epistemic | Delta |
|-------|-------|----------|-----------|-------|
| Q1 | Belief retrieval (direct) | | | |
| Q2 | Belief retrieval (indirect) | | | |
| Q3 | Evidence tracing | | | |
| Q4 | Rule application | | | |
| Q5 | Attack awareness | | | |
| Q6 | Reasoning chain | | | |
| Q7 | Cross-belief inference | | | |
| Q8 | Exception handling | | | |
| Q9 | Confidence assessment | | | |
| Q10 | Missing belief (negative) | | | |
| **Average** | | | | |

---

## Test Protocol

### Baseline Test (No Epistemic Context)

1. Start fresh Claude conversation
2. Provide only: CLAUDE.md + relevant session files
3. Ask each query
4. Record answer verbatim
5. Score 1-5

### Treatment Test (With Epistemic Context)

1. Start fresh Claude conversation
2. Provide: CLAUDE.md + session files + epistemic/ directory
3. Ask each query
4. Record answer verbatim
5. Score 1-5

### Analysis

For each query where Epistemic > Baseline:
- What made the difference?
- Was evidence cited?
- Was reasoning clearer?

For each query where Baseline >= Epistemic:
- Why didn't epistemic help?
- Missing belief?
- Poor evidence links?
- Schema issue?

---

## Success Criteria

| Metric | Target |
|--------|--------|
| Average Epistemic Score | >= 4.0 |
| Average Delta (Epistemic - Baseline) | >= 1.0 |
| Queries where Epistemic > Baseline | >= 7/10 |
| Queries with full evidence chain (score 5) | >= 5/10 |

---

## Next Steps After Evaluation

1. **If targets met:** Proceed to Phase E2 (schema validation)
2. **If targets not met:**
   - Error analysis on failures
   - Add missing beliefs
   - Improve evidence links
   - Re-evaluate
