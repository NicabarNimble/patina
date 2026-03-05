# Design: Host Emit WIT Interface — Plugins Can Write Facts

The full design narrative for host_emit lives in the parent container:
**[[spec-plugin-infrastructure]] DESIGN.md**, section "host_emit — The
Missing Write Path" (lines 55-140).

Key decisions documented there:
- **Pattern A (host import)** — not intent return. Helland analysis:
  facts are independently valid, host validates and writes, plugin gets
  confirmation.
- **Interface:** `emit-fact(schema, fact-type, data) -> result<u64, string>`
- **Validation:** schema exists, plugin declares it in manifest, fact-type
  exists in schema, data is valid JSON.
- **Worlds:** mother-child and task get emit. Pipeline and command do not.

See also: [[spec-plugin-infrastructure]] DESIGN.md, "Connector
Architecture — Three I/O Patterns" for how connectors use emit across
request/response, polling, and streaming patterns.

## Key Files

- `wit/deps/patina-host/host.wit` — `emit` interface added here
- `src/plugin/internal/host_support.rs` — host_emit implementation
- `src/plugin/internal/mother_child.rs` — bindgen wiring
- `src/plugin/internal/mod.rs` — capability gating for `host_emit`
