# portability seam: mother rivet integration

This document freezes the minimal backend seam used in phase-1 Rivet integration.

## Goal

Allow a second orchestrator backend later **without changing**:

- child WIT contracts
- child manifests
- typed invocation driver
- WASI/toy capability model

## Backend-specific boundary (current Rivet adapter)

Only these surfaces are Rivet-specific:

1. **HTTP ingress route**
   - `POST /api/rivet/dispatch`
   - file: `mother/src/http_routes.rs`

2. **Ingress payload shape**
   - `RivetDispatchRequest`
   - `RivetDispatchDeadLetter`
   - file: `mother/src/http_api.rs`

3. **Runtime adapter hook**
   - `ApiRuntime::rivet_dispatch(...)`
   - server implementation in `src/commands/mother/daemon.rs`

Everything downstream is backend-agnostic typed-call execution:

- `ChildCallRequest` (`mother/src/runtime.rs`)
- `ChildRegistry::call(...)` policy + invocation (`mother/src/registry.rs`)
- Wasmtime child execution path

## Correlation seam

Rivet identifiers are additive correlation metadata only:

- `rivet_run_id`
- `rivet_actor_id`
- `rivet_workflow_id`
- `rivet_job_id`

They live in `CallCorrelation` and `TypedCallObservation`, and are used for inspector filtering.
Business contracts do not depend on these fields.

## Delivery-policy seam

Rivet adapter maps delivery policy values onto typed-call outcomes:

- `required` -> primary failure is returned as error
- `best-effort` -> primary failure becomes non-fatal `best-effort-skipped`
- `dead-letter` -> primary failure reroutes to configured dead-letter child

Policy language remains Patina-native (`required|best-effort|dead-letter`).

## Rule for future backend addition

A new backend should add an ingress adapter parallel to Rivet and target the same typed-call runtime boundary (`ChildCallRequest` -> `ChildRegistry::call`).
No backend-specific types should be introduced in child WIT or SDK toy APIs.
