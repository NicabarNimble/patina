# design: mother rivet integration

## Intent

Leverage Rivet for orchestration velocity without weakening Patina's core contract model.

- Rivet provides actor/workflow/queue/scheduler primitives.
- Mother remains policy + typed invocation authority.
- Children remain WIT/WASI components.

## Core stance

1. **Integrate deeply with Rivet now** (reduce infra reinvention).
2. **Keep only minimal seams** needed to avoid hard lock-in.
3. **Do not move business contract authority out of WIT + Mother.**

## Runtime topology (v1)

```text
Rivet actor/workflow/queue trigger
          |
          v
Mother Rivet adapter ingress
          |
          v
Mother registry/policy (ingress + delivery + audit/metrics)
          |
          v
Mother typed invocation driver
          |
          v
Wasmtime component child (WIT business export)
```

## Required invariants

- A Rivet-triggered operation is transformed into the same `ChildCallRequest` path used by existing typed call surfaces.
- Existing deny/grant and delivery policy behavior remains unchanged.
- Typed call observations continue to be emitted from Mother as source of truth.
- Child code remains unchanged by orchestration backend (SDK guidance remains backend-neutral).

## Minimal seams (only three)

### 1) Rivet ingress adapter
Translate Rivet event/work item payloads into:
- target child
- operation id (`<package>:<interface>.<function>`)
- args JSON
- metadata (request id, rivet run id, attempt, trace fields)

### 2) Correlation envelope
Carry correlation fields through Mother observation records:
- `orchestrator = rivet`
- `rivet_run_id`
- `rivet_workflow_id` (optional)
- `attempt`

### 3) Profile switch
A runtime mode flag to enable/disable Rivet integration without changing standalone behavior.

## Why this is still portable

- Business contracts are still WIT.
- Execution is still Wasmtime components.
- Mother policy semantics stay canonical.

If a second orchestrator arrives later, only ingress/correlation adapters should change.

## Out-of-scope abstractions (explicit)

- No broad "universal orchestrator" trait explosion in phase 1.
- No rewrites of child manifests/contracts around Rivet concepts.
- No replacement of current typed invocation runtime with JS-native calls.

## Proof path

1. Introduce profile + ingress adapter.
2. Route one known flow (`folder-watch-actor` operations) via Rivet path.
3. Validate same outcomes in Mother typed call history/metrics.
4. Validate delivery policies in Rivet-triggered path.

## Risks and mitigations

- **Risk**: hidden Rivet coupling leaks into child contract layer.
  - **Mitigation**: forbid Rivet identifiers in WIT and child manifests.

- **Risk**: policy drift between direct and Rivet-triggered calls.
  - **Mitigation**: force both paths through registry typed call entrypoint.

- **Risk**: observability split-brain.
  - **Mitigation**: Mother remains canonical event source; Rivet IDs are correlation metadata only.
