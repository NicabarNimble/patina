# Epistemic Layer — Design

Reference document for the academic foundations, schema definitions, and system integration
design of Patina's epistemic layer. This is the "how and why" companion to [SPEC.md](SPEC.md).

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

## Core Framing

Patina implements:

**A persona-based epistemic belief revision system using atomic Markdown propositions with non-monotonic inference rules.**

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

**Note (E4):** Confidence signals in belief frontmatter are fabricated at creation time. Phase E4
replaces them with computed metrics (use/truth counts from real data). See SPEC.md for E4 status.

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
├── VALIDATION.md                # Testing approach
├── beliefs/                     # 47 belief files (source of truth)
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

**Removed: `_index.md`** (Session 20260201-084453). The file was a manually-maintained
materialized view of belief state that drifted as the system grew from 15 to 45 beliefs.
All derived data is now computed by `patina scrape` and displayed by `patina belief audit`.
Keeping a hand-maintained summary file violated Helland's principle: derived data should not
masquerade as source data.

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

## References

- AGM Framework: Alchourrón, Gärdenfors, Makinson (1985)
- Pat Helland: "Data on the Outside vs Data on the Inside"
- [[spec-surface-layer]] - Parent spec (capture/curate framing)
- [[spec/mothership-graph]] - Cross-project relationships
