---
id: concept-repo-patina
status: emerging
created: 2026-01-06
oxidizer: nicabar
tags: [concept, ref-repos, knowledge-extraction, delegate-model, codex]
references: [concept-rag-network, spec-mothership-graph]
---

# Repo Patina: Extracted Wisdom from Reference Repositories

## Key Insight

**Reference repos contain extractable wisdom beyond raw code search.**

When a user says "I want to build a dojo game", the LLM shouldn't just search dojo code. It should know:
- "v1.2 is stable, v1.3 has breaking changes"
- "Team recommends starting with World contract"
- "Common gotcha: don't forget to initialize X"

This is **repo patina** — institutional knowledge distilled from git history, issues, and code patterns.

---

## Two Types of Patina

| Type | Source | Content | Nature |
|------|--------|---------|--------|
| **Repo patina** | Git history, issues, code, docs | Objective wisdom about the repo | Facts from history |
| **User patina** | Sessions, corrections, approvals | Subjective preferences | Personal choices |

When user works with a ref repo:
```
Query result = repo patina + user patina + current context
```

---

## The Knowledge Layers

```
┌─────────────────────────────────────────────────────────────┐
│                    REF REPO (dojo)                          │
│                                                             │
│  Raw:        git history, issues, code, docs                │
│                         │                                   │
│                         ▼                                   │
│  Extracted:  "v1.2 stable, v1.3 has breaking changes"       │
│              "ECS pattern: World → Model → System"          │
│              "Common gotcha: don't forget X"                │
│              "Team recommends W over X"                     │
│                                                             │
│  ═══════════════════════════════════════════════════════    │
│  REPO PATINA - institutional knowledge, objective facts     │
└─────────────────────────────────────────────────────────────┘
                              │
                              │ user references dojo
                              ▼
┌─────────────────────────────────────────────────────────────┐
│              USER'S PROJECT (best-dojo-game)                │
│                                                             │
│  Project patina:  user's choices on THIS project            │
│                   "we use ECS style ABC"                    │
│                   "we chose v1.2 because..."                │
│                                                             │
│  User persona:    cross-project preferences                 │
│                   "terse naming", "Result types"            │
│                                                             │
│  ═══════════════════════════════════════════════════════    │
│  USER PATINA - personal preferences from corrections        │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                    THE INTERSECTION                         │
│                                                             │
│  LLM combines:                                              │
│    • Dojo wisdom: "v1.2 stable, use World → Model flow"     │
│    • User wisdom: "ABC style, terse naming"                 │
│    • Project context: "game about X"                        │
│                                                             │
│  Result: Contextualized plan that is both                   │
│          repo-correct AND user-aligned                      │
└─────────────────────────────────────────────────────────────┘
```

---

## Codex as Q&A Agent

Codex is not a batch extractor—it's an **RL-style agent** that builds Q&A documents through iterative exploration.

### The Core Loop

```
┌─────────────────────────────────────────────────────────────┐
│                    CODEX Q&A LOOP                           │
│                                                             │
│  1. QUESTION (generated from persona + project context)     │
│     "What ECS patterns does dojo use?"                      │
│     ↑ driven by: persona="prefers ECS", project="game"      │
│                                                             │
│  2. GATHER (patina tools search for evidence)               │
│     scry("ECS pattern", repo=dojo) → code hits              │
│     assay(inventory, repo=dojo) → structure                 │
│                                                             │
│  3. SYNTHESIZE (LLM creates answer from raw facts)          │
│     "dojo implements World→Model→System pattern"            │
│                                                             │
│  4. GROUND (link to actual files/commits)                   │
│     Evidence: world_contract.cairo:45, model.cairo:12       │
│                                                             │
│  5. WRITE (append to Q&A markdown document)                 │
│                                                             │
│  6. MEASURE (did this help?) ← RL reward signal             │
│                                                             │
│  Loop until: budget exhausted OR diminishing returns        │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### Where Things Live

```
Project (island)              Mothership
─────────────────            ──────────────────────────────
best-dojo-game/              ~/.patina/mother/
├── layer/core/              ├── graph.db (nodes, edges)
├── CLAUDE.md                ├── personas/
└── .patina/                 └── codex/
    └── config.toml              ├── agent.db (learning state)
         │                       └── runs/
         │                           └── dojo-20260106.md (Q&A output)
         │
         └── User launches from here
             Context: project rules + persona
```

**Key architectural point:** User launches from project (provides context). Codex runs at mothership level but anchored in project rules. Codex only works with ref repos.

### Question Generation

Questions are driven by three sources:

**1. Persona (who the user is)**
```
persona: "prefers ECS", "new to cairo"
→ "What ECS patterns does dojo use?"
→ "How do I get started with cairo in dojo?"
```

**2. Project context (why they're exploring)**
```
project: "building a game", "rust + cairo"
→ "What game-related features does dojo have?"
→ "How does rust tooling interact with dojo?"
```

**3. Templates (universal questions)**
```
→ "What is this repo?"
→ "What's the stable version?"
→ "What are the entry points?"
→ "What are common gotchas?"
```

### Grounding in Reality

**Every answer MUST link to evidence.** No evidence = no answer.

```markdown
## Q: What ECS patterns does dojo implement?

**A:** dojo implements ECS with three core concepts:
- **World**: Global state container
- **Model**: Data schema (like components)
- **System**: Logic operating on models

**Evidence:**
- World: `crates/dojo/core/src/world/world_contract.cairo:45`
- Model: `crates/dojo/core/src/model/mod.cairo:12`
- Storage: `crates/dojo/core/src/storage/storage.cairo:8`

**Confidence:** High (found in core module structure)
```

### RL-Style Design (Sutton Lens)

**State:** (persona, project_context, ref_repo, questions_asked)

**Actions:** Generate question → Search → Synthesize → Write

**Reward signals:**
- Immediate: Answer has evidence → +1
- Immediate: Covers persona interest → +1
- Deferred: User approves Q&A → +10
- Deferred: Q&A content used in session → +20

**Learning over time:** Track which questions were valuable for (project-type, persona, repo). Next similar context → ask better questions.

### Measurement (Ng Lens)

**Measurable outcomes:**
1. Groundedness: Does every answer have file/commit evidence?
2. Coverage: Did we answer questions aligned with persona?
3. Usefulness: Did user approve/use the Q&A?

**Baseline:** Without Q&A, user asks LLM, LLM searches raw, lots of iteration. With Q&A: pre-digested, grounded answers.

### Output: Markdown Q&A Document

```markdown
# dojo for best-dojo-game

**Generated:** 2026-01-06
**Persona context:** prefers ECS, new to cairo
**Project context:** building a game

---

## Q: What is dojo?

**A:** dojo is a provable game engine built on Starknet/Cairo...

**Evidence:** README.md, commit 2df8c76

**Confidence:** High

---

## Q: What ECS patterns does dojo implement?

**A:** ...

---
```

User reviews, approves sections, promotes to `layer/surface/` if valuable.

### Codex in the Graph

Codex lives as an **agent node** in the mother graph:

```
┌─────────────────────────────────────────────────────────────┐
│                      MOTHER GRAPH                           │
│                                                             │
│  ┌─────────┐                                                │
│  │ codex   │ (agent node)                                   │
│  │ agent   │                                                │
│  └────┬────┘                                                │
│       │                                                     │
│       ├── ANALYZES ──────► dojo (repo node)                 │
│       │                      │                              │
│       │                      └── HAS_WISDOM ──► Q&A doc     │
│       │                                        confidence: 0.7│
│       │                                        source: codex │
│       │                                                     │
│       └── ANALYZES ──────► usearch (repo node)              │
│                                                             │
│  ┌─────────┐                                                │
│  │ persona │ (user node)                                    │
│  └────┬────┘                                                │
│       │                                                     │
│       └── CONFIRMS ──────► Q&A doc                          │
│                            confidence: 0.7 → 0.95           │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

**Confidence escalation:**
- Codex proposes → confidence 0.5
- Evidence found → confidence 0.7
- User confirms → confidence 0.95
- User rejects → remove or mark low

---

## The Conversation Loop

The user doesn't learn the ref repo. The LLM knows it (via repo patina). The user just guides preferences.

```
User: "I want to build a dojo game"

LLM: [queries dojo repo patina]
     → "Latest is v1.3 but issues show breaking changes"
     → "Team recommends v1.2 for stability"
     → "Entry point: World contract, define Models first"

     [queries user patina]
     → "User prefers ABC style"
     → "User built cairo-chess with similar approach"

     [combines]

LLM: "I'd suggest starting with dojo v1.2 - the team recommends
     it for stability while v1.3 issues get resolved. Based on
     your ABC style from cairo-chess, we could structure it as..."

User: "Yes, and also..."  ◀── goes into PROJECT patina
```

---

## User Patina: Learning from Corrections

Over time, user patina accumulates from natural conversation:

**The gold is in corrections:**
```
User: "Great, but we do X and I want ABC style"
       ▲
       └── This is preference data. The user teaching the LLM.
```

**Approval confirms choices:**
```
User: "Yes, let's start"
       ▲
       └── Validates all choices in the proposed plan.
```

**The crux of the system:**
> "What would this user/persona choose if presented with this choice?"

Patina learns to answer this by accumulating corrections and approvals. The LLM is smart; patina just steers it toward this user's preferences on this user's data.

---

## Project-to-Repo Relationship

Each user project develops its own patina. When the project references a ref repo, the relationship shapes future interactions:

```
best-dojo-game
    │
    ├── USES dojo (ref repo)
    │     └── user's dojo choices stored in project patina
    │         "we use v1.2", "we prefer ABC ECS style"
    │
    └── SIMILAR_TO cairo-chess (prior project)
          └── patterns can transfer
```

**Future query from best-dojo-game about dojo:**
- Gets dojo repo patina (objective wisdom)
- Gets best-dojo-game project patina (user's dojo choices)
- Gets user persona (cross-project preferences)

---

## Open Questions

1. **Command name** — `patina mother explore`? `patina repo explore`? `patina learn`? Review existing commands for fit.

2. **Loop termination** — Fixed question budget? Diminishing returns detection? User can stop anytime?

3. **Storage for Q&A docs** — `~/.patina/mother/codex/runs/`? Graph nodes? Both?

4. **Approval flow** — How does user approve/reject Q&A sections? Promote to `layer/surface/`?

5. **Learning persistence** — How does Codex remember which questions worked for which (project-type, persona, repo)?

6. **Evidence quality** — What if scry returns low-level hits (function deps) but question needs high-level understanding?

7. **Multi-repo** — Can Codex explore multiple related repos in one run? (dojo + starknet-foundry)

---

## Design Principles

**Andrew Ng (measurement-first):**
- Define measurable outcomes before building
- Baseline: user without Q&A vs with Q&A
- Every feature must move a metric

**Richard Sutton (learning from experience):**
- Agent improves through usage, not hand-crafted rules
- Reward signal from user behavior (approval, usage)
- Simple loop + lots of runs beats complex upfront design

**Patina core values:**
- Projects are islands (user launches from project)
- Persona is cross-project (lives in mother)
- Codex proposes, user confirms (never auto-commit)
- Ground in reality (no evidence = no answer)

---

## Key Quotes

> "The user doesn't learn the ref repo. The LLM knows it (via repo patina). The user just guides preferences."

> "Repo patina is institutional knowledge. User patina is personal preference. The LLM combines both."

> "We're not building retrieval. We're building preference capture."

> "The gold is in the corrections. When user says 'we do X', that's preference data."

> "Codex generates questions the USER would ask (based on persona), finds answers grounded in git/code reality."

> "Patina is the layer of experience you carry between projects."

---

## First Spike

Build simplest loop to validate the approach:

1. Hardcode 5 template questions
2. For each: run scry + assay on ref repo
3. LLM synthesizes answer with evidence extraction
4. Write markdown Q&A document
5. Measure: does every answer have evidence?

If yes → add persona-driven questions, reward tracking, learning.
If no → understand why evidence extraction fails, iterate.

---

## References

- [concept-rag-network](./concept-rag-network.md) — RAG architecture, projects vs reference repos
- [spec-mothership-graph](./build/spec-mothership-graph.md) — Graph layer for cross-project awareness
- Session 20260106-130041 — Origin of this concept thread (G2.5 validation → architecture exploration)
