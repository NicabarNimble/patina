---
type: explore
id: monty-patina-sandbox-alignment
status: draft
created: 2026-04-02
sessions:
  origin: 20260402-120613-136366000
related:
  - layer/surface/build/feat/child-construction-canon/SPEC.md
  - layer/surface/build/feat/cloudflare-worker-child/SPEC.md
  - layer/surface/build/refactor/pando-vocabulary-alignment/SPEC.md
  - https://github.com/pydantic/monty
exit_criteria:
  - Clear map of Monty primitives to Patina primitives (authority, capability grants, execution model, host mediation)
  - Decision on which Monty ideas Patina should adopt directly vs adapt to Mother+child+toy constraints
  - Prioritized follow-on specs proposed with sequencing after currently open spec commitments
  - Security and latency tradeoff statement written for "Patina vs typical sandbox" positioning
---
# explore: monty goals vs patina goals sandbox alignment

> Understand where Monty and Patina share a north star, where they intentionally diverge, and which Monty patterns should inform Patina's next specs once current open work is closed.

## Question

How should Patina's Mother + child + toy WASI architecture learn from Monty's
interpreter-first sandbox model without collapsing Patina into a Python runtime
project?

## Why now

Recent work moved secrets authority into Mother and reinforced the toy-grant
model. In parallel, Monty provides a strong reference for fast, controlled
agent code execution. The overlap is meaningful, but active in-flight specs
should finish first; this exploration captures direction so we can sequence the
next wave intentionally.

## Findings (initial snapshot)

### Monty (reference repo + talk summary)

- Monty positions itself as a minimal secure Python interpreter written in Rust,
  explicitly for agent-written code execution.
- Security model is built from a restricted core outward: host-sensitive actions
  do not exist by default and are exposed only through controlled external
  function calls.
- Runtime can suspend at external calls, return function name/args, and resume
  later with host-provided values; snapshots can be serialized for long-running
  workflows.
- It emphasizes code-mode economics: very fast startup, tight iterative loops,
  and practical typecheck feedback in the execution loop.

### Patina (current canon + active direction)

- Patina defines Mother as authority boundary, children as WASM compute, and
  toys as explicit capability grants (`[needs].toys` + optional scopes).
- Security and control live in Mother mediation plus manifest-readable grants,
  not in broad host/shell access from child code.
- Active architecture direction is one child model with toybox-determined
  behavior (not kind-based branching), plus WASI-first interfaces.
- Patina's value center is composable multi-child systems and durable governance
  over capability surfaces, not a single-language interpreter runtime.

### Shared north star, different layer focus

- Shared: least privilege, explicit host mediation, portable runtime ambitions,
  and alternatives to heavy container sandbox complexity.
- Different: Monty optimizes execution of agent-authored Python code; Patina
  optimizes orchestrated capability systems across reusable children.

## Scope

In scope:
- Compare architecture and threat model: Monty interpreter boundary vs Patina
  WASM boundary.
- Compare capability mediation models: external function callbacks/snapshots vs
  `[needs].toys` grants + Mother host enforcement.
- Identify portability, latency, and operational differences vs container
  sandboxes.
- Produce concrete spec candidates for Patina after open spec work completes.

Out of scope:
- Replacing Patina child runtime with Python interpreter execution.
- Building a Monty compatibility layer in this cycle.
- Expanding open spec scope before current commitments are complete.

## Current hypotheses

1. Monty and Patina share the same control philosophy (whitelist capabilities,
   mediate all host access), but optimize different layers.
2. Patina's unique value is multi-child orchestration and capability governance;
   Monty's unique value is ultra-low-latency code-mode execution with
   snapshot/resume.
3. The strongest learning path is targeted adoption of execution patterns
   (suspend/resume ergonomics, typecheck feedback loops, strict supported-subset
   contracts) inside Patina's Mother-governed model.

## Comparative lens

### Dimension A: security boundary
- Monty: restricted Python subset, host calls suspended and brokered.
- Patina: WASM children with toy grants, Mother mediates capability openings.

### Dimension B: capability surface
- Monty: external function registry as callable surface.
- Patina: explicit toybox (`[needs].toys` + optional scopes) as platform API.

### Dimension C: runtime model
- Monty: embedded interpreter, REPL-centric, code execution first.
- Patina: component orchestration, event composition, system behavior first.

### Dimension D: portability
- Monty: Rust core embeddable across host languages.
- Patina: WIT/WASI component portability across hosts/runtimes.

### Dimension E: economics
- Monty: microsecond startup path for frequent iterative runs.
- Patina: lower integration complexity and stronger long-lived governance over
  child capabilities.

## Expected outputs

1. A direct primitive map (Monty <-> Patina) with non-equivalences called out.
2. A short "north star overlap" statement for project messaging.
3. A sequenced proposal list for next specs (post-open-spec phase), likely:
   - Mother-managed execution snapshot/resume exploration
   - Child execution feedback/type-contract loop improvements
   - Sandbox positioning and threat-model documentation

## Primitive mapping seed (v0)

| Monty primitive | Patina analog | Gap to resolve |
|---|---|---|
| External function registry | Toy host interfaces + grant manifest | Improve ergonomics for per-run callable context and error feedback |
| Suspend/resume snapshot at call boundary | Mother session/lifecycle + child event boundaries | Add explicit execution checkpoint contract for long-latency actions |
| Restricted Python subset contract | Child/toy capability contract + spec gates | Formalize "supported subset" messaging for child behaviors |
| Embedded single-process execution loop | Mother-mediated multi-child runtime | Decide where low-latency code-mode fits without weakening governance |
| Built-in typecheck loop | Spec checks + compile/runtime checks | Add tighter type/contract feedback in execution UX |

## Open questions

1. Where should Patina draw the line between "toy" and "execution runtime
   feature" when importing Monty-like ideas?
2. Should Patina support a dedicated code-mode child profile for ultra-fast
   iterative execution, or keep orchestration-first defaults?
3. Which latency targets are materially necessary for Patina objectives vs
   "nice-to-have"?
4. How should we express "subset truth" contracts for children/toys so model
   expectations stay reliable?

## Recommended next step

Keep this as exploration until current open spec work is closed, then promote
the highest-value outcome to a feat spec with explicit sequencing and gates.

## Sequencing gate

- Do not start implementation from this explore while `child-construction-canon`
  remains the active primary spec (`patina spec next`).
- Re-open this exploration for promotion when current open commitments are
  explicitly closed or deprioritized by spec workflow.
