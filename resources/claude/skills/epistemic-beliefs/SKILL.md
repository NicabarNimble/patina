---
name: epistemic-beliefs
description: Guide for creating and managing epistemic beliefs in Patina. Use this skill when synthesizing project decisions into formal beliefs, when the user says "create a belief", "add belief", "capture this as a belief", or when distilling session learnings into the epistemic layer. Beliefs capture project decisions with evidence and support/attack relationships. IMPORTANT - Proactively suggest belief capture when you notice design decisions, repeated patterns, strong principles, or statements like "we should always", "never do X", "the right way is". Do not wait for magic words.
---

# Epistemic Beliefs

Create formal beliefs that capture project decisions with evidence and reasoning.

## Proactive Belief Detection

**Do not wait for the user to say "create a belief".** Watch for:

| Pattern | Example | Action |
|---------|---------|--------|
| Design decision | "We should use sync, not async" | Suggest: "Capture as belief?" |
| Repeated principle | Said 3+ times in session | Suggest: "This keeps coming up..." |
| Strong preference | "Never do X", "Always Y" | Suggest: "This sounds like a core belief" |
| Contradiction found | Conflicts with existing belief | Ask: "This contradicts X - revise?" |
| Lesson learned | "That was a mistake because..." | Suggest: "Capture to avoid repeating?" |

When you notice these patterns, **ask the user**:
> "This sounds like a belief worth capturing: '{statement}'. Should I create it?"

If user confirms, proceed with belief creation. If user declines, move on.

## When to Create Beliefs

- User explicitly requests: "create a belief", "add this as a belief"
- **You notice a design decision or principle** (proactive)
- **A pattern is repeated multiple times** (proactive)
- Distilling session learnings into persistent knowledge
- Capturing architectural decisions with justification
- Recording design principles that guide future work

## Belief Creation Process

### Step 1: Gather Information

Before creating a belief, ensure you have:
- **Statement**: One clear sentence expressing the belief
- **Evidence**: At least one source — use `[[wikilinks]]` for verifiable references (e.g., `[[session-20260131-150141]]`)
- **Persona**: Usually "architect" for project-level decisions

**Do NOT guess a confidence score.** Confidence is computed by `patina scrape` from real data:
- **Use metrics**: How many other beliefs cite this? How many sessions reference it?
- **Truth metrics**: How many evidence links? Do they resolve to real files? Defeated attacks?

### Step 2: Use the Creation Script

Execute the belief creation script with required fields:

```bash
.claude/skills/epistemic-beliefs/scripts/create-belief.sh \
  --id "belief-id-here" \
  --statement "One sentence belief statement" \
  --persona "architect" \
  --evidence "[[session-YYYYMMDD-HHMMSS]] - description (weight: 0.9)" \
  --facets "domain1,domain2"
```

The script will:
1. Validate all required fields
2. Generate proper YAML frontmatter (no fake confidence scores)
3. Create the belief file in `layer/surface/epistemic/beliefs/`
4. Report success or validation errors

### Step 3: Enrich the Belief

After creation, edit the file to add:
- **Additional evidence links** — use `[[wikilinks]]` so `patina scrape` can verify them
- **Supports relationships** — beliefs this supports (e.g., `[[measure-first]]`)
- **Attacks relationships** — beliefs this defeats
- **Attacked-By relationships** — known challenges to this belief
- **Applied-In examples** — concrete code/architecture where this belief was applied

The more connections you add, the higher the computed use/truth metrics will be.

### Step 4: Run Scrape

After creating and enriching, run `patina scrape` to compute metrics. The belief will show up in `patina scry` with real numbers.

## Belief Format Reference

See `references/belief-example.md` for the complete format.

Key fields:
- **id**: Lowercase, hyphenated identifier (e.g., `sync-first`)
- **persona**: Epistemic agent (usually `architect`)
- **facets**: Domain tags (e.g., `rust`, `architecture`)
- **entrenchment**: `low`, `medium`, `high`, or `very-high`
- **status**: `active`, `scoped`, `defeated`, or `archived`
- **endorsed**: `true` if user explicitly created or confirmed

## How Metrics Work (E4)

Metrics are computed by `patina scrape`, not guessed by the LLM:

| Metric | What it measures | Source |
|--------|-----------------|--------|
| `cited_by_beliefs` | Other beliefs referencing this one | Cross-reference belief files |
| `cited_by_sessions` | Sessions mentioning this belief | Cross-reference session files |
| `applied_in` | Concrete applications listed | Count ## Applied-In entries |
| `evidence_count` | Evidence entries | Count ## Evidence entries |
| `evidence_verified` | Evidence [[wikilinks]] that resolve to real files | File existence check |
| `defeated_attacks` | Attacks this belief survived | Count ## Attacked-By with status: defeated |
| `external_sources` | Non-project evidence (papers, docs) | Evidence without session wikilinks |

**A strong belief** has high use (many citations) AND high truth (verified evidence).
**A weak belief** has low use and unverified evidence — it's an assertion, not yet tested.

## Validation Rules

The creation script enforces:
- ID must be lowercase with hyphens only
- Statement must be non-empty
- At least one evidence source required
- Persona must be specified
- `--confidence` is accepted but ignored (deprecated)
