---
type: feat
id: core-values-format
status: ready
created: 2026-02-27
sessions:
  origin: 20260227-105623
related:
- data-mother-schema
- data-architecture-v2
beliefs:
- if-its-patina-its-git
- beliefs-are-where-machine-meets-human
exit_criteria:
- id: value-format-template-defined-and-documented
  text: value format template defined with frontmatter schema and body structure (target <1KB per value)
  checked: false
- id: existing-core-docs-reviewed-and-distilled
  text: existing core docs reviewed — each produces one or more tight value files in layer/core/values/
  checked: false
- id: scrape-pipeline-picks-up-values-deterministically
  text: scrape pipeline picks up layer/core/values/*.md as beliefs with kind "value" and entrenchment "very-high"
  checked: false
- id: values-flow-through-full-system
  text: values appear in patina.db, sync to graph.db, visible in mother search — dangling edges from data-mother-schema W2 resolved
  checked: false
- id: llm-can-create-values-from-spec-template
  text: LLM produces a valid value file from the Value Format Template section of this spec — file passes scrape, appears in patina.db, and the new value ID is recorded in DESIGN.md
  checked: false
---
# feat: Core Values as First-Class Beliefs

> Define a tight value format, distill existing core docs into it, and
> integrate into the scrape pipeline so values flow through
> patina.db → graph.db → mother search. Resolves the 124 dangling edges
> in graph.db (W2 from [[data-mother-schema]] audit).

## Current State

Core layer documents (`layer/core/*.md`) are referenced in belief `supports:`
fields but don't exist in the belief system. 5 core doc IDs are referenced
across 39 beliefs (55 wiki-link references in source markdown). At sync time,
`graph.db` materializes these as edge rows in both `belief_supports` and
`belief_attacks` tables — `delete_dangling_edges()` cleans 124 rows per sync
because edges dangle on either end (`from_belief` or `to_belief` not in
beliefs table). The most-referenced: `dependable-rust` (21 refs),
`spec-driven-design` (15), `unix-philosophy` (12), `patina-identity` (6),
`adapter-pattern` (1).

**The size problem:** Current core docs range from 848 bytes to 11KB. They're
rich reference documents with tutorials, examples, cross-language bridges,
and "How to Apply" sections. This is valuable reference material but far too
large to load every session or index as beliefs.

**What exists today:**

| Location | Kind | Size | In system? |
|----------|------|------|------------|
| `layer/core/*.md` | Rich reference docs | 848B–11KB | No — too large, no belief frontmatter |
| `layer/core/beliefs/*.md` | Tight beliefs | ~1.2KB | No — not scraped |
| `~/.patina/layer/surface/beliefs/*.md` | Persona values | ~500B | Yes — synced to graph.db |
| `layer/surface/beliefs/*.md` | Project beliefs | ~500B–2KB | Yes — scraped to patina.db |

**The gap:** No mechanism to make core architectural principles (dependable-rust,
unix-philosophy, etc.) visible to the belief system. Beliefs reference them,
edges point to them, but they don't exist as rows.

## Target State

A tight **value format** (~500–800 bytes) that captures the essential principle
without tutorial content. Values are distilled from core docs — the original
rich docs remain as reference material.

Values flow through the full system:
```
layer/core/values/*.md → patina scrape → patina.db (kind: "value")
                       → graph sync → graph.db → mother search
```

Beliefs that `supports: [dependable-rust]` now point to real rows.
Dangling edges eliminated.

## Value Format Template

```markdown
---
type: value
id: <kebab-case-id>
status: active
entrenchment: very-high
facets: [<domain-tags>]
references: [<related-value-ids>]
created: <date>
distilled_from: <source-doc-path>
---
# <Title>

<One-sentence principle. This IS the value.>

## Test

<How to check compliance. One concrete heuristic or question.
"Before adding a public type, can you state its Do-X in one sentence?">

## Consequence

<What happens when you follow/violate this. 1-3 sentences max.>
```

**Format rules:**
- Target: under 800 bytes. Hard limit: 1KB. Scrape emits
  `"warning: value '<id>' exceeds 1KB (<size> bytes)"` to stdout during
  `patina scrape` (advisory, not blocking — values are hand-curated).
- `type: value` distinguishes from `type: belief`
- `entrenchment: very-high` — values are foundational, not speculative
- `distilled_from:` links back to the rich reference doc
- Body has exactly 3 sections: statement, test, consequence
- No tutorials, examples, code blocks, cross-language bridges
- The rich source doc remains in `layer/core/` for humans who want depth

**ID rule:** Value IDs **must match** the existing belief `supports:`
targets. Beliefs reference `[[dependable-rust]]` → the value file must
use `id: dependable-rust`. This is what eliminates the dangling edges.
For multi-value docs (e.g., `patina-identity.md` → 2-3 values), one
value takes the doc-level ID (`patina-identity`) and additional values
get new IDs. No existing `supports:` references need editing — the
doc-level ID is always preserved as a value.

## Steps

1. **Define value format** — Finalize the Value Format Template above.
   This spec is the authoritative reference for the format.

2. **Review existing core docs** — Audit each core doc to determine:
   - Is it a value? (principle → distill)
   - Is it multiple values? (split → distill each)
   - Is it reference-only? (skip — e.g., `build.md` is the roadmap)
   - Record doc → value ID mappings in DESIGN.md as each is processed

   Current inventory (8 candidates, excluding `build.md`):
   | Doc | Size | Likely values |
   |-----|------|---------------|
   | `dependable-rust.md` | 5.2KB | 1: small stable interface, hide internals |
   | `unix-philosophy.md` | 4.6KB | 1: one tool, one job |
   | `safety-boundaries.md` | 848B | 1: project-scoped, user consent |
   | `adapter-pattern.md` | 7.6KB | 1: trait-based external system bridges |
   | `oxidized-knowledge.md` | 5.3KB | 1: knowledge accumulates through use |
   | `session-capture.md` | 1.6KB | 1: friction-free context capture |
   | `spec-driven-design.md` | 8.9KB | 1-2: specs are source of truth |
   | `patina-identity.md` | 11KB | 2-3: protocol core, plugin boundary, local-first |

   Also review `layer/core/beliefs/*.md` (2 files: `spec-is-milestone`,
   `temporal-layering-causes-drift`) — these are already tight but use
   `type: belief`, may want to elevate to `type: value` if they're
   foundational enough.

3. **Distill core docs to value format** — Create value files in
   `layer/core/values/`. One file per value. Each under 800 bytes.
   Original docs untouched.

4. **Add value scraping to pipeline** — Modify scrape to pick up
   `layer/core/values/*.md`:
   - Detect `type: value` in frontmatter
   - Store in patina.db beliefs table with `kind = "value"`,
     `entrenchment = "very-high"`
   - Deterministic: file exists → belief row exists. No heuristics.

5. **Verify full-system integration** — After scrape + graph sync:
   - Values appear in `patina.db` (query beliefs WHERE kind = 'value')
   - Values flow to `graph.db` via mother sync
   - `patina mother search` returns values
   - `patina mother graph sync` output shows 0 dangling edges cleaned
     (was 124). Record the sync output in DESIGN.md as evidence.
   - `patina mother search "dependable-rust"` returns the value

## Non-Goals

- **Belief-to-value promotion workflow.** Future: review beliefs, promote to
  values. For now, values are created manually by the user (with LLM help).
- **Automated value extraction.** No ML/heuristic to detect "this belief
  should be a value." User decides.
- **Belief combination/merging.** Multiple beliefs into one value — future.
- **Changing existing core docs.** Rich docs stay as reference. Values are
  distilled alongside them, not replacements.
- **Persona value migration.** The 5 persona values in
  `~/.patina/layer/surface/beliefs/` are a separate concern (user-scoped
  vs project-scoped).

## Key Files

- `layer/core/values/` — New directory for value files (this spec creates it)
- `layer/core/*.md` — Existing rich docs (read-only, distill from)
- `src/commands/scrape/beliefs/mod.rs` — Scrape pipeline (add value detection)
- `src/commands/mother/graph.rs` — Collect pipeline (already handles kind field)
