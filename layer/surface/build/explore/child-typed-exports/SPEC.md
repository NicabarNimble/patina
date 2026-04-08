---
type: explore
id: child-typed-exports
status: draft
created: 2026-04-08
sessions:
  origin: 20260408-064526-677971000
beliefs:
  - "[[wasi-is-foundation-not-option]]"
  - "[[children-have-agency-toys-are-capabilities]]"
  - "[[beliefs-are-the-product]]"
related:
  - wit/child/child.wit
  - sdk/patina-sdk/src/child.rs
  - src/child/internal/child.rs
  - children/
  - mother/src/pando.rs
  - layer/surface/build/feat/belief-system-hardening/SPEC.md
  - layer/surface/build/feat/multiproject-belief-share/SPEC.md
exit_criteria: []
---
# explore: Typed WIT exports for children

> Explore how to move child data-processing exports from
> `handle(string, string)` to typed WIT interfaces aligned with the
> Bytecode Alliance component model.

## Why This Exists

Session 20260408 identified that Patina is aligned with the component
model at every layer except child data exports. All children export the
same untyped `handle(action: string, payload: string)` function. This
collapses WIT's type system at the child boundary — data contracts
between children are invisible to the component model.

The alignment gap is real. But building typed interfaces now would be
premature.

## Why This Is Explore, Not Active

We originally wrote this as a refactor spec with 8 exit criteria targeting
the 6 record-processing canon children. Audit and discussion revealed
three problems with that approach:

### 1. Designing types from one example

The typed interface (`RecordProcessor`, `record-envelope`) was designed
entirely from the folder-text-to-parquet ingestion pipeline — the only
pando that exists today. The actual next work is **belief sharing across
projects**, which processes beliefs, not records. If we typed the record
boundary now, we'd immediately face `handle(string, string)` again for
belief children. We need at least two domain examples before we can
design interfaces that generalize.

### 2. Children are still being understood

Patina's MCT (Mother/Child/Toy) ecosystem is inspired by Fastly and
Cloudflare's edge compute model — local-first WASM components with
platform-provided capabilities. Mother and Toys are well understood.
But the shape of children — what makes a child maximally reusable, how
children compose across domains (records, beliefs, queries, measurements)
— is still being discovered. Locking in typed exports before
understanding the general child shape would create premature structure.

### 3. World proliferation concern

The spec proposed `child-record-processor` as a second world. But what
about `child-belief-processor`, `child-query-processor`, etc? Each
domain gets a world, a macro, a host dispatch path. This felt like
building a taxonomy from a few examples rather than a general design.

The alternative (dynamic discovery of typed exports) is more general
but less explored. This needs investigation with real examples.

## What We Learned

### The alignment gap is real

`handle(string, string)` bypasses everything the component model gives us
at the child boundary. Data contracts are JSON strings. Stream names are
hardcoded in Rust source. The component model cannot verify composition.
This is the gap between "uses WASM" and "is a component model citizen."

### The right time is after two domains

When Patina has both the record-processing pipeline AND a belief-sharing
pipeline with real children, we'll have two data points. The typed
interface design should emerge from what those domains share, not from
theorizing about one.

### handle(string, string) is the right tool for now

It gives flexibility to iterate while children are being figured out.
It's ugly from a component model perspective but correct for a boundary
where types aren't stable yet. When types stabilize across domains,
typing the boundary will be a clear win with obvious shapes.

### Mother's data-plane ownership is the big runtime shift

The most significant finding: typed child exports (`process(records)`)
imply Mother owns subscribe/ack/emit — the child becomes a pure
transform. Today children manage their own IO. This is a fundamental
execution model change that needs to be designed carefully, not bolted
onto a type system spec.

### Two worlds vs dynamic discovery is unresolved

WIT worlds require all exports. Two options were identified:
- **Two worlds** (`child` + `child-record-processor`) — simple but doesn't generalize
- **Dynamic discovery** (one world, children export extra interfaces Mother inspects) — general but unproven with wasmtime

Neither was implemented. Both need prototyping with real children.

## Design Work Completed (Preserved)

The DESIGN.md in this directory contains detailed analysis that remains
valid for when this spec is revisited:

- Shared WIT record types (`patina:record@0.1.0` shape)
- Two-world approach with `include` keyword
- Separate `register_record_processor!` macro rationale
- Data-plane ownership model (Mother owns IO for typed children)
- Host dispatch with dual bindgen
- Structured pando wiring for type validation
- WASI 0.3 stream migration path

## When to Revisit

Promote this explore to active when:

1. **A second domain has real children** — belief-sharing pando is built
   with children that process beliefs, giving us two examples of typed
   data flowing between children.
2. **The deps alignment is done** — WIT dependency management follows
   WASI/BA conventions properly (see spec: wit-deps-wasi-alignment).
3. **Child shape is understood across domains** — we know what makes a
   child reusable for records AND beliefs AND other domains.

## Related Specs

- `belief-system-hardening` — HITL proposal flow, quality gates, conflict
  resolution. The belief work that will produce the second domain example.
- `multiproject-belief-share` — cross-project belief federation. The next
  pando that needs children.
- `wit-deps-wasi-alignment` — fix WIT dependency management to use proper
  WASI packages instead of flattened deps copies.
