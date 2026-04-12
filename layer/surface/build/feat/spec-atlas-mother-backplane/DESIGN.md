# Design: Spec Atlas Mother Backplane

## Build Target

Expose atlas snapshot through Mother API and consume it from CLI when available.

## Route

- `GET /api/atlas/snapshot`
  - auth-gated under existing Mother API routing policy
  - returns atlas snapshot JSON

## Runtime wiring

- Extend `ApiRuntime` with `atlas_snapshot()`.
- Implement in daemon `ServerState` by delegating to `commands::atlas` snapshot builder.
- Keep snapshot model identical to local atlas command output.

## CLI behavior

- Atlas command tries Mother snapshot when:
  - Mother is configured or UDS socket exists.
- On any Mother fetch/decode failure:
  - logs fallback reason
  - computes local snapshot from repository truth

## Safety

- Mother route is read-only.
- No mutation side effects.
- Fallback preserves standalone guarantees.

## Direct code targets

- `mother/src/http_routes.rs`
- `mother/src/http_api.rs`
- `src/commands/mother/daemon.rs`
- `src/mother/internal.rs`
- `src/commands/atlas/mod.rs`
- `src/commands/atlas/internal.rs`
