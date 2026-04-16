---
type: feat
id: ba-truths
status: draft
created: 2026-04-10
sessions:
  origin: 20260409-143847-707078000
related:
- layer/surface/build/feat/sdk-vision-lock/SPEC.md
- feat/sdk-developer-platform
- refactor/child-typed-composition
beliefs:
- '[[sdk-is-mct-entry-point]]'
- '[[children-have-agency-toys-are-capabilities]]'
- '[[wasi-is-foundation-not-option]]'
exit_criteria:
- id: bt1-lean-skill-spec
  text: "`sdk/ba/skills/discover-direction.md` exists and is lean: conference-first funnel, primitive extraction rules, strict stop rules, and manual-only operation."
  checked: true
- id: bt2-primitives-baseline
  text: "`sdk/ba/PRIMITIVES.md` exists and locks the specific primitives Patina aligns to (component model, WIT, WASI direction, composition model), with plain-language acceptance tests."
  checked: true
- id: bt3-conference-pass
  text: "First manual run produces `sdk/ba/DIRECTION.md` grounded first in Wasm I/O, WasmCon, and WebAssembly Workshop signals, then cross-checked against canonical BA sources."
  checked: false
- id: bt4-repo-budget
  text: "`sdk/ba/repos.toml` is generated with a hard cap (<= 12 repos total) and each repo is tagged to a primitive/theme. Repo sprawl is explicitly rejected."
  checked: false
- id: bt5-mother-repo-authority
  text: "Tracked repos are registered through Patina/Mother repo workflow (`patina repo add`/`sdk/ba/scripts/add-all.sh`) with no parallel storage system introduced."
  checked: false
- id: bt6-six-child-alignment-matrix
  text: "`sdk/ba/ALIGNMENT.md` maps the six typed baseline children to BA primitives and identifies where Patina intentionally extends beyond standards."
  checked: true
- id: bt7-first-alignment-belief
  text: "At least one `ba-aligns-*` / `ba-extends-*` / `ba-diverges-*` belief is written from a concrete source fact and tied to an SDK/runtime decision."
  checked: false
- id: bt8-manual-only
  text: "No automation (cron/hooks/scheduled jobs). Direction runs are manual until trust is explicitly raised by a later spec."
  checked: false
- id: bt9-conference-catalog
  text: "Conference intake artifacts exist: `sdk/ba/conferences/SOURCES.toml` and `sdk/ba/conferences/catalog.jsonl` with event dates, speaker/topic, keynote/workshop flags, and schedule/video/slides/repo links tied to source-confidence and primitive tags."
  checked: true
- id: bt10-reality-filter
  text: "Reality filter is enforced in skill/output: each claim has confidence (`official_schedule`/`official_video`/`community_post`) and status (`confirmed`/`inferred`/`unverified`); roadmap decisions cite confirmed evidence only."
  checked: false
---
# feat: BA Truths Foundation (Lean, Conference-First)

## Problem

The previous shape drifted toward broad repo discovery and analysis overhead.
That risks over-engineering and delays the real objective: stay aligned with the
actual primitives being defined by the WebAssembly/BA ecosystem.

## Goal

Create a **tight, low-drift BA alignment loop** that:

1. Starts from the key conference signal stream (Wasm I/O, WasmCon, WebAssembly Workshop)
2. Locks the primitives Patina cares about
3. Tracks a small, intentional repo set through Mother-owned repo workflow
4. Produces clear alignment artifacts for SDK and six-child decisions
5. Captures speaker/topic/video signals with a reality filter (so noise does not steer roadmap)

## Non-Goals

- Tracking every wasm-adjacent project
- Building an automated intelligence pipeline
- Creating new storage/index infrastructure outside Patina/Mother
- Turning this into a general ecosystem crawler

## Direction Model

### 1) Source priority (strict)

**Tier 0 (primary): standards and governance artifacts**
- WASI / component model group outputs
- Canonical proposal/design artifacts and meeting records

**Tier 1 (direction signal): core conferences**
- Wasm I/O
- WasmCon
- WebAssembly Workshop

**Tier 2 (implementation signal):**
- RustConf / FOSDEM (implementation evidence)
- KubeCon (cloud/platform pressure and adoption patterns)

**Tier 3 (research horizon, optional):**
- POPL / PLDI / ASPLOS

If Tier 0+1 do not justify a claim, it cannot drive roadmap decisions.

### 2) Primitive-first before repo-first

Patina aligns to primitives, not repo count. Baseline primitives are captured in
`PRIMITIVES.md` and drive every other artifact.

### 3) Repo budget

`repos.toml` is intentionally small (<= 12 repos total) until a follow-on spec
approves expansion.

Recommended starting set (adjustable by run evidence, still budget-capped):
- WebAssembly/component-model
- WebAssembly/WASI
- bytecodealliance/wasmtime
- bytecodealliance/wit-bindgen
- bytecodealliance/wasm-tools
- bytecodealliance/wac
- bytecodealliance/cargo-component
- (optional ecosystem evidence: Spin, wasmCloud, Extism)

### 4) Reality filter (mandatory)

Every captured item includes:
- `source_confidence`: `official_schedule | official_video | community_post`
- `status`: `confirmed | inferred | unverified`
- `primitive_tags`: e.g. `component-model`, `wit`, `wasi`, `composition`, `runtime`
- `decision_eligible`: true only when supported by confirmed official evidence

No roadmap/spec decision may cite `unverified` items as primary evidence.

## Conference Intake (new)

### Files

```
sdk/ba/conferences/
├── SOURCES.toml      # approved conference/source endpoints
└── catalog.jsonl     # normalized records (speaker/topic/video)
```

### `SOURCES.toml` intent

Defines approved inputs per event:
- official conference schedule URL(s)
- official playlist/channel URL(s)
- optional notes source(s)
- confidence defaults

### `catalog.jsonl` schema (minimum)

Each line captures one talk/session record:
- `event`, `year`, `event_date_start`, `event_date_end`
- `speaker`, `title`, `talk_type`, `is_keynote`, `is_workshop`
- `topic_summary`, `primitive_tags` (list)
- `schedule_url`, `video_url`, `slides_url`, `repo_urls`
- `source_confidence`, `status`, `decision_eligible`
- `observed_at`

This catalog feeds `DIRECTION.md` and helps detect: trend persistence, topic drop-off,
and potential project decay.

Slides are link-first. When slide content is critical, add markdown extraction
notes under `sdk/ba/conferences/slides/` (no binary dump by default).

## Artifacts

```
sdk/ba/
├── README.md
├── PRIMITIVES.md
├── DIRECTION.md
├── ALIGNMENT.md
├── COUNTER-OBSERVATIONS.md
├── repos.toml
├── conferences/
│   ├── SOURCES.toml
│   └── catalog.jsonl
├── skills/
│   └── discover-direction.md
└── scripts/
    └── add-all.sh
```

- `PRIMITIVES.md`: what we align to
- `DIRECTION.md`: current directional snapshot
- `ALIGNMENT.md`: six typed baseline children mapped to primitives + explicit deltas
- `repos.toml`: small generated support set
- `conferences/*`: normalized talk/speaker/video intake with confidence tags

## Implementation Order

1. Write lean `discover-direction.md` skill (conference-first, stop rules, manual-only).
2. Write `PRIMITIVES.md` (explicit primitive definitions + acceptance tests).
3. Add conference intake sources (`conferences/SOURCES.toml`) and normalized schema (`catalog.jsonl`).
4. Run first manual direction pass to produce `DIRECTION.md`.
5. Generate budget-capped `repos.toml` and register repos through Mother flow.
6. Write `ALIGNMENT.md` for six typed baseline children.
7. Write first BA alignment belief from concrete source evidence.

## Resolved Decisions

- Conference signal leads; repo crawling follows.
- Primitive alignment leads; inventory follows.
- Small repo budget is a feature, not a limitation.
- Mother/Patina repo workflow remains authoritative.
- Manual operation until proven trustworthy.
- Reality filter is mandatory; noisy items are retained as context, not direction truth.

## Verification

```bash
# core artifacts
test -s sdk/ba/skills/discover-direction.md
test -s sdk/ba/PRIMITIVES.md
test -s sdk/ba/DIRECTION.md
test -s sdk/ba/ALIGNMENT.md
test -s sdk/ba/repos.toml

# conference intake artifacts
test -s sdk/ba/conferences/SOURCES.toml
test -s sdk/ba/conferences/catalog.jsonl

# repo budget guard (<= 12 entries)
awk '/\[\[repo\]\]/{count++} END{print count+0}' sdk/ba/repos.toml

# Mother repo registration path remains canonical
sdk/ba/scripts/add-all.sh --dry-run

# at least one BA alignment belief exists
ls layer/surface/epistemic/beliefs/ba-{aligns,extends,diverges}-*.md 2>/dev/null | head -1
```

## Build Readiness

High. This is mostly a simplification and focus correction, not a new platform.
