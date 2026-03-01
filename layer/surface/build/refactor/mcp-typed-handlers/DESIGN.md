# Design: MCP Typed Handlers — Eliminate Value Soup at Protocol Boundary

## Approach

Single-pass refactor across all 4 MCP server files. Each handler module
defines `#[derive(Deserialize)]` structs matching its tool schema. Dispatch
in `mod.rs` calls `serde_json::from_value()` once per tool, returning
`-32602` with the serde error message on failure. Handlers receive owned
typed structs — no `&serde_json::Value` anywhere.

**Struct design decisions:**
- `Option<T>` for optional fields, `.unwrap_or()` in handler for defaults
- `#[serde(default)]` for bool fields defaulting to false, `Vec` defaulting to empty
- `#[serde(default = "default_true")]` for `impact: bool` (defaults to true)
- Flat `SpecArgs` struct for all spec.*/schemas.* tools with `require!()` macro for per-subcommand validation
- `_prefix` + `#[serde(rename)]` for schema-present but handler-unwired fields (include_issues, context repo/all_repos)
- `ToolCallParams` struct at protocol level to type the `name` + `arguments` extraction

## Commits
1. `typed handlers: eliminate value soup at MCP protocol boundary` — define all 6 args structs, update dispatch, rewrite all handlers
2. `fmt: rustfmt fixups` — line wrapping on expanded_terms, assay fn signature

## Key Files
- `src/mcp/server/mod.rs` — dispatch with `ToolCallParams` + per-tool `from_value()`
- `src/mcp/server/scry.rs` — `ScryArgs`, `ContextArgs`, `MotherArgs` structs
- `src/mcp/server/spec.rs` — `SpecArgs` flat struct + `require!()` macro
- `src/mcp/server/assay.rs` — `AssayArgs` struct

## Open Questions
None — all 5 exit criteria verified.
