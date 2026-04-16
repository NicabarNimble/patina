# Design: Atlas Native Mother Spec Lens

## Architectural intent

Atlas is a Mother-native control-plane lens for spec state.
It is not a general MCT inventory command.

## Source of truth

- Spec files in `layer/surface/build/**/SPEC.md`
- Spec parser/model from existing spec infrastructure

## Data model (spec-only)

- summary:
  - total spec count
  - status counts
  - lane counts
  - criteria completion summary
- specs[]:
  - id, title, status, type, lane, checked/total, passed
  - blocked_by[] and related[] edges
- edges[]:
  - `blocked_by`
  - resolvable `related`

No children/toys sections in Atlas payload.

## Mother routes

- `GET /api/atlas/specs` (authoritative API)
- `GET /atlas/specs.json` (web alias)
- `GET /atlas` and `/atlas/index.html` (Svelte app shell)

## CLI behavior

`patina atlas` becomes thin client:

- call Mother atlas endpoint
- emit JSON/HTML via API payload
- if Mother unavailable -> explicit fail-closed guidance (`patina mother start`)

No local snapshot rebuild path in CLI.

## UI behavior (Svelte/SvelteKit)

- UI fetches `/atlas/specs.json`
- rendering concerns are decoupled from data contract
- API contract test locks prevent accidental schema drift

## Migration plan

1. Introduce Mother-native atlas spec module.
2. Add new API route(s) and schema locks.
3. Update CLI atlas command to client-only.
4. Remove children/toys atlas payload fields and related rendering.
5. Update UI to spec-only model.
6. Add deterministic tests and HITL demo packet.
