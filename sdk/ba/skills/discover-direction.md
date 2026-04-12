# Skill: Discover BA Direction (Lean, Conference-First)

> Purpose: produce a high-signal BA direction snapshot for Patina with minimal noise and explicit confidence tagging.

## Mission

Generate/refresh `sdk/ba/DIRECTION.md` using a strict funnel:

1. Standards/governance truth (WASI + component model artifacts)
2. Core conference signal (Wasm I/O, WasmCon, WebAssembly Workshop)
3. Implementation evidence (selected repos already tracked by Patina/Mother)

Do **not** optimize for coverage. Optimize for direction clarity.

## Required Inputs

- `sdk/ba/PRIMITIVES.md`
- `sdk/ba/conferences/SOURCES.toml`
- `sdk/ba/conferences/catalog.jsonl`
- `sdk/ba/conferences/README.md`
- `sdk/ba/repos.toml`
- Patina repo/search tools (`patina context`, `patina scry`, `patina assay`)

## Source Priority (hard rule)

- **Tier 0:** standards/governance artifacts (`WebAssembly/WASI`, `WebAssembly/component-model`, relevant meeting notes)
- **Tier 1:** core conferences (`Wasm I/O`, `WasmCon`, `WebAssembly Workshop`)
- **Tier 2:** implementation evidence (small canonical repo set)
- **Tier 3:** secondary ecosystem and research context (non-decisive)

No roadmap conclusion may rely primarily on Tier 3.

## Reality Filter (mandatory)

Each claim must include:

- `source_confidence`: `official_schedule | official_video | community_post`
- `status`: `confirmed | inferred | unverified`
- `decision_eligible`: `true` only if supported by confirmed official evidence

`unverified` claims may be listed as context but cannot drive decisions.

## Process

### Step 1 — Re-orient in project truth

```bash
patina context --topic "ba alignment"
```

Capture active beliefs/specs that this run may affect.

### Step 2 — Refresh conference intake (lean metadata only)

For each tracked event/year, capture only:
- event dates
- speakers
- talk titles/topics
- keynote/workshop flags
- schedule/video links
- slides link and repo link where available

Update `sdk/ba/conferences/catalog.jsonl` with confidence/status fields.

If slides are relevant, add markdown extraction note:
- `sdk/ba/conferences/slides/<event>/<year>/<slug>.md`

Do not dump large binary files by default.

### Step 3 — Extract primitive signals

For each primitive in `PRIMITIVES.md`:
- collect 1-3 strongest confirmed signals from Tier 0/1
- add supporting implementation evidence from tracked repos when available

Use Patina tools for grounding:

```bash
patina scry "<primitive query>" --all-repos
patina assay search "<primitive query>" --all-repos
```

### Step 4 — Produce direction snapshot

Write/update `sdk/ba/DIRECTION.md` with:
1. generated date + operator
2. sources used (grouped by tier)
3. primitive-by-primitive status
4. strengthened/weakened signals since prior snapshot
5. open questions (explicitly `unverified`)
6. candidate repo adds/removals (budget-constrained)

### Step 5 — Repo budget check

- Keep `sdk/ba/repos.toml` at **<= 12 repos** unless explicitly approved elsewhere.
- Every repo must map to a primitive/theme.

### Step 6 — Belief handoff

If this run changes a design assumption, propose at least one belief:
- `ba-aligns-*`
- `ba-extends-*`
- `ba-diverges-*`

with concrete source citations.

## Stop Rules

Stop and report if:
1. Source confidence cannot be established for key claims
2. Repo additions would exceed budget without explicit approval
3. Claims conflict with high-confidence beliefs and cannot be reconciled
4. More than 3 unresolved open questions remain

## Output Contract

- Update/create: `sdk/ba/DIRECTION.md`
- Update/create: `sdk/ba/conferences/catalog.jsonl`
- Optional update: `sdk/ba/repos.toml` (budget constrained)
- Optional recommendation: new/updated BA belief file

## Reviewer Checklist

- Are date/speaker/title/keynote/workshop fields captured for relevant talks?
- Are slides/repo links captured when available?
- Is every major claim tagged with confidence + status?
- Are conclusions driven by Tier 0/1 evidence?
- Is repo set still intentional (not sprawling)?
