# Design: watch null sink child

## Shape

- Child: `children/watch-null-sink`
- World: `watch-null-sink`
- Imports: `wasi:logging`, `patina:measure`
- Exports: `patina:watch/events`

## Runtime behavior

`emit(change)`:
1. increments `watch_events_dropped`
2. logs dropped event (kind + path)
3. returns `ok`

No keyvalue/sql/filesystem usage.

## Why this is strict

- Business contract is typed WIT export, not string action.
- Mother remains runtime/orchestrator only.
- Child is single-purpose and ephemeral.
