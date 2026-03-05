# Design: Plugin Roles — Connector, Grammar, Extension, App Metadata

The full design narrative for plugin roles lives in the parent container:
**[[spec-plugin-infrastructure]] DESIGN.md**, section "Plugin Roles —
What Plugins DO" (lines 142-199).

Key decisions documented there:
- **Worlds vs Roles:** Worlds = capability boundary (what you CAN do).
  Roles = purpose (what you're FOR). Two axes, orthogonal.
- **Four roles:** connector, grammar, extension, app.
- **Role in manifest:** `role = "connector"` in plugin.toml. One role
  per plugin.
- **Valid combinations:** connector→mother-child, grammar→pipeline,
  extension→command/task, app→mother-child/task. Invalid combos aren't
  blocked — doctor can warn.
- **Roles don't grant capabilities.** The world does that. Roles tell
  the system what the plugin is for.

See also: [[spec-plugin-infrastructure]] DESIGN.md, "Connector
Architecture — Three I/O Patterns" and "Connector Destination
Independence" for how roles interact with routing.

## Key Files

- `src/plugin/internal/mod.rs` — `PluginManifest` gains `role` field
- `src/plugin/internal/mod.rs` — `check_capabilities()` adds
  role-world validation
- `src/commands/plugin.rs` — `patina plugin list` gains role column
