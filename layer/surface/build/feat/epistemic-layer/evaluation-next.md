---
type: evaluation
id: epistemic-value-test
status: complete
created: 2026-02-02
related:
  - layer/surface/build/feat/epistemic-layer/evaluation.md
  - layer/surface/build/feat/epistemic-layer/E4.6a-fix/SPEC.md
---

# Is the Belief System Adding Value?

> "If you can't measure it, you can't improve it. And if you can't show it helps,
> you shouldn't ship it." — the concern behind this eval.

**The fear:** We built a 3-hop grounding pipeline, 94 reach files, 100% precision,
86% recall — but does any of this change how the LLM behaves? Are we measuring the
measurement instead of measuring the outcome?

---

## The One Metric (Ng: "Pick one number")

**Decision quality delta:** Given a code modification task, does the LLM make fewer
violations of project principles when beliefs are visible vs invisible?

Not "does the LLM see beliefs" (it does). Not "are beliefs grounded to code" (they
are). The question is: **does it matter?**

---

## Eval Protocol (10 queries, A/B, blind scoring)

### Setup

- **Treatment A (baseline):** `scry` with `impact: false` — LLM sees code but no
  belief annotations.
- **Treatment B (beliefs):** `scry` with `impact: true` (current default) — LLM sees
  code + belief annotations.

### The 10 Queries

Each query is a realistic task the LLM might face. Score each response 0-2:
- **0** = violates a project principle or makes wrong choice
- **1** = acceptable but doesn't leverage project knowledge
- **2** = demonstrates awareness of project principles

| # | Query (task for the LLM) | Relevant Belief | What "2" Looks Like |
|---|--------------------------|-----------------|---------------------|
| 1 | "Add async HTTP client for API calls" | sync-first | Pushes back: "This project is sync-first, consider reqwest blocking" |
| 2 | "Refactor eventlog into multiple tables" | eventlog-is-truth | Warns: "eventlog is the source of truth, materialized views derive from it" |
| 3 | "Skip the spec, just write the code" | spec-first, read-code-before-write | Suggests writing spec first, reading existing code |
| 4 | "Add a Python script to process embeddings" | (Rust-first from CLAUDE.md) | Notes: "This project uses pure Rust at runtime, ort crate for embeddings" |
| 5 | "Batch these 5 changes into one commit" | commit-early-commit-often | Suggests splitting: "one commit = one purpose" |
| 6 | "Add a new belief file manually" | system-owns-format | Notes the system owns format, suggests using the skill |
| 7 | "Delete this unused function without investigating" | investigate-before-delete | Suggests checking callers/history first |
| 8 | "Add feature flags for backwards compatibility" | (over-engineering concern) | Keeps it simple, just changes the code |
| 9 | "Store config in a separate YAML file" | project-config-in-git | Notes config belongs in git-tracked project files |
| 10 | "Add error handling for every internal function" | (over-engineering concern) | Only validates at system boundaries |

### Scoring

Run each query twice (A and B). Record scores. Compute:

```
baseline_score = sum(A scores) / 20    # max possible = 20
belief_score  = sum(B scores) / 20
delta = belief_score - baseline_score
```

**Thresholds:**
- delta >= 0.15 (3+ points) → beliefs clearly help, worth the complexity
- delta 0.05-0.15 (1-2 points) → marginal, simplify before scaling
- delta < 0.05 → not adding value, rethink the approach

---

## The Harder Question: Complexity Budget

Even if beliefs help, are they worth their weight? Measure the cost:

| Cost | How to Measure |
|------|---------------|
| Scrape time added | `time patina scrape` with vs without belief phase |
| DB size added | `beliefs` + `belief_code_reach` + `belief_fts` table sizes |
| Code complexity | Lines in `beliefs/mod.rs` + `enrichment.rs` belief code |
| Cognitive load | Can a new contributor understand grounding in 5 min? |

**The Amidi rule:** If the feature adds 20% complexity but only 5% improvement,
cut it. If it adds 20% complexity and 20% improvement, keep it but stop adding.

---

## What Would Kill the Feature

Be honest about what results would mean "stop":

1. **delta < 0.05** on the 10-query eval → beliefs don't change behavior
2. **Precision drops below 80%** as more beliefs are added → grounding is noisy
3. **Scrape time > 2x** with beliefs vs without → too expensive for the value
4. **No one reads the audit** → the measurement exists but nobody acts on it

If any of these are true, the right move is to archive the epistemic layer as an
experiment and preserve the learnings, not to add more complexity to fix it.

---

## What Would Validate Scaling

Results that justify the cross-project vision:

1. **delta >= 0.15** on 10-query eval → beliefs measurably improve LLM decisions
2. **3+ beliefs trigger behavioral change** → not just one lucky match
3. **Recall > 80%** sustained as beliefs grow → grounding scales
4. **A belief prevents a real mistake** during normal development → not just eval

If these hold, the architecture (semantic hop + structural hop + lexical fallback)
is sound for cross-project belief transfer. The complexity serves the vision.

---

## Results (2026-02-02)

### Raw Scores

| # | Query | A (no beliefs) | B (beliefs) | Belief surfaced? | Source of correct answer |
|---|-------|:-:|:-:|---|---|
| 1 | Async HTTP client | 1 | 1 | No — `sync-first` not retrieved | Code pattern (`reqwest::blocking`) |
| 2 | Refactor eventlog | 2 | 2 | No — `eventlog-is-truth` not retrieved | Session decision + code comments |
| 3 | Skip the spec | 2 | 2 | No — `spec-first` not retrieved | Session history + commit messages |
| 4 | Python embeddings | 2 | 2 | N/A — principle in CLAUDE.md | CLAUDE.md ("Rust-first") |
| 5 | Batch commits | 2 | 2 | No — `commit-early-commit-often` not retrieved | CLAUDE.md ("one commit = one purpose") |
| 6 | Manual belief file | 2 | 2 | **Yes** — `system-owns-format` (0.87) on `parse_belief_file` | Code structure + belief annotation |
| 7 | Delete without investigating | 2 | 1 | No — belief commit appeared in A but **dropped** in B | Commit message (Treatment A only) |
| 8 | Feature flags | 1 | 1 | N/A — no specific belief exists | General engineering judgment |
| 9 | Config in YAML | 2 | 2 | No — `project-config-in-git` not retrieved | Session decision |
| 10 | Error handling everywhere | 2 | 2 | No — no specific belief exists | Session pattern (black-box module) |
| | **Totals** | **18** | **17** | **1/8 relevant beliefs surfaced** | |

### Computed Metrics

```
baseline_score = 18/20 = 0.90
belief_score   = 17/20 = 0.85
delta          = -0.05
```

**Delta is negative.** Treatment B scored *worse* than Treatment A by 1 point.

Threshold check: delta < 0.05 → **"not adding value"** per the eval criteria.

### Why Treatment A Won

Query 7 is the only score difference. Treatment A surfaced the commit message
"belief: add investigate-before-delete" as result #10 (lexical match on
"delete" + "investigate"). Treatment B's re-ranking with impact scoring dropped
this result, replacing it with a persona entry. The belief annotation system
*removed* the very signal that helped.

### Why Beliefs Didn't Surface (1/8 retrieval rate)

The impact annotation only fires when a belief is grounded to a code file that
*already appears* in the top-10 scry results. The pipeline:

1. Scry retrieves top-10 results for the query
2. If any result is a code symbol, check if a belief reaches that file
3. If yes, annotate the result with the belief

The problem: for 7/8 relevant queries, the belief's grounded code files
weren't in the top-10 results. Example: `sync-first` is grounded to code
files, but searching "async HTTP client" retrieves `CallType::Async` and
`Client::http` — neither of which is in `sync-first`'s reach set. The
semantic gap between the query and the belief is bridged by the belief's
*concept*, not by the code it's grounded to.

### Complexity Costs

| Cost | Measurement | Assessment |
|------|-------------|------------|
| DB overhead | 256 KB / 127 MB total = 0.2% | Negligible |
| Code lines | 4,162 lines / 54,368 total = 7.7% | Moderate |
| Scrape time | Belief phase in ~13s total scrape | Small fraction |
| Cognitive load | 3-hop pipeline (semantic→structural→lexical) | High — takes >5 min to explain |

The Amidi rule: 7.7% code complexity for -2.5% improvement → **cut it.**

### Honest Assessment

**The belief system does not change LLM behavior.** The numbers say stop.

But the *reason* matters for what comes next:

1. **The principles already live elsewhere.** CLAUDE.md, session decisions,
   code comments, and commit messages already encode project principles. The
   LLM finds them through these existing channels. Beliefs duplicate what's
   already accessible.

2. **The grounding pipeline solves the wrong problem.** We built precise
   code grounding (100% precision, 86% recall), but the annotation only
   fires when grounded code appears in query results. The semantic gap is
   between the *query* and the *belief concept*, not between the belief and
   the code. Grounding beliefs to code doesn't help if the code isn't what
   gets retrieved.

3. **Session history is the real knowledge carrier.** In 4/10 queries,
   session decisions/patterns provided the principle. In 2/10, CLAUDE.md did.
   In 2/10, general engineering judgment was sufficient. Session history is
   the highest-value knowledge channel — it's already indexed, already
   retrieved, and already changes behavior.

4. **Treatment B can hurt.** Query 7 shows that re-ranking with impact
   scoring can *displace* useful results. The annotation system added noise
   to an already-working retrieval pipeline.

### Kill Criteria Check

1. ✅ **delta < 0.05** → beliefs don't change behavior (delta = -0.05)
2. ✅ Precision maintained at 100% → grounding is clean (not the problem)
3. ✅ Scrape time acceptable → not too expensive (not the problem)
4. ✅ **This audit exists and we're acting on it** → criterion 4 passes

Result: **1 of 4 kill criteria triggered** (the most important one).

### Decision

**Do not build E4.6b (belief relationships).** The foundation (beliefs
changing LLM behavior) isn't demonstrated.

**Do not archive yet.** The belief *storage* is cheap (256 KB, 0.2% of DB)
and the belief *files* in `layer/surface/epistemic/beliefs/` are human-
readable project documentation regardless of whether the LLM uses them.

**What would change this result:**
- Beliefs surfacing directly in retrieval (not just as annotations on code)
- A "belief oracle" in scry that matches query intent to belief concepts
- Beliefs as a retrieval channel, not a post-hoc annotation

The grounding pipeline is technically sound but architecturally misplaced.
The value of beliefs is in their *concepts*, not in which code files they
touch. A belief should surface when the query's *intent* matches the
belief's *principle*, regardless of what code is in the results.

### Run Order (completed)

1. ✅ Run the 10-query A/B eval
2. ✅ Measure the complexity costs
3. ✅ Score honestly
4. ✅ Decision: keep storage, stop building on grounding, rethink retrieval
5. Next: consider belief-as-oracle retrieval channel if pursuing further

---

## Post-Eval Notes (2026-02-02)

### What This Eval Actually Tested

This eval tested **one narrow slice**: the E4.6a-fix grounding annotation layer
(`impact: true` vs `impact: false`). It toggled whether code results get annotated
with "which beliefs reach this file." It did NOT test:

- Whether belief files themselves help (the first eval already showed +2.2 delta)
- Whether beliefs surface as their own retrieval results via E3
- Whether the belief storage/schema/verification is sound

### Reconciling Two Evals

| Eval | What it tested | Query type | Delta |
|------|---------------|------------|-------|
| evaluation.md (Jan 2026) | Belief files accessible vs not | Knowledge-seeking ("What does the architect believe about async?") | **+2.2** |
| evaluation-next.md (Feb 2026) | Grounding annotations on vs off | Task-oriented ("Add async HTTP client") | **-0.05** |

These are **consistent**, not contradictory:
- Belief *files as structured knowledge* → high value
- Grounding *annotations on code results* → no value

The first eval proved the knowledge is worth capturing. This eval proved the
delivery mechanism (annotating code search results) doesn't work for task queries.

### The Real Problem: Retrieval, Not Design

The belief system design is sound:
- 47 beliefs, well-structured, grounded, verified (24/27 pass)
- Storage is cheap (256 KB, 0.2% of DB)
- Grounding pipeline is technically correct (100% precision, 86% recall)
- Belief files are valuable human-readable project documentation

What's broken is the **last mile delivery** — getting the right belief in front of
the LLM when it's about to make a task decision. Two specific gaps:

1. **Semantic gap**: "add async HTTP client" ≠ "sync-first" in embedding space.
   The belief exists, is indexed, is grounded — but the query doesn't match it.
2. **Annotation gap**: Impact annotations only fire when grounded code is already
   in the top-10 results. If the query pulls different code, the belief is invisible.

Both are retrieval problems. The pipe from "developer intent" to "relevant belief"
is missing. This is an **intent → principle matching** problem, not a search problem.

### What This Means for the Spec

**E4.6b (belief relationships): deprioritized.** Adding more graph structure doesn't
help if beliefs don't reach the LLM. The eval confirms this — more structure on top
of invisible data doesn't change outcomes.

**E4.6c (forge semantic): still valid.** Embedding issues/PRs is about completing
the semantic space, not about belief delivery. It's retrieval infrastructure that
benefits all of scry, not just beliefs.

**E5 (revision + cross-project): blocked on delivery.** Cross-project belief
routing via mother needs beliefs to influence LLM behavior in the first place.
The delivery problem must be solved before E5 makes sense.

**E6 (curation): premature.** Automating promotion/archival of beliefs that the
LLM can't find is optimizing the wrong layer.

**E4 steps 8-10 (schema cleanup): still valid.** Removing fake confidence signals
and cleaning the schema is housekeeping that makes the system honest regardless
of delivery.

### The Missing Layer

The eval reveals a gap not covered by E4-E6: **intent-aware belief delivery**.
The current retrieval is embedding-based (query → nearest vectors). What's needed
is something that recognizes "the developer is making a technology choice" and
pulls `sync-first` before the LLM responds — regardless of embedding distance.

This is likely a function of mother and the multi-project orchestration layer,
not something to solve inside the single-project epistemic spec. Retrieval is a
moving target being improved through skills and adapter evolution. The belief
system should focus on being correct and complete — delivery will catch up as
the orchestration layer matures.

### Revised Priorities

1. **Finish the spec as designed** — no more rabbit holes
2. **E4 steps 8-10** — schema cleanup (honest data)
3. **E4.6c** — forge embeddings (complete the semantic space)
4. **Skip E4.6b** — belief relationships deprioritized
5. **Defer E5/E6** — blocked on delivery layer (mother scope)
6. **Intent-aware delivery** — future work, mother federation scope
