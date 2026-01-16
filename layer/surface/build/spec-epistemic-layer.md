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

### Phase E2: Belief Creation System

**Goal:** Make belief creation deterministic - system provides format, not LLM discovery.

**Key Insight:** The LLM synthesizes beliefs, but should NOT discover format through trial and error. The system provides format knowledge and validation.

#### E2 Research (Session 20260116-095954)

Explored three adapter codebases to understand extensibility mechanisms:

**Adapter Command Systems (verified via scry on ref repos):**

| Adapter | Command Format | Location | Shell Injection |
|---------|---------------|----------|-----------------|
| Claude Code | Markdown + YAML frontmatter | `.claude/commands/*.md` | `` !`cmd` `` |
| OpenCode | Markdown + YAML frontmatter | `.opencode/command/*.md` | Direct bash |
| Gemini CLI | TOML with prompt field | `.gemini/commands/*.toml` | `!{cmd}` |

**Claude Code Skills System (Oct 2025):**

Skills are auto-invoked context providers - Claude loads them when task matches description.

```
.claude/skills/skill-name/
├── SKILL.md           # Required: frontmatter (name, description) + instructions
├── scripts/           # Optional: executable code for deterministic tasks
├── references/        # Optional: documentation loaded on-demand
└── assets/            # Optional: files used in output
```

Key properties:
- **Auto-triggered**: Loaded based on description matching, not slash commands
- **Progressive disclosure**: metadata (~100 words) → SKILL.md (<5k words) → resources (unbounded)
- **Scripts are deterministic**: Shell/Python executed without loading into context
- **References on-demand**: Large docs loaded only when needed

Sources:
- [Claude Code Skills Docs](https://code.claude.com/docs/en/skills)
- [anthropics/skills GitHub](https://github.com/anthropics/skills)
- [Anthropic Engineering Blog](https://www.anthropic.com/engineering/equipping-agents-for-the-real-world-with-agent-skills)

#### E2 Design Decision: Skills over MCP

| Consideration | Skills + Shell | MCP Tool |
|---------------|---------------|----------|
| Adapter support | Claude Code only | All three |
| Format ownership | Shell script | Rust struct |
| Validation | Shell script checks | Strong typing |
| Progressive disclosure | ✅ Built-in | ❌ Not applicable |
| Implementation effort | ✅ Low (prototype done) | Medium (Rust work) |
| Learning value | ✅ New system to understand | Already using MCP |

**Decision:** Start with Skills prototype for Claude Code. MCP remains option for adapter-agnostic future.

**Rationale:**
1. Skills are the emerging standard in Claude Code
2. Progressive disclosure matches belief complexity (simple creation → detailed format)
3. Shell scripts provide sufficient validation for prototype
4. Learning the skills system has value beyond this use case
5. Can always add MCP tool later if needed

#### E2 Prototype (Implemented)

```
.claude/skills/epistemic-beliefs/
├── SKILL.md                    # Auto-loads when discussing belief creation
├── scripts/
│   └── create-belief.sh        # Validates + writes markdown
└── references/
    └── belief-example.md       # Complete format reference

.claude/commands/
└── belief-create.md            # Optional explicit /belief-create trigger
```

**The Flow:**
```
User discusses creating a belief
        │
        ▼
Claude auto-loads epistemic-beliefs skill
        │
        ▼
Claude synthesizes belief from context
        │
        ▼
Claude calls create-belief.sh with args
        │
        ▼
Script validates: id, statement, confidence, evidence
        │
        ▼
Script writes layer/surface/epistemic/beliefs/{id}.md
```

**Script Validation:**
- ID: lowercase, hyphens, starts with letter
- Statement: required, non-empty
- Confidence: required, 0.0-1.0
- Evidence: required, at least one source
- Persona: required
- Prevents overwriting existing beliefs

#### E2 Tasks

- [x] Research adapter extensibility mechanisms
- [x] Understand Claude Code skills system
- [x] Create skill prototype (`epistemic-beliefs`)
- [x] Implement validation script (`create-belief.sh`)
- [x] Add format reference (`belief-example.md`)
- [x] Create optional slash command (`/belief-create`)
- [ ] Test skill auto-triggering in real usage
- [ ] Iterate based on testing
- [ ] Document for other adapters (Gemini CLI, OpenCode)
- [ ] **Deployment gap**: Add skills to `templates.rs` for `adapter refresh`

#### E2 Deployment Note

**Current state**: Skills source files in `resources/claude/skills/` but NOT auto-deployed.

**Why**: Patina has two deployment paths:
1. `session_scripts.rs` - old internal path (not used by `adapter refresh`)
2. `templates.rs` - actual path used by `adapter refresh`

Skills need to be added to `templates.rs` → `install_claude_templates()` to be deployed automatically.

**Workaround**: Manually copy from resources:
```bash
mkdir -p .claude/skills/epistemic-beliefs/{scripts,references}
cp resources/claude/skills/epistemic-beliefs/SKILL.md .claude/skills/epistemic-beliefs/
cp resources/claude/skills/epistemic-beliefs/scripts/* .claude/skills/epistemic-beliefs/scripts/
cp resources/claude/skills/epistemic-beliefs/references/* .claude/skills/epistemic-beliefs/references/
chmod +x .claude/skills/epistemic-beliefs/scripts/*.sh
```

#### E2 Future: MCP Alternative

If adapter-agnostic creation needed:

```
LLM provides JSON → Rust struct validates → Rust writes markdown
```

Tasks (deferred):
- Rust struct defining belief fields
- `patina surface create-belief` CLI command
- MCP tool exposing `create_belief`
- `patina surface validate` for checking existing files

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

### Phase E2 Exit

- [x] Skill prototype implemented (`.claude/skills/epistemic-beliefs/`)
- [x] Validation script works (`create-belief.sh`)
- [x] Format reference available (`references/belief-example.md`)
- [ ] Skill auto-triggers correctly in real usage
- [ ] Created 3+ beliefs using the skill
- [ ] No format errors in created beliefs

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
| Beliefs | 6 |
| Rules | 3 |
| Avg Confidence | 0.885 |
| Highest Entrenchment | very-high (eventlog-is-truth) |
| Defeated Attacks | 7 |
| Active Attacks | 7 |
| Personas | 1 (architect) |
| Total Lines | ~525 |

### Belief Inventory

| ID | Confidence | Entrenchment |
|----|------------|--------------|
| sync-first | 0.88 | high |
| spec-first | 0.85 | high |
| dont-build-what-exists | 0.90 | high |
| smart-model-in-room | 0.88 | high |
| eventlog-is-truth | 0.92 | very-high |
| measure-first | 0.88 | high |

---

## Open Questions

1. **Persona explosion**: When do we need multiple personas vs facets?
2. **Confidence decay**: How fast should confidence decay for fast-moving domains?
3. **Cross-project attacks**: Can a belief in project A attack a belief in project B?
4. **Rule inheritance**: Do rules from core apply automatically to surface?
5. **Visualization**: How to visualize the argument graph? (Obsidian? Custom?)
6. ~~**Skills evolution**: Claude Code moving from custom commands to skills.~~ **RESOLVED (Session 20260116-095954)**: Skills are now the standard. Prototype implemented using skills system.
7. ~~**Heredoc limitations**: Can shell scripts handle complex belief structures?~~ **RESOLVED**: Shell script with command-line args works for belief creation. Complex structures (evidence arrays) handled as single string args, expanded in template.

---

## Evaluation: Andrew Ng Methodology

### Approach

Following Andrew Ng's measurement-driven methodology:

1. **"Show me the failure cases"** - Test where the system fails, not just where it works
2. **"Establish a baseline first"** - Measure without epistemic layer, then with
3. **"Error analysis on real examples"** - Manually examine failures to find patterns
4. **"Iterate on data, not architecture"** - Fix data gaps before adding complexity

### Evaluation Query Set

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

### Scoring Rubric

| Score | Description |
|-------|-------------|
| 1 | Wrong or no answer |
| 2 | Vague, generic answer |
| 3 | Correct but no evidence cited |
| 4 | Correct with partial evidence |
| 5 | Correct with full evidence chain |

### Results (Q1-Q10 Complete)

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

### Key Findings

**1. Error Analysis Reveals Data Gaps**

Q1 initially failed in treatment (score: 2) because `sync-first` belief was missing. The belief existed in sessions (Aug 2025) but wasn't extracted into the epistemic layer.

**Action:** Created `sync-first.md` from session-20250804-073015.
**Result:** Treatment score improved 2 → 5.

**Lesson:** Error analysis reveals data gaps, not algorithm problems.

**2. Baseline vs Treatment Differences**

| Aspect | Baseline | Treatment |
|--------|----------|-----------|
| Answer source | Scattered session fragments | Structured belief files |
| Confidence | Unknown | Explicit (0.85-0.92) |
| Evidence | Raw mentions | Weighted links |
| Exceptions | Not found | Documented attacks |
| Reasoning | Inferred | Explicit chains |

**3. Treatment Advantages**

- **Q1:** Clear statement vs inferred from fragments
- **Q2:** "No, unless exceptions" vs "probably not"
- **Q3:** Helland cited, L2 eventlog explained vs basic reasons only
- **Q4:** 4-step process with exceptions vs scattered hints

**4. Q5-Q10 Analysis (Session 20260116-080414)**

| Query | Finding |
|-------|---------|
| Q5 (Attack awareness) | Treatment provided specific attack (analysis-paralysis), confidence (0.3), and scope ("only when spec exceeds 1 week") vs generic risks |
| Q6 (Reasoning chain) | Largest delta (+3.0) - two-belief reasoning chain with defeated attacks and phased approach |
| Q7 (Cross-belief) | Rule derivation explicit: measure-first + spec-first → implement-after-measurement |
| Q8 (Exceptions) | Specific criteria (20 lines, security urgency) vs vague "it depends" |
| Q9 (Confidence) | Full signal breakdown impossible without epistemic layer - largest treatment advantage |
| Q10 (Missing belief) | Only non-5 score: graceful gap acknowledgment + related belief inference |

**5. Strongest Treatment Advantages (Q5-Q10)**

- **Exception handling (Q8, +3.0)**: Explicit exceptions with criteria impossible to know without documentation
- **Confidence assessment (Q9, +3.0)**: Signal breakdown only available from frontmatter - baseline cannot answer
- **Reasoning chain (Q6, +3.0)**: Multi-belief chains with defeated alternatives show reasoning process

**6. Gap Identified (Q10) - Error Analysis**

Q10 scored 4 instead of 5 because no explicit belief exists for SQLite vs Postgres. The system correctly:
1. Acknowledged the gap
2. Found related beliefs (eventlog-is-truth implies SQLite)
3. Suggested creating a new belief

**Deep analysis:** Evidence for SQLite preference IS scattered in the codebase:
- `sync-first` mentions "SQLite queries (single-threaded is fine)"
- `rqlite-architecture` was defeated and "migrated to SQLite"
- `eventlog-is-truth` mentions "patina.db" architecture

**Two paths forward:**
1. **Create explicit belief:** `sqlite-preferred` or `local-first-storage` to make decision explicit
2. **Enhance search:** Treatment could search WITHIN belief bodies, not just belief IDs

**Lesson:** The epistemic layer handles gaps gracefully, but:
- Coverage matters for max score
- Information scattered across beliefs is less valuable than explicit beliefs
- Error analysis reveals opportunities to extract implicit decisions into explicit beliefs

### Success Criteria Assessment (Final)

| Metric | Target | Actual (Q1-Q10) | Status |
|--------|--------|-----------------|--------|
| Avg Epistemic Score | >= 4.0 | 4.9 | ✅ Pass |
| Avg Delta | >= 1.0 | +2.2 | ✅ Pass |
| Epistemic wins | >= 7/10 | 10/10 (100%) | ✅ Pass |
| Full evidence (score 5) | >= 5/10 | 9/10 (90%) | ✅ Pass |

### Validated Hypothesis

> **"Can an LLM correctly explain WHY a decision was made, with traceable evidence?"**

**Without epistemic layer:** Guesses or fragments from sessions (avg 2.7)
**With epistemic layer:** Cites beliefs, evidence chains, exceptions (avg 4.9)

**Conclusion:** The epistemic layer provides measurable improvement (+2.2 points average) in LLM reasoning quality about project decisions. All 10 queries showed improvement, with 9/10 achieving maximum score (5.0).

### Evaluation Complete

All success criteria met. Ready to proceed to Phase E2 (Schema Validation).

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
