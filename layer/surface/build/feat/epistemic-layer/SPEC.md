---
type: feat
id: epistemic-layer
status: in_progress
created: 2026-01-16
updated: 2026-01-22
sessions:
  origin: 20260116-054624
related:
  - layer/surface/build/feat/v1-release/SPEC.md
---

# feat: Epistemic Markdown Layer

**Progress:** E0 ✅ | E1 (in progress) | E2 ✅ | E2.5 ✅ | E3 ✅ | E4 (steps 1-7 ✅, steps 8-10 remaining) | E4.5 (exploring) | E5-E6 (planned)
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
├── VALIDATION.md                # Testing approach
├── beliefs/                     # 45 belief files (source of truth)
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
materialized view of belief state — statistics, inventory tables, argument graph, attack
graph — that drifted as the system grew from 15 to 45 beliefs. All derived data is now
computed by `patina scrape` and displayed by `patina belief audit`. Process documentation
(belief creation, enrichment) lives in `SKILL.md`. Academic grounding (AGM framework) lives
in this SPEC. Keeping a hand-maintained summary file violated Helland's principle: derived
data should not masquerade as source data. The file was actively misleading (showed removed
`--confidence` flag, reported 15 beliefs when 45 exist).

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
- [x] Test skill auto-triggering in real usage
- [x] Iterate based on testing
- [ ] Document for other adapters (Gemini CLI, OpenCode)
- [x] **Deployment gap**: Add skills to `templates.rs` for `adapter refresh`

#### E2 Deployment Note

**Status**: ✅ RESOLVED (Session 20260116-105801)

Skills are now embedded in `templates.rs` and deployed via `patina adapter refresh claude`.

**Implementation**:
- `resources/claude/skills/` → `include_str!()` in `claude_templates` module
- `install_claude_templates()` creates `.claude/skills/` structure
- `copy_dir_recursive` handles deployment (same pattern as commands)

**Behavior**:
- Patina-managed skills (e.g., `epistemic-beliefs`) are overwritten on refresh
- User custom skills (created directly in `.claude/skills/`) survive refresh

#### E2 Testing Results (Session 20260117-072948)

**Status**: ✅ COMPLETE

**Test Method:**
- Created 5 beliefs through natural conversation without explicit skill invocation
- Tested both explicit script calls and auto-triggering
- Validated format consistency across all beliefs

**Beliefs Created:**
1. `error-analysis-over-architecture` (0.88) - Andrew Ng methodology
2. `commit-early-commit-often` (0.90) - Git discipline pattern
3. `project-config-in-git` (0.85) - CI config tracking lesson
4. `session-git-integration` (0.87) - Session-git workflow integration
5. `phased-development-with-measurement` (0.89) - G0/E0 measurement-first pattern

**Key Findings:**

1. **Auto-triggering is silent**: Skills load contextually without visible notification
   - Validated by consistent format adherence without explicit invocation
   - Progressive disclosure worked: SKILL.md → references on-demand
   - No format errors across 5 beliefs

2. **Script validation effective**:
   - All required fields enforced (id, statement, confidence, evidence, persona)
   - ID format validation caught errors (lowercase, hyphens only)
   - Confidence range validation (0.0-1.0)
   - Overwrite prevention worked

3. **Enrichment pattern emerged**:
   - Base belief created by script (minimal)
   - Manual enrichment adds: multiple evidence sources, relationships, applied-in examples
   - Two-step process works well: creation → enrichment

4. **Format consistency validated**:
   - Zero broken wikilinks after enrichment
   - All beliefs follow reference format
   - Confidence signals generated automatically
   - Revision logs initialized correctly

**Result:** All E2 exit criteria met. System ready for production use.

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

### Phase E2.5: Session-Belief Loop (Next)

**Goal:** Make belief capture visible through the session lifecycle.

**Key Insight:** The adapter LLM creates beliefs silently during sessions via the E2 skill. But we have no visibility into whether capture is happening. Session-end should measure it, session-start should recall it.

#### The Loop

```
SESSION (during)              SESSION END                    SESSION START (next)
      │                            │                              │
      ▼                            ▼                              ▼
LLM creates beliefs ──────► Count + summarize ──────► Recall previous beliefs
(via skill, silent)         to active-session.md       from last-session.md
```

#### Session-End Capture Format

Add to active-session.md before archiving:

```markdown
## Beliefs Captured: 2
- **commit-early-commit-often**: Make small, focused commits frequently rather than batching changes into large commits
- **project-config-in-git**: Track project configuration in git, separate from machine-specific settings
```

Format:
- Count of beliefs created this session
- One line per belief: `**{id}**: {one-liner summary}`
- One-liner comes from the line after `# {id}` in belief file

#### Session-Start Recall

When reading last-session.md, surface the beliefs:

> Previous session captured **2 beliefs**:
> - `commit-early-commit-often`: Make small, focused commits...
> - `project-config-in-git`: Track project configuration...

This creates a feedback loop:
- **0 beliefs** repeatedly → LLM not recognizing capture opportunities
- **N beliefs** → Active capture working
- **Mismatch** → User remembers decisions not captured → improve triggers

#### Finding Beliefs by Session

Beliefs link back via evidence:
```markdown
## Evidence
- [[session-20260117-104322]] - discovered during CI debugging (weight: 0.8)
```

Query: `grep -r "session-{id}" layer/surface/epistemic/beliefs/`

#### E2.5 Tasks

- [x] Update `session-end.sh` to count beliefs created since session-start tag
- [x] Update `session-end.sh` to extract one-liner summaries
- [x] Update `session-end.sh` to write "Beliefs Captured" section
- [x] Update `session-start.sh` to read and surface previous session's beliefs
- [x] Test the loop across 2-3 sessions

#### E2.5 Exit Criteria

- [x] Session-end captures belief count + summaries
- [x] Session-start recalls previous session's beliefs
- [x] Format is grep-able (beliefs findable by session ID)

#### E2.5 Implementation Notes (Session 20260117-205031)

**session-end.sh (lines 112-152):**
- Uses `git diff --name-only ${SESSION_TAG}..HEAD` to find modified belief files
- Extracts belief ID and statement from frontmatter
- Appends "## Beliefs Captured: N" section to session file

**session-start.sh (lines 181-202):**
- Reads `last-session.md` to find archived session path
- Parses "## Beliefs Captured:" section from archived session
- Displays: "📝 Previous session 'X' captured N belief(s):" or "no beliefs captured"

**Feedback loop value:**
- 0 beliefs repeatedly on architecture sessions → skill not triggering
- 0 beliefs on bug fixes → expected, fine
- N beliefs → system learning organically

---

### Phase E3: Scry Integration (COMPLETE - Session 20260122-220957)

- [x] Index beliefs in oxidize pipeline
- [x] Query beliefs via scry
- [x] Return beliefs in MCP `scry` tool

**Implementation (2026-01-22):**

1. **Belief Scraper** (`src/commands/scrape/beliefs/mod.rs`):
   - Parses beliefs from `layer/surface/epistemic/beliefs/`
   - Creates `beliefs` table and `belief_fts` for lexical search
   - Extracts: id, statement, persona, facets, confidence, entrenchment, status

2. **Oxidize Integration** (`src/commands/oxidize/mod.rs`):
   - `BELIEF_ID_OFFSET = 4_000_000_000` (after commits at 3B)
   - Embeds belief statement + persona + facets + confidence
   - 22 beliefs indexed in semantic projection

3. **Scry Enrichment** (`src/commands/scry/internal/enrichment.rs`):
   - Handles belief ID range in semantic results
   - Returns `belief.surface` event type
   - Format: `{statement} [confidence: X%, entrenchment] (file_path)`

**Commits:**
- `884eef2a feat(scrape): add belief scraper for epistemic layer (E3)`
- `9aeceff3 feat(scry): index and retrieve beliefs in semantic search (E3)`

**Test Results:**
```
patina scry "what do we believe about async"
→ [2] Score: 0.853 | belief.surface | sync-first
   Prefer synchronous, blocking code over async in Patina. [confidence: 88%, high]

patina scry "what do we believe about specs"
→ [2] Score: 0.879 | belief.surface | truthful-specs
→ [8] Score: 0.852 | belief.surface | spec-first
```

### Phase E4: Computed Belief Metrics (Use & Truth)

**North Star:** Replace LLM-fabricated confidence scores with metrics computed from real data. A belief's strength comes from how much it's used and how grounded it is in evidence — not from an LLM picking 0.88.

**Problem:** Current `confidence.signals` are fabricated at creation time by `create-belief.sh` (lines 104-115: adds 0.05 to score for "evidence", hardcodes survival=0.50, endorsement=0.50). These numbers are fiction. Meanwhile, real data exists in the system — sessions cite beliefs, beliefs reference each other, evidence links point to real commits and sessions — but nothing counts it.

**Principle:** Measure, don't guess. (Andrew Ng: "Show me the data, not the model.")

#### Two Axes: Use and Truth

| Axis | Question | Signals |
|------|----------|---------|
| **Use** | Is this belief doing work? | Citations by other beliefs, citations by sessions, scry query returns |
| **Truth** | Is the evidence real? | Evidence link count, verified links (resolve to files), defeated attacks, applied-in entries |

**Why not time/survival?** A belief sitting unchallenged for 60 days isn't stronger — it might just be ignored. Use is active. Truth is verifiable.

#### Computed Schema (replaces `confidence.signals`)

```yaml
metrics:                        # Computed by `patina scrape`, not by LLM
  use:
    cited_by_beliefs: 9         # other beliefs referencing this in Supports/Attacks
    cited_by_sessions: 4        # sessions mentioning this belief ID
    applied_in: 3               # entries in ## Applied-In section
  truth:
    evidence_count: 9           # entries in ## Evidence section
    evidence_verified: 7        # evidence [[wikilinks]] that resolve to real files
    defeated_attacks: 1         # Attacked-By entries with status: defeated
    external_sources: 1         # evidence from outside the project (papers, docs)
endorsed: true                  # user explicitly created or confirmed
```

No composite score. No 0.88. Just counts that tell you *why* to trust (or question) a belief.

**Example — what this reveals:**
- `measure-first`: cited_by_beliefs=9, cited_by_sessions=4, evidence=9 → load-bearing, well-grounded
- `spec-drives-tooling`: cited_by_beliefs=0, cited_by_sessions=1, evidence=0 → assertion, not yet tested

#### Build Steps

- [x] 1. Compute use/truth metrics in belief scraper (`src/commands/scrape/beliefs/mod.rs`)
  - Cross-reference beliefs table against sessions, other belief files, layer/ patterns
  - Count evidence links, verify wikilink resolution, count Applied-In entries
  - Store in new columns on `beliefs` table
- [x] 2. Update scry enrichment to surface computed metrics instead of fake confidence
  - Replace `[confidence: 88%, high]` with `[use: 9+4 | truth: 9/7 | 1 defeated]`
  - Format: `[use: {belief_citations}+{session_citations} | truth: {evidence}/{verified} | {defeated} defeated]`
- [x] 3. Drop fake confidence signals from `create-belief.sh`
  - Remove `confidence.score` and `confidence.signals` from creation template
  - Keep: id, statement, persona, facets, entrenchment, status, endorsed
  - Confidence is computed, not declared
- [x] 4. `patina belief audit` command — show all beliefs ranked by use/truth
  - Table output: belief ID, use metrics, truth metrics, warnings
  - Warnings: 0 evidence links, unverified wikilinks, no session citations, no Applied-In
  - Like `patina doctor` but for the epistemic layer
- [x] 5. Backfill evidence provenance — every evidence line must cite its source session
  - 17 unverified evidence lines across 13 beliefs backfilled via `git log` and `git tag --contains`
  - Target achieved: 81/81 evidence verified (100%)
- [x] 6. Fix verifier: `[[commit-*]]` via `git rev-parse`, broader reference recognition
  - Added `git rev-parse` for `[[commit-HASH]]` wikilinks
  - Fuzzy session matching: `[[session-20260105]]` → finds `layer/sessions/20260105-*.md`
- [x] 7. Update `create-belief.sh` to auto-attach current session ID to every evidence line
  - Reads active session from `.patina/local/active-session.md`
  - Every evidence line gets `[[session-{id}]]:` prefix at creation time
  - Prevents future provenance gaps
- [ ] 8. Update existing 44 belief files — remove fake confidence.signals block
  - Batch migration: strip `confidence:` block from YAML frontmatter
  - Add `endorsed: true` for beliefs created via explicit user request
- [ ] 9. Update belief scraper to read new schema (handle both old and new format during migration)
- [ ] 10. MCP `context` tool includes belief metrics in pattern context

#### E4 Exit Criteria

- [x] All belief metrics computed from real data (zero fabricated numbers)
- [x] `patina belief audit` shows use/truth for all beliefs
- [x] Scry results display computed metrics instead of fake confidence
- [x] `create-belief.sh` no longer generates confidence.signals
- [x] Cross-reference data queryable: "which beliefs have no evidence?" "which are most cited?"
- [x] 100% evidence verification — 81/81 evidence lines trace to real files/sessions/commits
- [x] Verifier handles `[[commit-*]]` and fuzzy session IDs
- [x] `create-belief.sh` auto-attaches session provenance to evidence lines
- [ ] Fake `confidence.signals` removed from all belief files
- [ ] MCP `context` tool surfaces belief metrics

### Phase E4.5: Belief Verification — Connecting Beliefs to Ingredients

**Promoted to standalone spec:** See [[belief-verification]] (`layer/surface/build/feat/belief-verification/SPEC.md`)

E4.5 was expanded from "add SQL queries to beliefs" into a full measurement-driven design after
running a 10-belief x 6-layer evidence experiment (Session 20260201-084453). Key findings: SQL +
Assay are the strong proof layers (not just SQL), scry has a lexical routing bug that must be
fixed before it's useful for verification, and process beliefs correctly have no structural proof.
The original design context below is preserved; the new spec contains the full build plan.

**Original insight (Session 20260131-210617):** The `patina scrape` pipeline already builds a rich knowledge database (function_facts, call_graph, code_search, commits, sessions, co_changes, import_facts, type_vocabulary — ~46K rows total). Beliefs have 95 belief-to-belief links and 43 session links, but almost zero links to this structural data. The data exists. The beliefs don't query it.

**Andrew Ng framing:** Three levels of evidence quality:
1. **Testimony** — "We discussed this in a session." (What beliefs have now — 84% of evidence.)
2. **Artifact** — "This commit removed tokio." (4 commit links total across all beliefs.)
3. **Measurement** — "Zero async functions across 1,591 functions." (Data exists in DB. No belief uses it.)

**Helland framing (Data on the Outside vs Inside):**

Beliefs currently reference **outside data** (session testimony — non-deterministic, unreproducible) but not **inside data** (patina.db tables — deterministic, re-derivable). That's backwards from where trust should come from.

The boundary map:

```
Human decision  →  LLM conversation  →  Belief file     →  Verification query  →  DB result
  (outside)         (outside)           (captured)          (captured)             (derived)
  unreproducible    non-deterministic    source of truth     deterministic          materialized view
                                         in git              re-runnable            rebuilt every scrape
```

Each arrow is a boundary crossing. Each crossing either **captures** (writes to durable store) or **derives** (computes from existing data). Key principle: **derived data never flows backwards** — verification results live in the DB (materialized view), never written back to belief files (source of truth). This is why the scraper never modifies belief files: it would mix derived data into the source of truth.

The verification query itself crosses a non-deterministic boundary (LLM generates SQL) but once committed to the belief file, it becomes deterministic and re-runnable. This is the capture-at-boundary pattern applied to structural evidence.

**Two evidence types, complementary by design:**

Not all beliefs are structurally testable — and that's correct. Beliefs about **code structure** (sync-first, eventlog-is-truth) live at the deterministic inside — SQL verification queries are the right evidence. Beliefs about **process** (measure-first, spec-first) live at the human decision boundary — session testimony IS the right evidence. Forcing SQL queries onto process beliefs mixes levels. The two types complement each other; neither replaces the other.

**Design Decision: Option C (LLM generates once, scraper runs mechanically)**

Three options considered (Session 20260131-210617):

| Option | Approach | Cost at scrape time | Who writes queries |
|--------|----------|--------------------|--------------------|
| A | Human writes queries in belief file manually | Zero (SQL only) | Human |
| B | Convention-based mapping (belief ID → hardcoded queries in Rust) | Zero (SQL only) | Developer in Rust |
| **C** | **LLM generates queries at creation/enrichment; scraper re-runs them** | **Zero (SQL only)** | **LLM once, then mechanical** |

**Why C:** The expensive part (deciding what to query) happens once during conversation. The cheap part (running SQL) happens every scrape. No LLM inference at scrape time. The LLM speaks SQL fluently and understands the belief's intent — better positioned than a hardcoded mapping. Queries live in the belief file (portable, auditable), not buried in Rust code.

**Why not A:** Requires the user to know the DB schema and write SQL by hand. Friction kills adoption.
**Why not B:** Every new belief needs a Rust code change. Doesn't scale. Couples belief content to scraper implementation.

#### Proposed Format

A new `## Verification` section in belief markdown files:

```markdown
## Verification

```verify label="No async functions in codebase" expect="= 0"
SELECT COUNT(*) FROM function_facts WHERE is_async = 1
```

```verify label="No tokio imports" expect="= 0"
SELECT COUNT(*) FROM import_facts WHERE import_path LIKE '%tokio%'
```
```

- Fenced code blocks with `verify` info-string
- `label` = human-readable assertion, `expect` = comparison (`= 0`, `> 5`, `>= 1`, `< 10`)
- SQL must be SELECT-only (scraper validates)
- Result = first column of first row compared against expectation

#### Available Tables for Queries

| Table | Rows | What it knows |
|-------|------|---------------|
| `function_facts` | 1,591 | is_async, is_public, is_unsafe, return_type, parameter_count |
| `call_graph` | 18,909 | caller → callee edges, file, line_number |
| `code_search` | 2,955 | all symbols (functions, structs, enums) with file:line |
| `commits` | 1,520 | sha, message, author, timestamp |
| `commit_files` | — | files changed per commit, lines added/removed |
| `sessions` | 551 | title, branch, classification, files_changed |
| `co_changes` | 20,634 | files that change together |
| `import_facts` | — | import paths, imported names |
| `type_vocabulary` | — | structs, enums, traits with visibility and usage count |
| `patterns` | 111 | layer patterns (core, surface) with status and tags |

#### Example Queries for Real Beliefs

**sync-first** (structurally testable — strong candidate):
- `function_facts WHERE is_async = 1` expect `= 0`
- `import_facts WHERE import_path LIKE '%tokio%'` expect `= 0`
- `import_facts WHERE import_path LIKE '%rusqlite%'` expect `>= 1`

**eventlog-is-truth** (structurally testable):
- `call_graph WHERE callee LIKE '%insert_event%'` expect `> 10`
- `sqlite_master WHERE name = 'eventlog'` expect `>= 1`

**commit-early-commit-often** (measurable from git data):
- avg files per commit expect `< 10`
- total commits expect `> 100`

**measure-first** (process principle — harder to test structurally):
- Can verify measurement infrastructure exists, but can't verify the principle itself
- ~15 of 44 beliefs have meaningful structural tests; the rest are process/principle beliefs

#### Open Questions

1. ~~**How many beliefs are structurally testable?**~~ **RESOLVED (Helland analysis, Session 20260131-210617):** Not all beliefs are structurally testable — and that's correct by design. Code-structure beliefs (sync-first, eventlog-is-truth) get SQL verification queries. Process beliefs (measure-first, spec-first) stay grounded in session testimony. Two evidence types, complementary, not competing. Estimate: ~15 of 44 beliefs are candidates for structural verification.

2. **Skill context size.** Exposing the full DB schema to the LLM is a lot of tokens. Should the schema be a progressive-disclosure reference file the skill loads on demand? Or inline in SKILL.md?

3. **What does failure mean? Two distinct failure modes:**
   - **World changed:** `sync-first` query returns 1 because someone added an `async fn`. The belief is now contested by its own structural evidence. The audit warns; automated revision is E5's job.
   - **Query is wrong:** `eventlog-is-truth` query checks `callee = 'insert_event'` but call_graph stores full paths like `crate::eventlog::insert_event`. Query returns 0, but the belief is true. This is measure-the-measurement — fix the query, not the belief.

   Both are normal operations. Query revision (fixing a bad query) is expected — the LLM's SQL generation crossed a non-deterministic boundary and may need correction. Errors (bad SQL, missing table) are counted separately from failures (query ran, expectation not met).

4. **Verification vs Applied-In.** Currently `## Applied-In` is free text describing where beliefs manifest. Verification queries test structural claims. Are these complementary or overlapping? Should Applied-In entries become verifiable too?

5. **Query staleness.** If the DB schema changes (table renamed, column dropped), queries break. Errors are counted separately from failures. Is that sufficient, or do queries need versioning?

6. **Incremental vs full.** Verification queries depend on the full DB state (all tables populated). In incremental scrape mode, should we skip verification? Or always run it since the queries are cheap?

#### Implementation Notes

**Primary file:** `src/commands/scrape/beliefs/mod.rs` — parsing, execution, DB schema, integration
- New structs: `VerificationQuery { label, sql, expect }`, `Expectation` enum (Eq/Gt/Ge/Lt/Le/Ne)
- New fields on `BeliefMetrics`: `verification_total`, `verification_passed`, `verification_failed`
- New functions: `parse_verification_queries()`, `validate_query_safety()`, `run_verification_queries()`
- Integration point: Phase 2.5 in `run()` — after `cross_reference_beliefs()` (line ~688), before insertion loop (line ~690). All other tables are populated at this point because the belief scraper runs last in the pipeline.
- DB migration: 3 new `INTEGER DEFAULT 0` columns on `beliefs` table (same pattern as E4 metric columns)

**Audit file:** `src/commands/belief/mod.rs` — display changes
- Extend `BeliefRow` with 3 new fields, add `V-OK` column (passed/total), add `verification-failing` warning

**Skill files:** `.claude/skills/epistemic-beliefs/SKILL.md` + `resources/claude/skills/epistemic-beliefs/SKILL.md`
- New section teaching LLM the format, available tables, and when to add queries
- Schema reference as progressive-disclosure content (loaded on demand, not always in context)

#### Build Steps (tentative — pending exploration)

- [ ] 1. Implement `parse_verification_queries()` and `Expectation` in belief scraper
- [ ] 2. Implement `validate_query_safety()` (SELECT-only enforcement)
- [ ] 3. Implement `run_verification_queries()` execution against DB
- [ ] 4. Add `verification_total/passed/failed` to BeliefMetrics + beliefs table
- [ ] 5. Integrate into scraper `run()` as Phase 2.5 (after cross-reference, before insert)
- [ ] 6. Update `patina belief audit` to show V-OK column and verification-failing warning
- [ ] 7. Update skill SKILL.md with verification query format + available tables
- [ ] 8. Add `## Verification` to 3-4 proof-of-concept beliefs (sync-first, eventlog-is-truth, commit-early-commit-often)
- [ ] 9. End-to-end test: scrape → audit → verify results display correctly

#### E4.5 Exit Criteria

- [ ] Scraper parses and executes `## Verification` queries from belief files
- [ ] Safety: only SELECT queries execute; DML/DDL rejected
- [ ] `patina belief audit` shows verification pass/total per belief
- [ ] At least 3 beliefs have live verification queries passing
- [ ] Skill teaches LLM how to write verification queries with correct format
- [ ] Open questions above resolved or explicitly deferred

---

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

### Phase E2 Exit (✅ COMPLETE - Session 20260117-072948)

- [x] Skill prototype implemented (`.claude/skills/epistemic-beliefs/`)
- [x] Validation script works (`create-belief.sh`)
- [x] Format reference available (`references/belief-example.md`)
- [x] Skill auto-triggers correctly in real usage
- [x] Created 3+ beliefs using the skill (5 created)
- [x] No format errors in created beliefs

### Phase E3 Exit (✅ COMPLETE - Session 20260122-220957)

- [x] `patina scry "what do we believe about X"` returns beliefs
- [x] Beliefs appear in MCP tool results (via `patina serve` MCP server)
- [x] Belief embeddings in usearch index (22 beliefs in semantic.usearch)

### Phase E5 Exit

- [ ] Conflicting belief triggers revision flow
- [ ] LLM proposes resolution
- [ ] User can approve/reject
- [ ] Revision logged in L2 eventlog

---

## Current Prototype Statistics (Updated 2026-01-31)

| Metric | Value |
|--------|-------|
| Beliefs | 44 |
| Rules | 3 |
| Highest Use (cited_by_beliefs) | measure-first (10) |
| Highest Use (cited_by_sessions) | sync-first (6) |
| Most Evidence Links | error-analysis-over-architecture (12) |
| Highest Entrenchment | very-high (eventlog-is-truth) |
| Personas | 1 (architect) |
| Indexed in Semantic | ✅ 44 beliefs in usearch |
| Queryable via Scry | ✅ Verified working |
| Evidence Verified | ✅ 81/81 (100%) — E4 steps 5-7 complete |
| Confidence Scores | ❌ Fabricated in 43/44 files — E4 step 8 pending |
| Structural Evidence | ❌ Zero beliefs link to code/DB — E4.5 exploring |

### Belief Inventory (Top 10 by Confidence)

| ID | Confidence | Entrenchment |
|----|------------|--------------|
| eventlog-is-truth | 0.92 | very-high |
| dont-build-what-exists | 0.90 | high |
| commit-early-commit-often | 0.90 | high |
| phased-development-with-measurement | 0.89 | high |
| sync-first | 0.88 | high |
| smart-model-in-room | 0.88 | high |
| measure-first | 0.88 | high |
| error-analysis-over-architecture | 0.88 | medium |
| session-git-integration | 0.87 | high |
| spec-first | 0.85 | high |

---

## Open Questions

1. **Persona explosion**: When do we need multiple personas vs facets?
2. **Confidence decay**: How fast should confidence decay for fast-moving domains?
3. **Cross-project attacks**: Can a belief in project A attack a belief in project B?
4. **Rule inheritance**: Do rules from core apply automatically to surface?
5. **Visualization**: How to visualize the argument graph? (Obsidian? Custom?)
6. **Signal vs noise curation**: How to distinguish valuable beliefs from noise as corpus grows?
   - Usage tracking needed (like Mother's edge_usage)
   - Review triggers for low-usage, stale, or attacked beliefs
   - Four-tier curation: automated signals → usage tracking → review triggers → human curation
   - Phase E3 (scry integration) prerequisite for usage data
7. ~~**Skills evolution**: Claude Code moving from custom commands to skills.~~ **RESOLVED (Session 20260116-095954)**: Skills are now the standard. Prototype implemented using skills system.
8. ~~**Heredoc limitations**: Can shell scripts handle complex belief structures?~~ **RESOLVED**: Shell script with command-line args works for belief creation. Complex structures (evidence arrays) handled as single string args, expanded in template.
9. ~~**Skill auto-triggering validation**: Does the skill system actually work in practice?~~ **RESOLVED (Session 20260117-072948)**: Skills auto-trigger silently via contextual loading. Validated by creating 5 beliefs with consistent format adherence. Progressive disclosure works: metadata → SKILL.md → references on-demand.
10. **Structural evidence gap (Session 20260131-182129)**: Beliefs have 95 belief-to-belief links and 43 session links but almost zero links to source code, call graph, or function facts — despite all that data existing in `patina.db`. E4.5 explores verification queries as the mechanism to close this gap.
11. **Which beliefs are structurally testable?** Early estimate: ~15 of 44 beliefs make testable claims about code structure. The rest are process/principle beliefs (measure-first, spec-first) where structural testing is indirect at best. Is partial coverage acceptable?

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
- [[spec/mothership-graph]] - Cross-project relationships
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
