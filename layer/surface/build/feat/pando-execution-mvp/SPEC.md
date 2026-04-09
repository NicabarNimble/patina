---
type: feat
id: pando-execution-mvp
status: draft
created: 2026-04-09
sessions:
  origin: 20260409-090238-892748000
beliefs:
  - "[[children-have-agency-toys-are-capabilities]]"
  - "[[wasi-is-foundation-not-option]]"
  - "[[projects-are-sovereign-mother-coordinates]]"
related:
  - refactor/child-typed-composition
  - feat/voice-lake-mvp1
  - feat/child-construction-canon
  - resources/pandos/folder-text-to-parquet/pando.toml
  - src/commands/mother/daemon.rs
  - mother/src/pando.rs
exit_criteria:
  - id: pe1-push-children-pure
    text: "All 6 push children are pure transforms with no upstream imports. dedup-filter and record-writer lose their patina:records/transform import. Each child's push interface is a standalone function of its inputs."
    checked: false
  - id: pe2-pando-package
    text: "patina:pando@0.1.0 WIT package exists in wit/pando/ with composed run() interfaces per stage: source, extract, transform, write, catalog. No domain types — only references to patina:records types."
    checked: false
  - id: pe3-adapters-built
    text: "6 thin pando adapter components exist. Each imports upstream patina:pando/<prev-stage> + push child patina:records/<stage>, exports patina:pando/<stage>::run(). Source adapter is special: no upstream pando import, injects folder from config capability."
    checked: false
  - id: pe4-adapters-glue-only
    text: "Every adapter is strictly: pull upstream run(), call push child, return. Zero business logic. Verified by code review — no filtering, validation, or transformation in adapters."
    checked: false
  - id: pe5-compose-and-load
    text: "Mother composes 12 components (6 push + 6 adapters) via wac-graph at runtime, encodes, and loads in wasmtime. Explicit WAC instance aliases for repeated transform stages (schema-enforcer vs dedup-filter)."
    checked: false
  - id: pe6-single-entry
    text: "Mother calls patina:pando/catalog::run() as single entry point. Full chain executes: catalog→write→transform→transform→extract→source. Data cascades through adapters and push children."
    checked: false
  - id: pe7-output-exists
    text: "After execution, parquet output exists at Mother-controlled path (not /tmp/patina/). record-writer uses filesystem preopen."
    checked: false
  - id: pe8-handle-children-work
    text: "Handle-based service children (belief-verifier, session-writer, spec-manager, doctor) continue working. Regression: patina spec list returns results."
    checked: false
  - id: pe9-parity-tests
    text: "Per-stage parity: push(fixture_input) == composed.run() output for equivalent fixtures. Mock upstream for composed path. All 6 stages pass parity."
    checked: false
  - id: pe10-e2e-test
    text: "E2E: 3 unique .txt files in temp folder, Mother calls pando/catalog::run(), verify 3 accepted records, 0 rejected, 1 parquet at Mother-controlled path with 3 rows, 1 catalog entry."
    checked: false
  - id: pe11-size-tracking
    text: "Artifact size tracked per child + adapter .wasm. Baseline recorded before and after Fix 2. No unexplained bloat."
    checked: false
---
# feat: Pando Execution MVP

## Problem

All 6 canon children are compiled to typed WIT components. wac-graph composition
validation works. But the pipeline has never executed end-to-end.

Additionally, the children's composition model is wrong: dedup-filter and
record-writer call upstream imports internally, coupling push interfaces to
composition wiring. And there is no composed execution surface — no way for
Mother to run the pipeline with a single call.

## Goal

Establish the canonical dual-surface child model (Fix 2) and make
folder-text-to-parquet run end-to-end via a single Mother call.

## Design: Fix 2 — Dual Surface with Pando Adapters

### Core Principle

Children are push-pure. Composition is handled by thin adapter components.
Mother controls outside toys. Adapters control inside chain. Business logic
lives only in push children.

### Two Surfaces Per Stage

**Push surface** (`patina:records`): Typed verbs with explicit parameters.
Standalone. Testable in isolation. No upstream imports.

```
scan(folder: string) → list<file-found>
extract(files: list<file-found>) → list<record-envelope>
transform(records: list<record-envelope>) → transform-result
write(records: list<record-envelope>) → list<file-written>
register(files: list<file-written>) → list<catalog-entry>
```

**Composed surface** (`patina:pando`): Parameterless `run()` per stage.
Only works inside a wac-graph composition. Adapters pull from upstream
and delegate to push children.

```
source::run() → list<file-found>
extract::run() → list<record-envelope>
transform::run() → transform-result
write::run() → list<file-written>
catalog::run() → list<catalog-entry>
```

### Component Architecture

```
PUSH CHILD (pure, no upstream imports):
┌──────────────────────────┐
│ schema-enforcer.wasm     │
│ imports: logging, measure│
│ exports: records/transform│
│ logic: process(records)  │
└──────────────────────────┘
           ↕ wac-graph wires
PANDO ADAPTER (thin glue, no business logic):
┌────────────────────────────────────────┐
│ schema-enforcer-pando.wasm             │
│ imports: pando/extract (upstream)      │
│ imports: records/transform (push child)│
│ exports: pando/transform::run()       │
│ logic: run() {                         │
│   records = pando::extract::run()?;    │
│   records::transform::transform(records)│
│ }                                      │
└────────────────────────────────────────┘
```

### Special Cases

**Source adapter**: No upstream `patina:pando/*` import. Imports push child
`patina:records/source` + new `patina:config` toy (read-only key/value).
Reads folder path from config, calls `source::scan(folder)`. WASI env is
local-dev fallback only — `patina:config` is the primary injection mechanism.

**Repeated transform stages**: schema-enforcer and dedup-filter both produce
`patina:pando/transform`. Use explicit WAC instance aliases to avoid wiring
ambiguity (e.g., `schema-transform` and `dedup-transform`).

### Full Composition Graph (12 components)

```
Mother composes via wac-graph:

  fsm.wasm → fsm-pando.wasm
              exports pando/source::run()
              (injects folder from config, calls records/source::scan)
                    ↓
  ce.wasm → ce-pando.wasm
             imports pando/source, imports records/extract
             exports pando/extract::run()
                    ↓
  se.wasm → se-pando.wasm  [alias: schema-transform]
             imports pando/extract, imports records/transform
             exports pando/transform::run()
                    ↓
  df.wasm → df-pando.wasm  [alias: dedup-transform]
             imports pando/transform (from schema), imports records/transform
             exports pando/transform::run()
                    ↓
  rw.wasm → rw-pando.wasm
             imports pando/transform (from dedup), imports records/write
             exports pando/write::run()
                    ↓
  lc.wasm → lc-pando.wasm
             imports pando/write, imports records/catalog
             exports pando/catalog::run()

Mother calls: pando/catalog::run()
```

### Shared Business Logic Rule

Each push child has ONE core function: `process(input) → output`. Both the
push export and the pando adapter delegate to this same logic. For push,
input comes from the function parameter. For composed, input comes from
upstream `run()`. Same core, different plumbing.

```rust
// Push child: schema-enforcer
fn process(records: Vec<RecordEnvelope>) -> Result<TransformResult, String> {
    // ALL validation logic here — the one source of truth
}

// Push export (standalone)
fn transform(records: Vec<RecordEnvelope>) -> Result<TransformResult, String> {
    process(records)
}

// Pando adapter (separate .wasm component)
fn run() -> Result<TransformResult, String> {
    let records = patina::pando::extract::run()?;
    patina::records::transform::transform(records)  // calls push child
}
```

## What Changes From Current Code

| Item | Current | After |
|---|---|---|
| dedup-filter.wasm | imports records/transform, calls upstream | Pure: no upstream import, dedup logic only |
| record-writer.wasm | imports records/transform, calls upstream | Pure: no upstream import, write logic only |
| Other 4 children | Already pure | No change to logic |
| record-writer output | Hardcoded `/tmp/patina/` | Filesystem preopen, Mother-controlled |
| Pando adapters | Don't exist | 6 new adapter components in `~/.patina/pando-adapters/` |
| `patina:pando` package | Doesn't exist | New WIT package: run() per stage |
| `patina:config` toy | Doesn't exist | New minimal read-only key/value config toy |
| Adapter crate structure | N/A | One crate per stage (6 crates) + optional adapters-common |
| pando.toml | Legacy string wiring | Typed wiring referencing 12 components |
| Mother composition | Validates only | Composes 12 components, encodes, loads, calls |

## WIT Package Layout

```
wit/
├── toys/deps/       patina:records@0.1.0 types (RecordEnvelope, TransformResult, etc.)
├── child/           patina:records@0.1.0 push interfaces (scan, extract, transform, write, register)
└── pando/           patina:pando@0.1.0 composed interfaces (run() per stage, refs shared types)
```

`patina:pando` contains ONLY composed execution interfaces. No domain types.
All types reference `patina:records`.

`patina:pipeline` is reserved — it refers to the existing grammar engine lane,
not composition.

## Non-Goals

- Voice/namespace scoping (voice-lake-mvp1)
- Federation query integration (voice-lake-mvp1)
- Per-child observability metrics verification (link measure toy, don't verify)
- Security audit logging of grant decisions
- Auto-generating adapters from pando manifest (future optimization)

## Implementation Order

1. Make push children pure — remove upstream imports from dedup-filter and
   record-writer, update their implementations
2. Fix record-writer output path — filesystem preopen instead of /tmp/patina/
3. Create `patina:pando@0.1.0` WIT package — run() interfaces per stage
4. Build 6 pando adapter components — thin glue, zero business logic
5. Update pando.toml — typed wiring for 12 components with instance aliases
6. Mother compose + load — wac-graph encodes 12 components, wasmtime loads
7. Mother linker — outside toys for composed component
8. Mother entry call — pando/catalog::run(), collect results
9. Dispatch enum — HandleBased vs Composed in Mother registry
10. Parity tests — per-stage push vs composed equivalence
11. E2E test — 3 .txt files, single entry call, verify output
12. Size tracking — baseline and post-Fix-2 artifact sizes

## Verification

```bash
cargo check --workspace -q
cargo nextest run

# Push children rebuild without upstream imports:
cargo build -p patina-ai-child-dedup-filter --target wasm32-wasip2
cargo build -p patina-ai-child-record-writer --target wasm32-wasip2

# Parity tests (per stage):
cargo test --test pando_parity

# E2E test:
cargo test --test pando_execution -- folder_text_to_parquet
# 3 unique .txt files → 3 accepted, 0 rejected
# 1 parquet at Mother-controlled path, 3 rows
# 1 catalog entry
# Output NOT at /tmp/patina/

# Handle-child regression:
patina spec list  # spec-manager (handle-based) must respond

# Adapter glue-only audit:
grep -rn "fn " adapters/*/src/lib.rs
# Each adapter: only run(), only 2-3 lines, only upstream call + push delegation
```

## Build Readiness

All foundation exists:
- 6 push children compiled to .wasm
- wac-graph dependency in Cargo.toml
- Composition graph validation in daemon.rs
- wasmtime component API available
- Existing outside toy linker code

The work: purify 2 children, create patina:pando package, build 6 adapters,
update pando manifest, compose + load + call in Mother, tests.
