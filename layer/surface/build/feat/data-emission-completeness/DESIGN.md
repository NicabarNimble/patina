# Design: Emission Completeness — No Silent Operations

## Approach

Followed the code scraper pattern exactly: one `emit_or_warn()` call at the
end of each scraper's `run()` function. For context/assay, used direct
`eventlog::insert_event()` calls (not measure verbs) matching the scry pattern.
Session events retargeted from patina.db to events.db via `open_events_db()`.

## Commits
1. `f49c6335` — Register context.query and assay.query in Layer 0 event registry
2. `81082011` — Wire measure.capture into git, layer, beliefs, forge scrapers
3. `471c39d0` — Fix scry session-id early return (session_id: null is valid)
4. `9e92b1fd` — Emit context.query and assay.query from CLI and MCP paths
5. `4ae2e726` — Add emission coverage check to patina doctor
6. `96e59ce7` — Retarget session lifecycle events to events.db

## Key Files
- `src/commands/scrape/git/mod.rs` — gap 1: measure.capture for git scrape
- `src/commands/scrape/layer/mod.rs` — gap 2: measure.capture for layer scrape
- `src/commands/scrape/beliefs/mod.rs` — gap 3: measure.capture for beliefs scrape
- `src/commands/scrape/forge/mod.rs` — gap 4: measure.capture for forge scrape
- `src/commands/context.rs` — gap 5: context.query emission (CLI)
- `src/commands/assay/mod.rs` — gap 6: assay.query emission (CLI)
- `src/commands/scry/internal/logging.rs` — gap 7: session_id early return fix
- `src/mcp/server/scry.rs` — context.query emission (MCP) + shared emit helper
- `src/mcp/server/assay.rs` — assay.query emission (MCP)
- `src/commands/doctor.rs` — emission coverage check (12 Active types)
- `src/commands/session/internal.rs` — session events retargeted to events.db

## Open Questions
- WASM doctor plugin needs rebuild to include emission coverage check
  (bundled doctor has it, but WASM plugin takes priority when installed)
