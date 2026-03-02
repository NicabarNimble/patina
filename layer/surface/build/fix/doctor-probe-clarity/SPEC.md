---
type: fix
id: doctor-probe-clarity
status: draft
created: 2026-03-02
sessions:
  origin: 20260302-144326
related:
- data-measure-surface
- data-architecture-v2
beliefs:
- measure-reads-tables-not-events
- events-are-autobiography-not-telemetry
- parse-at-boundary-type-the-interior
- correctness-by-construction-not-convention
- plugin-is-agent-plus-skill
exit_criteria:
- id: doctor-emits-before-display
  text: doctor emits `measure.capture` event before any terminal output — emit is the primary action, display is secondary
  checked: false
- id: zero-serde-json-value-in-health-check-struct
  text: '`HealthCheck` and `ToolChange` structs use no `serde_json::Value` fields — all typed'
  checked: false
- id: zero-get-chains-in-doctor-plugin
  text: zero `.get().and_then().unwrap_or()` chains in `plugins/doctor/src/lib.rs` — environment JSON parsed into typed struct at boundary
  checked: false
- id: doctor-terminal-output-labels-itself-as-probe
  text: terminal output clearly frames results as "check complete, emitted to event stream" not as the canonical health view
  checked: false
- id: measure-full-shows-doctor-findings
  text: '`patina measure --full` displays doctor findings (from last `measure.capture` health-check event) — measure is the dashboard, not doctor'
  checked: false
---
# fix: Doctor as Probe — Clarify Emit-First Role and Clean Type Soup

> Doctor plugin muddies probe/dashboard separation — displays health AND emits
> events. Clarify as probe-first, clean type soup in WASM plugin.

## Problem

`patina doctor` is a WASM command plugin (the first extracted command — proves
the command world). It checks environment health (tools installed, config valid,
layer files exist) and emits a `measure.capture` event with mode `health-check`.

Two problems:

1. **Probe/dashboard confusion.** Doctor both emits findings AND displays a full
   health summary. Users see two commands that answer "is my project healthy?"
   — doctor and measure. The architectural intent is clear (doctor = probe,
   measure = dashboard) but the UX muddies it.

2. **Type soup in WASM plugin.** `plugins/doctor/src/lib.rs` uses
   `serde_json::Value` with `.get().and_then().unwrap_or()` chains to parse
   environment JSON and build output. Same anti-soup pattern cleaned from
   measure (v0.35.7) and MCP handlers (v0.35.2). The plugin should parse
   environment JSON into typed structs at the boundary.

## Root Cause

Doctor was written as a standalone health check before the measure dashboard
existed. When measure was built, doctor gained an emit call (line 174) but kept
its full display output. The two commands grew in parallel instead of separating
into probe + dashboard.

The type soup exists because WASM plugins receive host data as JSON strings —
the natural path is `serde_json::from_str` → Value → `.get()` chains. The fix
is the same as everywhere else: `#[derive(Deserialize)]` structs at the boundary.

## Fix

### 1. Clarify emit-first flow

Move the `measure::record_measurement()` call to happen before any terminal
output. Doctor's primary job is emitting — display is a convenience.

Update terminal output to frame results as probe output:
```
  Doctor check complete (emitted to event stream)

  Environment: healthy
    tools: 3/3 present
    config: valid
    layer: 69 patterns, 795 sessions, 180 beliefs

  View full health: patina measure --full
```

Not a full health dashboard — just a summary of what was checked and a pointer
to measure for the complete picture.

### 2. Type the environment boundary

Replace `serde_json::Value` parsing with typed structs:

```rust
#[derive(Deserialize)]
struct Environment {
    tools: HashMap<String, ToolInfo>,
}

#[derive(Deserialize)]
struct ToolInfo {
    available: bool,
    version: Option<String>,
    path: Option<String>,
}
```

Parse once at the boundary (`serde_json::from_str::<Environment>`), use typed
fields throughout. Eliminate all `.get().and_then().unwrap_or()` chains.

### 3. Type the HealthCheck output

Replace `HealthCheck::to_json()` (which builds `serde_json::json!({})` by hand)
with `#[derive(Serialize)]` on the struct. Direct serialization, no manual
JSON construction.

## Non-Goals

- **Continuous mode / watch / tick lifecycle.** Future feat spec — depends on
  plugin system evolution and Mother scheduling.
- **New health checks.** This spec fixes the existing checks, doesn't add new ones.
- **Changing what doctor checks.** Environment, config, layer — same scope.

## Exit Criteria

1. Doctor emits `measure.capture` event before any terminal output
2. Zero `serde_json::Value` in `HealthCheck` and `ToolChange` structs
3. Zero `.get().and_then().unwrap_or()` chains in doctor plugin
4. Terminal output frames results as probe output with pointer to `patina measure --full`
5. `patina measure --full` shows doctor findings from last health-check event
