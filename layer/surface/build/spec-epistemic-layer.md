# Spec: Epistemic Markdown Layer

**Status:** Active (Prototype Built)
**Created:** 2026-01-16
**Origin:** Session 20260116-054624, external LLM collaboration on academic grounding
**Prototype:** `layer/surface/epistemic/`

---

## North Star

> Patina is not a note system. It is epistemic infrastructure for LLM collaboration.

Most knowledge systems store entities, facts, documents. Patina stores **beliefs, justifications, revisions, and reasoning**.

---

## Core Framing

Patina implements:

**A persona-based epistemic belief revision system using atomic Markdown propositions with non-monotonic inference rules.**

---

## Academic Grounding

### Key Domains

| Domain | Patina Application |
|--------|-------------------|
| **Epistemology** | Beliefs with justification and revision |
| **Knowledge Representation** | Markdown as belief substrate |
| **AGM Belief Revision** | Expansion, contraction, revision operations |
| **Non-Monotonic Reasoning** | Defeasible rules, exceptions |
| **Argumentation Theory** | Support/attack graphs via wikilinks |

### AGM Framework (Alchourrón, Gärdenfors, Makinson, 1985)

Operations on belief sets:
- **Expansion**: Add belief (no conflict)
- **Contraction**: Remove belief (defeated or obsolete)
- **Revision**: Add belief + remove conflicts (minimal damage)

### Key Terms

| Academic Term | Patina Equivalent |
|--------------|-------------------|
| Epistemic agent | Persona |
| Belief state | Persona belief vault |
| Epistemic entrenchment | Belief strength (how costly to remove) |
| Partial meet revision | Minimal belief damage |
| Defeasible rules | Rules that can fail (exceptions) |
| Argument graph | Wikilink network |

---

## Mapping to Existing Patina

| Patina Today | Epistemic Equivalent |
|--------------|---------------------|
| **Persona** | Epistemic agent |
| **Session observations** | Evidence / justification |
| **layer/core/** | High-entrenchment beliefs (proven over time) |
| **layer/surface/** | Active belief set (working memory) |
| **layer/dust/** | Contracted/defeated beliefs (archived) |
| **Connection scoring** | Justification strength |
| **Importance scoring** | Epistemic entrenchment |
| **Mother graph** | Cross-project argumentation network |

### The Three-Layer Lifecycle IS AGM

```
layer/surface/  ──────────────────────────────────────────────
       │                                                      │
       │ EXPANSION                              CONTRACTION   │
       │ (new belief)                           (defeated)    │
       ▼                                              │       │
   ┌─────────┐     REVISION (conflict)           ┌────▼────┐  │
   │ Belief  │ ◄────────────────────────────────►│  Dust   │  │
   │  Added  │                                   │(archive)│  │
   └────┬────┘                                   └─────────┘  │
        │                                                     │
        │ HIGH ENTRENCHMENT (proven over time)                │
        ▼                                                     │
   ┌─────────┐                                                │
   │  Core   │ ◄──────────────────────────────────────────────┘
   │(eternal)│   (resurrection: dust → surface if re-validated)
   └─────────┘
```

---

## Schema Definitions

### Belief File

```yaml
---
type: belief
id: <unique-identifier>
persona: <epistemic-agent>
facets: [<domain-tags>]
confidence:
  score: <0.0-1.0>
  signals:
    evidence: <0.0-1.0>
    source_reliability: <0.0-1.0>
    recency: <0.0-1.0>
    survival: <0.0-1.0>
    user_endorsement: <0.0-1.0>
entrenchment: <low|medium|high|very-high>
status: <active|scoped|defeated|archived>
extracted: <ISO-date>
revised: <ISO-date>
---

# <belief-title>

<One-sentence statement of the belief>

## Statement

<Expanded explanation of what this belief means>

## Evidence

- [[source-1]] - description (weight: 0.X)
- [[source-2]] - description (weight: 0.X)

## Supports

- [[other-belief-1]]
- [[other-belief-2]]

## Attacks

- [[attacked-belief]] (status: defeated|scoped, reason: "...")

## Attacked-By

- [[attacker-belief]] (status: active|defeated, confidence: 0.X, scope: "...")

## Applied-In

- [[concrete-application-1]]
- [[concrete-application-2]]

## Revision Log

- YYYY-MM-DD: Event description (confidence: old → new)
```

### Rule File

```yaml
---
type: rule
id: <unique-identifier>
persona: <epistemic-agent>
rule_type: <explicit|synthesized|heuristic>
confidence: <0.0-1.0>
derived_from: [<belief-ids>]
status: <active|suspended|deprecated>
extracted: <ISO-date>
---

# rule: <rule-title>

## Conditions

- [[belief-1]] (confidence > X.X)
- [[belief-2]] (confidence > X.X)

## Conclusion

<What follows from the conditions>

## Rationale

<Why this rule makes sense>

## Exceptions

- [[exception-1]] - when this exception applies
- [[exception-2]] - when this exception applies

## Applied-In

- [[application-1]]
- [[application-2]]

## Evidence

- [[supporting-source]]

## Revision Log

- YYYY-MM-DD: Event description
```

---

## Confidence vs Entrenchment

These are orthogonal dimensions:

| Dimension | Question | Changes When |
|-----------|----------|--------------|
| **Confidence** | How justified is this belief? | New evidence, attacks resolved |
| **Entrenchment** | How costly to remove? | Time, usage, dependencies |

A belief can be:
- **High confidence, low entrenchment**: New, well-supported, not yet proven over time
- **Low confidence, high entrenchment**: Old assumption, many things depend on it

**Revision strategy**: When conflicts arise, prefer removing low-entrenchment beliefs even if confidence is similar.

---

## Confidence Signals

| Signal | Source | Description |
|--------|--------|-------------|
| `evidence` | Links to sessions/commits | Strength of supporting evidence |
| `source_reliability` | Trusted repo vs external | How reliable is the source? |
| `recency` | Timestamps | Decays for fast-moving domains |
| `survival` | Time without attack | How long unchallenged? |
| `user_endorsement` | Explicit confirmation | User explicitly validated |

**Composite score**: Weighted average (configurable weights).

---

## Attack Resolution

Three-layer decision policy:

### 1. Deterministic Guardrails (Cheap, Predictable)

- Pinned/endorsed beliefs resist removal
- Higher evidence strength resists attack
- Higher entrenchment resists removal

### 2. Adapter LLM Arbitration (Structured, Logged)

LLM proposes one of:
- Keep both (with scopes)
- Weaken one (add qualifiers)
- Split into cases
- Replace one

Must output: justification + proposed edits.

### 3. User Override (Rare, Explicit)

- Approve proposed revision
- Mark belief as "policy" (immune to attack)
- Force contraction

**Key design move**: Allow beliefs to be **scoped** instead of deleted.

Example: "Library X is stable" → "Library X is stable under low concurrency"

---

## Rule Derivation

### Sources (Tagged with Provenance)

| Type | Authority | Example |
|------|-----------|---------|
| **Explicit** | Highest | User-defined, team policy |
| **Synthesized** | Medium | LLM distilled from observations |
| **Heuristic** | Lowest | Pattern detection, co-occurrence |

### Promotion Thresholds

Rules are hypotheses until:
- Evidence count ≥ N
- Conflict rate ≤ X%
- User endorsement OR survival ≥ Y days

---

## Revision Triggers

### Primary Triggers

1. **Contradicting belief arrives** (direct attack)
2. **Evidence invalidated**
   - Commit reverted
   - Issue closed as "not a bug"
   - Benchmark superseded
3. **Scope change**
   - Dependency version bump
   - Architecture pivot

### Secondary Triggers

4. **Time-based decay review** (scheduled for fast-moving domains)
5. **Explicit user action** ("re-evaluate this belief set")

### Revision Output

- Patch plan (belief edits)
- Rule impact list (rules to update)
- Changelog entry (what changed and why)

---

## Directory Structure

```
layer/surface/epistemic/
├── _index.md                    # Graph overview, statistics
├── VALIDATION.md                # Testing approach
├── beliefs/
│   ├── spec-first.md
│   ├── dont-build-what-exists.md
│   ├── smart-model-in-room.md
│   ├── eventlog-is-truth.md
│   └── measure-first.md
└── rules/
    ├── implement-after-measurement.md
    ├── use-adapter-for-synthesis.md
    └── capture-at-boundary.md
```

---

## Argument Graph (Implicit from Links)

Wikilinks create the argument graph without a graph database:

| Section | Edge Type |
|---------|-----------|
| `## Evidence` | Justification (belief ← source) |
| `## Supports` | Support (belief → belief) |
| `## Attacks` | Attack (belief ⚔ belief) |
| `## Attacked-By` | Attack (belief ⚔ belief) |
| `## Conditions` | Rule antecedent (rule ← belief) |
| `## Exceptions` | Defeasibility (rule ← exception) |
| `## Applied-In` | Grounding (belief → concrete) |

---

## Integration with Existing Patina

### L2 Eventlog

Surface belief operations become eventlog events:

```
surface.belief.expand    {belief_id, persona, statement, evidence, confidence}
surface.belief.contract  {belief_id, reason, defeated_by}
surface.belief.revise    {belief_id, old_confidence, new_confidence, trigger}
surface.belief.attack    {attacker_id, target_id, outcome}
surface.rule.derive      {rule_id, conditions, conclusion}
surface.rule.apply       {rule_id, context, result}
```

### Scry Integration

- Index beliefs in semantic search
- Query: "What do we believe about async?" → returns relevant beliefs
- Beliefs appear in results with `type: belief`

### Oxidize Integration

- Embed belief statements
- Belief-to-belief similarity for attack/support discovery
- Belief-to-session similarity for evidence linking

### Mother Integration

- Cross-project belief attacks
- Persona beliefs transfer between projects
- Rule propagation via graph edges

---

## Implementation Phases

### Phase E0: Prototype (COMPLETE)

- [x] Directory structure created
- [x] 5 belief files from real sessions
- [x] 3 rule files derived from beliefs
- [x] Index with argument graph
- [x] Validation approach documented

### Phase E1: Manual Population

- [ ] Extract 15 more beliefs from sessions
- [ ] Add evidence links to real sessions/commits
- [ ] Derive 5 more rules from belief clusters
- [ ] Test revision scenario (add conflicting belief)

### Phase E2: Schema Validation

- [ ] JSON Schema for belief frontmatter
- [ ] JSON Schema for rule frontmatter
- [ ] CLI validator: `patina surface validate`
- [ ] Link integrity checker

### Phase E3: Scry Integration

- [ ] Index beliefs in oxidize pipeline
- [ ] Query beliefs via scry
- [ ] Return beliefs in MCP `scry` tool

### Phase E4: Extraction Automation

- [ ] `patina surface capture` extracts beliefs from sessions
- [ ] Connection scoring finds evidence links
- [ ] Adapter LLM synthesizes rules from patterns

### Phase E5: Revision Automation

- [ ] Conflict detection on new belief
- [ ] Entrenchment calculation
- [ ] Adapter LLM proposes resolution
- [ ] User approval flow

### Phase E6: Curation Automation

- [ ] Importance scoring based on usage
- [ ] Promotion: surface → core
- [ ] Archival: surface → dust
- [ ] Resurrection: dust → surface

---

## Validation Criteria

### Phase E1 Exit

- [ ] 20+ beliefs in epistemic layer
- [ ] 10+ rules derived
- [ ] All beliefs have ≥1 evidence link
- [ ] Zero broken wikilinks
- [ ] Manual revision tested

### Phase E3 Exit

- [ ] `patina scry "what do we believe about X"` returns beliefs
- [ ] Beliefs appear in MCP tool results
- [ ] Belief embeddings in usearch index

### Phase E5 Exit

- [ ] Conflicting belief triggers revision flow
- [ ] LLM proposes resolution
- [ ] User can approve/reject
- [ ] Revision logged in L2 eventlog

---

## Current Prototype Statistics

| Metric | Value |
|--------|-------|
| Beliefs | 5 |
| Rules | 3 |
| Avg Confidence | 0.886 |
| Highest Entrenchment | very-high (eventlog-is-truth) |
| Defeated Attacks | 5 |
| Active Attacks | 5 |
| Personas | 1 (architect) |
| Total Lines | 446 |

---

## Open Questions

1. **Persona explosion**: When do we need multiple personas vs facets?
2. **Confidence decay**: How fast should confidence decay for fast-moving domains?
3. **Cross-project attacks**: Can a belief in project A attack a belief in project B?
4. **Rule inheritance**: Do rules from core apply automatically to surface?
5. **Visualization**: How to visualize the argument graph? (Obsidian? Custom?)

---

## References

- [[spec-surface-layer]] - Parent spec (capture/curate framing)
- [[spec-mothership-graph]] - Cross-project relationships
- [[session-20260116-054624]] - This session
- AGM Framework: Alchourrón, Gärdenfors, Makinson (1985)
- Pat Helland: "Data on the Outside vs Data on the Inside"

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
