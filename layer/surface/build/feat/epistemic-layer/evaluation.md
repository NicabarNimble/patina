# Epistemic Layer — Evaluation (Andrew Ng Methodology)

Results from measuring the epistemic layer against a 10-query evaluation set.
Methodology follows Andrew Ng's measurement-driven approach. Completed during
sessions 20260116-054624 and 20260116-080414.

---

## Approach

1. **"Show me the failure cases"** — Test where the system fails, not just where it works
2. **"Establish a baseline first"** — Measure without epistemic layer, then with
3. **"Error analysis on real examples"** — Manually examine failures to find patterns
4. **"Iterate on data, not architecture"** — Fix data gaps before adding complexity

---

## Evaluation Query Set

10 queries testing different epistemic capabilities:

| # | Type | Query |
|---|------|-------|
| Q1 | Belief retrieval (direct) | "What does the architect believe about async vs sync?" |
| Q2 | Belief retrieval (indirect) | "Should I add tokio to this CLI tool?" |
| Q3 | Evidence tracing | "Why append-only eventlog instead of mutable tables?" |
| Q4 | Rule application | "I want to add a new feature. What should I do first?" |
| Q5 | Attack awareness | "What are the risks of the spec-first approach?" |
| Q6 | Reasoning chain | "Why frontier LLMs for synthesis instead of local?" |
| Q7 | Cross-belief inference | "How do measure-first and spec-first work together?" |
| Q8 | Exception handling | "When is it okay to skip writing a spec?" |
| Q9 | Confidence assessment | "How confident are we in eventlog-is-truth?" |
| Q10 | Missing belief (negative) | "What about SQLite vs Postgres?" |

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

## Results (Q1-Q10 Complete)

| Query | Topic | Baseline | Treatment | Delta |
|-------|-------|----------|-----------|-------|
| Q1 | Belief retrieval (direct) | 3.0 | 5.0 | +2.0 |
| Q2 | Belief retrieval (indirect) | 3.0 | 5.0 | +2.0 |
| Q3 | Evidence tracing | 3.5 | 5.0 | +1.5 |
| Q4 | Rule application | 2.5 | 5.0 | +2.5 |
| Q5 | Attack awareness | 3.0 | 5.0 | +2.0 |
| Q6 | Reasoning chain | 2.0 | 5.0 | +3.0 |
| Q7 | Cross-belief inference | 3.0 | 5.0 | +2.0 |
| Q8 | Exception handling | 2.0 | 5.0 | +3.0 |
| Q9 | Confidence assessment | 2.0 | 5.0 | +3.0 |
| Q10 | Missing belief (negative) | 3.0 | 4.0 | +1.0 |
| **Average** | | **2.7** | **4.9** | **+2.2** |

---

## Key Findings

### 1. Error Analysis Reveals Data Gaps

Q1 initially failed in treatment (score: 2) because `sync-first` belief was missing. The belief
existed in sessions (Aug 2025) but wasn't extracted into the epistemic layer.

**Action:** Created `sync-first.md` from session-20250804-073015.
**Result:** Treatment score improved 2 → 5.

**Lesson:** Error analysis reveals data gaps, not algorithm problems.

### 2. Baseline vs Treatment Differences

| Aspect | Baseline | Treatment |
|--------|----------|-----------|
| Answer source | Scattered session fragments | Structured belief files |
| Confidence | Unknown | Explicit (0.85-0.92) |
| Evidence | Raw mentions | Weighted links |
| Exceptions | Not found | Documented attacks |
| Reasoning | Inferred | Explicit chains |

### 3. Treatment Advantages

- **Q1:** Clear statement vs inferred from fragments
- **Q2:** "No, unless exceptions" vs "probably not"
- **Q3:** Helland cited, L2 eventlog explained vs basic reasons only
- **Q4:** 4-step process with exceptions vs scattered hints

### 4. Q5-Q10 Analysis (Session 20260116-080414)

| Query | Finding |
|-------|---------|
| Q5 (Attack awareness) | Treatment provided specific attack (analysis-paralysis), confidence (0.3), and scope ("only when spec exceeds 1 week") vs generic risks |
| Q6 (Reasoning chain) | Largest delta (+3.0) — two-belief reasoning chain with defeated attacks and phased approach |
| Q7 (Cross-belief) | Rule derivation explicit: measure-first + spec-first → implement-after-measurement |
| Q8 (Exceptions) | Specific criteria (20 lines, security urgency) vs vague "it depends" |
| Q9 (Confidence) | Full signal breakdown impossible without epistemic layer — largest treatment advantage |
| Q10 (Missing belief) | Only non-5 score: graceful gap acknowledgment + related belief inference |

### 5. Strongest Treatment Advantages (Q5-Q10)

- **Exception handling (Q8, +3.0)**: Explicit exceptions with criteria impossible to know without documentation
- **Confidence assessment (Q9, +3.0)**: Signal breakdown only available from frontmatter — baseline cannot answer
- **Reasoning chain (Q6, +3.0)**: Multi-belief chains with defeated alternatives show reasoning process

### 6. Gap Identified (Q10) — Error Analysis

Q10 scored 4 instead of 5 because no explicit belief exists for SQLite vs Postgres. The system
correctly:
1. Acknowledged the gap
2. Found related beliefs (eventlog-is-truth implies SQLite)
3. Suggested creating a new belief

**Deep analysis:** Evidence for SQLite preference IS scattered in the codebase:
- `sync-first` mentions "SQLite queries (single-threaded is fine)"
- `rqlite-architecture` was defeated and "migrated to SQLite"
- `eventlog-is-truth` mentions "patina.db" architecture

**Two paths forward:**
1. **Create explicit belief:** `sqlite-preferred` or `local-first-storage`
2. **Enhance search:** Treatment could search WITHIN belief bodies, not just belief IDs

**Lesson:** The epistemic layer handles gaps gracefully, but coverage matters for max score.
Information scattered across beliefs is less valuable than explicit beliefs.

---

## Success Criteria Assessment (Final)

| Metric | Target | Actual (Q1-Q10) | Status |
|--------|--------|-----------------|--------|
| Avg Epistemic Score | >= 4.0 | 4.9 | Pass |
| Avg Delta | >= 1.0 | +2.2 | Pass |
| Epistemic wins | >= 7/10 | 10/10 (100%) | Pass |
| Full evidence (score 5) | >= 5/10 | 9/10 (90%) | Pass |

---

## Validated Hypothesis

> **"Can an LLM correctly explain WHY a decision was made, with traceable evidence?"**

**Without epistemic layer:** Guesses or fragments from sessions (avg 2.7)
**With epistemic layer:** Cites beliefs, evidence chains, exceptions (avg 4.9)

**Conclusion:** The epistemic layer provides measurable improvement (+2.2 points average) in LLM
reasoning quality about project decisions. All 10 queries showed improvement, with 9/10 achieving
maximum score (5.0).

---

## Appendix: Example Belief (Full)

```markdown
---
type: belief
id: spec-first
persona: architect
facets: [development-process, design]
confidence:
  score: 0.85
  signals:
    evidence: 0.90
    source_reliability: 0.85
    recency: 0.80
    survival: 0.85
    user_endorsement: 0.70
entrenchment: high
status: active
extracted: 2026-01-15
revised: 2026-01-16
---

# spec-first

Design before coding. Write specs as artifacts of learning, not blueprints.

## Statement

Prefer designing the solution in a spec document before implementing code.
Specs capture where thinking was at that moment and serve as exploration artifacts.

## Evidence

- [[session-20260115-121358]] - "Spec first, spike second" pattern observed (weight: 0.9)
- [[session-20260115-053944]] - Spec review before implementation (weight: 0.8)
- [[spec-surface-layer]] - Example of spec-driven design (weight: 0.7)

## Supports

- [[exploration-driven-development]]
- [[measure-first]]

## Attacks

- [[move-fast-break-things]] (status: defeated, reason: leads to rework)

## Attacked-By

- [[analysis-paralysis]] (status: active, confidence: 0.3, scope: "only when spec exceeds 1 week")

## Revision Log

- 2026-01-15: Extracted from session-20260115-121358 (confidence: 0.7)
- 2026-01-16: Multiple session evidence added (confidence: 0.7 → 0.85)
```
