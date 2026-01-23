# Signal Over Noise - Design Notes

Extended thinking and background for the signal/noise exploration.

---

## The Expanded Framing

Initial framing focused on "anti-slop" for code PRs. Expanded to recognize:

**Slop is one form of noise. The real goal is signal over noise across all surfaces.**

Patina already indexes GitHub issues. The same signal/noise problem applies there - and to discussions, docs, and any "easy to access" contribution surface.

### Noise Across All Surfaces

| Surface | Noise Forms | Why Hard to Filter |
|---------|-------------|-------------------|
| **Code PRs** | Generic changes | Syntactically correct, passes CI |
| **Issues** | Vague reports, AI feature requests | Well-formed, reasonable-sounding |
| **PR descriptions** | "Improved X" boilerplate | Grammatically correct |
| **Discussions** | Drive-by opinions | May contain partial truth |
| **Docs** | Surface rewording | Changes something |

### The Common Thread

All noise forms share: **could apply to any project**. They lack project-specific engagement.

All signal shares: **engages with project-specific knowledge**. References patterns, aligns with beliefs, builds on sessions.

### Patina's Angle

Patina captures project-specific knowledge that:
1. Isn't in public AI training data
2. Requires engagement to learn
3. Can be used to evaluate incoming content

This applies equally to issues, PRs, discussions - any surface where content can be evaluated against captured knowledge.

---

## Git Blame for Intent (Problem Framing)

*From session discussion*

### What Is Breaking Down

**1. Git Tracks Output, Not Intent**

Git excels at: line-level diffs, authors, timestamps, commit messages (optional, informal).

Git does not track: intent behind changes, constraints assumed, process used, whether author understood the code.

As AI-generated changes increase, the gap between output and intent widens.

**2. AI Introduces Authorship Ambiguity**

- Human may author the prompt, not the code
- Same prompt yields different outputs over time
- Model versions, temperatures, tools drift silently

Questions git blame can't answer:
- Who is responsible for this line?
- Written deliberately or generated incidentally?
- Was expertise applied or result accidental?

**3. Expertise Moved Upstream into Prompts**

Real expertise now appears in: prompt phrasing, constraints/exclusions, acceptance criteria, iterative steering.

These are rarely preserved in version control. Two contributors can submit identical diffs - one encodes deep understanding, one blind generation. Git cannot distinguish.

**4. Non-Determinism Breaks Historical Reasoning**

Even if prompts are saved:
- Model behavior changes across versions
- Sampling introduces randomness
- Tool outputs vary

The repository becomes a record of outcomes, not decisions.

### The Reframe

> The problem is not that AI writes code. The problem is that version control has no concept of intent, and AI amplifies that absence.

Traditional git blame: "Who last touched this line?"
Intent-aware blame: "What goal caused this line to exist?"

---

## Why Intent Capture Alone Isn't Enough

**Intent can be fabricated.**

A slop contributor could write:
```
Intent-Goal: "improve performance"
Intent-Confidence: 0.95
```
...for a change that degrades performance.

This suggests intent capture is necessary but not sufficient. Also needed:
- **Verification** - Does change serve stated intent?
- **Reputation** - Does contributor's stated intent match historical outcomes?
- **Pattern alignment** - Does change align with project beliefs?

---

## The Identity Problem

If Patina is the tool that generates "I did this correctly" signals:
```
Good actor + Patina → "followed process" signal
Bad actor + Patina → "followed process" signal
```

The tool can't distinguish its users. This shifts the problem from "did you follow good process?" to "who are you really?"

### What Actually Can't Be Faked?

Only **outcomes over time** are genuinely hard to fake:
- Did your code get reverted?
- Cause regressions?
- Become load-bearing infrastructure?
- Require constant maintenance?

This is how academic reputation works - not "did you follow scientific method?" but "did your papers replicate and compound?"

---

## Future Directions (Out of Scope for Now)

### ZK / Starknet Possibilities

*Explored in session but deferred as "too much for Patina"*

**The dream architecture:**

```
┌─────────────────────────────────────────────────────────────┐
│                    OUTCOME LAYER                            │
│  (What actually happened to contributions over time?)       │
└─────────────────────────────────────────────────────────────┘
                            ↑
┌─────────────────────────────────────────────────────────────┐
│                   REPUTATION LAYER                          │
│  (On-chain, mass-conserving, tied to outcomes)             │
│  - ZK proofs of reputation threshold                       │
│  - Slashable stake for bad outcomes                        │
└─────────────────────────────────────────────────────────────┘
                            ↑
┌─────────────────────────────────────────────────────────────┐
│                   IDENTITY LAYER                            │
│  (Proof of unique personhood, anti-sybil)                  │
│  - ZK proof of uniqueness                                  │
└─────────────────────────────────────────────────────────────┘
                            ↑
┌─────────────────────────────────────────────────────────────┐
│                   EVIDENCE LAYER (Patina)                   │
│  (Forensic record of what happened and why)                │
└─────────────────────────────────────────────────────────────┘
```

**What ZK could enable:**
- Prove reputation exceeds threshold without revealing score
- Prove contribution to N projects without revealing which
- Prove unique personhood without revealing identity
- Put stake at risk pseudonymously

**Starknet specifically:**
- STARKs = no trusted setup
- Native account abstraction
- Cairo for provable computation

### Proof of Understanding

What if you could prove "I understood the codebase" not just "I used the tool"?

```
ZK proof: "I correctly answered N questions about this codebase's
          architecture, constraints, and historical decisions
          (generated from Patina's knowledge base)
          without revealing which questions or my answers"
```

Patina's scry + beliefs become the **question generator**. ZK proves substantive engagement.

---

## Patina's Practical Role

Given the scope constraints, Patina focuses on:

**Evidence layer** - The forensic record that lets outcomes be evaluated later.

**Knowledge gate** - Project-specific knowledge that slop generators don't have and won't engage with.

**Asymmetric friction** - Make quality contributions easier (by surfacing patterns) and slop harder (by requiring engagement).

Not perfect security. Raising the floor.

---

## Leveraging Existing Patina Capabilities

### Issues Already Indexed

Patina's GitHub scraper already indexes issues. This means:
- Semantic search across issue corpus exists
- New issues can be compared to existing ones
- Duplicate/similar detection is achievable now

### Scry for Triage

```bash
# New issue: "App crashes when uploading large files"
patina scry "upload large files crash"

# Returns:
# - Issue #87: "Memory spike on file upload" (0.82)
# - Session 20250612: "Investigated upload limits" (0.71)
# - Belief: stream-not-buffer (0.68)
```

Immediately surfaces: is this new (signal) or covered (noise)?

### Beliefs as Alignment Check

```bash
# Feature request: "Add WebSocket support"
patina scry --beliefs "websocket real-time"

# Returns:
# - sync-first (0.88) - "Prefer synchronous code"
# - http-stateless (0.75) - "Avoid connection state"
```

Doesn't block - but surfaces context for evaluation.

### Sessions as "Already Explored"

```bash
# Proposal: "Use Redis for caching"
patina scry --sessions "redis caching"

# Returns:
# - Session 20250301: "Evaluated Redis vs SQLite cache"
# - Outcome: "Chose SQLite, Redis adds operational complexity"
```

Saves maintainer time - we already thought about this.

---

## Practical Next Steps

1. **Manual workflow first** - Document how to use `patina scry` for triage
2. **CONTRIBUTING.md** - Guide contributors to check before submitting
3. **Bot later** - Automate only after manual workflow proven

Start with friction that helps good contributors (easy to learn context) before building detection systems.

---

## Session Notes

Origin: 20260123-050814

Key quotes:
- "The problem is not AI-generated code - it's unattributed intent"
- "Slop is generic. Projects are specific."
- "Noise is generic. Signal engages with project-specific knowledge."
- "Outcomes over time are the only signal genuinely hard to fake"
- "Patina makes project-specific wisdom explicit and queryable"
