# design: pando delivery policy

## Manifest extensions

In `mother/src/pando.rs`:

- `PandoDeliveryPolicy` enum (`required`, `best-effort`, `dead-letter`)
- `PandoTypedWiring.delivery` (optional, defaults to `required`)
- `PandoComposition.dead_letter` target:
  - `child`: destination instance id
  - `toy` (optional): override reroute toy

## Runtime behavior (`compose_typed_component`)

For each typed rule:

1. Resolve `from` instance; if missing:
   - `required` => bail
   - otherwise deny audit + continue
2. Resolve `to` instance; if missing:
   - `required` => bail
   - `best-effort` => deny audit + continue
   - `dead-letter` => try reroute via dead-letter target
3. If interface match fails between `from` and `to`:
   - `required` => bail
   - `best-effort` => deny audit + continue
   - `dead-letter` => try reroute

Dead-letter reroute writes explicit GRANT/DENY audit events with reason text indicating fallback outcome.

## Why this aligns with end-state direction

- Policy stays orchestration-level (Mother), domain behavior stays child-owned.
- No watcher-specific or record-specific branches were added.
- Fail-closed remains default (`required`).
