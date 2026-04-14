# DESIGN — mother-rivet-correlation-join

## Value alignment

- **spec-driven-design**: narrow, verifiable slice focused on one behavior join.
- **dependable-rust**: additive fields at stable boundaries; no cross-module leakage of Rivet-specific logic into child contracts.
- **adapter-pattern**: correlation metadata enters at Mother adapter boundary (HTTP payload), not inside child business interfaces.
- **safety-boundaries**: no extra network/system side effects; metadata only augments existing in-memory/history payloads.
- **unix-philosophy**: one small job — correlate and filter typed-call observations.

## Planned implementation

1. **Runtime types (additive metadata only)**
   - `mother/src/runtime.rs`
   - Add `CallCorrelation` and optional `correlation` to `ChildCallRequest`.

2. **Registry persistence**
   - `mother/src/registry.rs`
   - Add optional correlation field to `TypedCallObservation`.
   - Record correlation metadata for denied/success/error observation paths.
   - Add test for persisted correlation metadata.

3. **HTTP typed-call payload support**
   - `mother/src/http_api.rs`
   - Parse optional `correlation` from `/child/{name}/call` request body.
   - Pass through to runtime child-call dispatch.

4. **Inspector filter support**
   - `mother/src/http_api.rs`
   - Extend inspector request payload with optional Rivet filters.
   - Filter returned calls by correlation fields and preserve output shape.
   - Add test for filter behavior and count correctness.

5. **Daemon wiring**
   - `src/commands/mother/daemon.rs`
   - Pass optional correlation into `ChildCallRequest`.

6. **Rivet lab bridge pass-through**
   - `/Users/nicabar/Projects/Patina/rivet-deno-lab/main.ts`
   - Include correlation payload in typed call requests.

## Compatibility

- Existing callers can keep sending `{operation_id,args}`.
- Existing inspector callers can keep sending `{limit}`.
- New behavior is strictly additive.

## Risks

- Correlation shape drift between Rivet and Mother payloads.
  - Mitigation: keep explicit typed fields and deterministic tests.
- Future non-Rivet backends need correlation too.
  - Mitigation: keep field nested under generic `correlation` object; Rivet IDs are optional keys.
