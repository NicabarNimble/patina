# Phase 1 Design: The Bucket (WIT Interface + Event Schema)

> Implementation design for measurement-coverage Phase 1.
> Read SPEC.md first — this document covers HOW, not WHY.

## Implementation Order

Dependencies flow top-down. Each step is one commit.

```
1. WIT interface definition          (wit/deps/patina-host/host.wit)
2. World imports                     (wit/{command,mother-child,task}/)
3. Host-side logic                   (src/plugin/internal/host_support.rs)
4. Bindgen trait impls               (src/plugin/internal/{command,mother_child,task}.rs)
5. Capability gating                 (src/plugin/internal/mod.rs)
6. SDK re-exports                    (plugins/sdk/src/{command,mother_child,task}.rs)
7. Core helper module                (src/measure.rs + src/lib.rs)
8. Doctor plugin migration           (plugins/doctor/src/lib.rs + plugin.toml)
9. MCP tool description update       (src/mcp/server/tools.rs)
```

Steps 1-2 are WIT changes (no Rust compilation). Steps 3-6 are the host/guest
plumbing. Step 7 is the core-side helper for compiled-in tools. Step 8 proves
the plugin path works end-to-end.

## Step 1: WIT Interface Definition

**File:** `wit/deps/patina-host/host.wit`

Add after the `schema` interface:

```wit
/// Measurement reporting for plugins.
///
/// Plugins call record-measurement to report metrics to the host.
/// The host writes measurement events to the eventlog with the
/// plugin's name as source (host overrides — plugins can't
/// impersonate core tools).
///
/// verb: one of the 5 protocol verbs (capture, index, search, believe, evolve)
/// tool: which tool produced this (doctor, grammar-rust, etc.)
/// mode: tool-specific sub-mode (freshness-check, parse-coverage, etc.)
/// metrics-json: JSON object with numeric values only
interface measure {
    record-measurement: func(
        verb: string,
        tool: string,
        mode: string,
        metrics-json: string,
    ) -> result<_, string>;
}
```

**Design decisions:**
- Single function, not per-verb functions. Verb is a string parameter because
  the verb set is stable (5 verbs) and validated host-side. An enum would
  require WIT changes to add a verb — string is more resilient.
- `metrics-json` is a string, not a typed record. Metric keys vary per tool.
  Host validates that values are numeric JSON. Same pattern as `host/query`
  which uses JSON strings at the WIT boundary.
- No `source` parameter — host overrides with the plugin name from HostState.
  This is a security property: plugins cannot impersonate core tools.

## Step 2: World Imports

Add `import patina:host/measure@0.1.0;` to three worlds.

**File:** `wit/command/command.wit`
```wit
world command {
    import patina:host/log@0.1.0;
    import patina:host/layer@0.1.0;
    import patina:host/query@0.1.0;
    import patina:host/measure@0.1.0;   // NEW
    // ... exports unchanged
}
```

**File:** `wit/mother-child/mother-child.wit`
```wit
world mother-child {
    import patina:host/log@0.1.0;
    import patina:host/types@0.1.0;
    import patina:host/layer@0.1.0;
    import patina:host/query@0.1.0;
    import patina:host/http@0.1.0;
    import patina:host/measure@0.1.0;   // NEW
    // ... exports unchanged
}
```

**File:** `wit/task/task.wit`
```wit
world task {
    import patina:host/log@0.1.0;
    import patina:host/types@0.1.0;
    import patina:host/layer@0.1.0;
    import patina:host/query@0.1.0;
    import patina:host/http@0.1.0;
    import patina:host/measure@0.1.0;   // NEW
    // ... exports unchanged
}
```

**NOT added to:** `wit/pipeline/pipeline.wit` — pipeline plugins are pure
compute (parsers, chunkers). They have `host_log` only. If a pipeline plugin
needs to report metrics, the host can measure it externally (timing, output
size) without the plugin knowing.

## Backward Compatibility: Existing Plugins

Adding `import patina:host/measure` to world definitions does NOT break
existing plugins compiled against the old WIT:

1. **Host-side (linker):** `add_to_linker()` registers the `measure` host
   functions on the linker. Old plugin components don't import `measure`,
   so wasmtime never calls those functions. The linker has extra capabilities
   that the guest doesn't use — this is safe and normal.

2. **Guest-side (old WASM binaries):** Old `.wasm` files were compiled against
   the old world definition (without `measure`). They instantiate fine because
   the component model is additive for imports — the host provides more than
   the guest needs.

3. **Guest-side (rebuilding old plugins):** When an old plugin's SDK dependency
   is updated, `wit_bindgen::generate!()` regenerates bindings that include
   the `measure` import. The plugin doesn't need to call it — the generated
   bindings produce a no-op stub. The plugin compiles and runs as before.

4. **New plugins:** Opt in by setting `host_measure = true` in plugin.toml
   and calling `measure::record_measurement()` from the SDK. Without the
   capability declared, the plugin simply doesn't call the function.

**No breaking change. No migration required. Additive only.**

## Step 3: Host-Side Logic

**File:** `src/plugin/internal/host_support.rs`

Add a new section after the HTTP host support:

```rust
// =========================================================================
// Measure host support
// =========================================================================

/// Valid protocol verbs for measurement events.
const VALID_VERBS: &[&str] = &["capture", "index", "search", "believe", "evolve"];

/// Record a measurement event from a plugin.
///
/// Validates verb, checks metrics are numeric JSON, writes to eventlog
/// with source overridden to the plugin name (security: plugins can't
/// impersonate core).
///
/// The host opens patina.db, writes the event, and closes. No connection
/// caching — measurement events are infrequent (one per tool run).
pub(super) fn record_measurement(
    project_root: &Option<PathBuf>,
    plugin_name: &str,
    verb: &str,
    tool: &str,
    mode: &str,
    metrics_json: &str,
) -> Result<(), String> {
    // Validate verb
    if !VALID_VERBS.contains(&verb) {
        return Err(format!(
            "invalid verb '{}': must be one of {:?}",
            verb, VALID_VERBS
        ));
    }

    // Validate metrics_json is a JSON object with numeric values
    let metrics: serde_json::Value = serde_json::from_str(metrics_json)
        .map_err(|e| format!("invalid metrics JSON: {}", e))?;

    let obj = metrics
        .as_object()
        .ok_or_else(|| "metrics must be a JSON object".to_string())?;

    for (key, value) in obj {
        if !value.is_number() {
            return Err(format!(
                "metric '{}' must be numeric, got {}",
                key,
                value
            ));
        }
    }

    // Open patina.db
    let root = project_root
        .as_ref()
        .ok_or_else(|| "no project root".to_string())?;
    let db_path = root.join(crate::eventlog::PATINA_DB);
    let conn = crate::eventlog::initialize(&db_path)
        .map_err(|e| format!("open patina.db: {}", e))?;

    // Build event data — source is always the plugin name
    let event_data = serde_json::json!({
        "verb": verb,
        "tool": tool,
        "mode": mode,
        "metrics": metrics,
        "source": plugin_name,
    });

    let event_type = format!("measure.{}", verb);
    // Include plugin_name in source_id to prevent collision between plugins
    // that use the same tool:mode strings. Core tools use "tool:mode" only
    // (no namespace needed — tool names are unique within the binary).
    let source_id = format!("plugin:{}:{}:{}", plugin_name, tool, mode);
    let timestamp = chrono::Utc::now().to_rfc3339();

    crate::eventlog::insert_event(
        &conn,
        &event_type,
        &timestamp,
        &source_id,
        None,
        &event_data.to_string(),
    )
    .map_err(|e| format!("insert measurement event: {}", e))?;

    Ok(())
}
```

**Key design notes:**
- Host opens the DB per call. Measurement events are infrequent — no need to
  cache the connection on HostState (unlike HTTP client which is cached).
- Source is always `plugin_name`, not caller-controlled. Security property.
- `source_id` uses `plugin:<name>:<tool>:<mode>` format for plugins, vs
  `<tool>:<mode>` for core tools. This prevents two plugins from colliding
  on the same tool:mode key. The `data.source` field also carries the plugin
  name, but source_id is indexed — it's the aggregation key.
- Verb validation is strict (must be one of 5). Tool/mode are freeform strings.
- Metrics must be a flat JSON object with numeric values only. No nested objects.

## Step 4: Bindgen Trait Implementations

Each world that imports `patina:host/measure` needs a Host trait impl.
All three delegate to `host_support::record_measurement()`.

**File:** `src/plugin/internal/command.rs`

Add inside `mod command_bindings` after the `patina::host::query::Host` impl:

```rust
// patina:host/measure — delegates to host_support
impl patina::host::measure::Host for CommandHostState {
    fn record_measurement(
        &mut self,
        verb: String,
        tool: String,
        mode: String,
        metrics_json: String,
    ) -> Result<(), String> {
        super::super::host_support::record_measurement(
            &self.project_root,
            &self.plugin_name,
            &verb,
            &tool,
            &mode,
            &metrics_json,
        )
    }
}
```

**File:** `src/plugin/internal/mother_child.rs`

Add inside `mod bindings` after the `patina::host::http::Host` impl:

```rust
// patina:host/measure — delegates to host_support
impl patina::host::measure::Host for HostState {
    fn record_measurement(
        &mut self,
        verb: String,
        tool: String,
        mode: String,
        metrics_json: String,
    ) -> Result<(), String> {
        super::super::host_support::record_measurement(
            &self.project_root,
            &self.plugin_name,
            &verb,
            &tool,
            &mode,
            &metrics_json,
        )
    }
}
```

**File:** `src/plugin/internal/task.rs`

Add inside `mod task_bindings` after the `patina::host::http::Host` impl:

```rust
// patina:host/measure — delegates to host_support
impl patina::host::measure::Host for TaskHostState {
    fn record_measurement(
        &mut self,
        verb: String,
        tool: String,
        mode: String,
        metrics_json: String,
    ) -> Result<(), String> {
        super::super::host_support::record_measurement(
            &self.project_root,
            &self.plugin_name,
            &verb,
            &tool,
            &mode,
            &metrics_json,
        )
    }
}
```

All three are identical because they delegate to the same `host_support` function.
This follows the established pattern — see how `host/query` is implemented
identically across all three worlds.

## Step 5: Capability Gating

**File:** `src/plugin/internal/mod.rs`

### 5a. Add `host_measure` to allowed capabilities

```rust
impl PluginWorld {
    pub fn allowed_capabilities(&self) -> &[&str] {
        match self {
            Self::MotherChild => &["host_log", "host_layer", "host_query", "host_http", "host_measure"],
            Self::Command => &["host_log", "host_layer", "host_query", "host_measure"],
            Self::Task => &["host_log", "host_layer", "host_query", "host_http", "host_measure"],
            Self::Pipeline => &["host_log"],
        }
    }
}
```

### 5b. Manifest parsing

`host_measure` is a boolean capability — same as `host_log` and `host_layer`.
No additional parsing needed beyond the existing boolean capability scan.
The existing code already handles this:

```rust
let capabilities = cap_table
    .map(|cap| {
        cap.iter()
            .filter(|(_, v)| v.as_bool() == Some(true))
            .map(|(k, _)| k.clone())
            .collect()
    })
    .unwrap_or_default();
```

A plugin.toml with `host_measure = true` will be picked up by the boolean
scan. No capability-specific gating is needed at call time — if the interface
is imported by the world definition, the host impl is always available.
There's no call-time check needed because:
- If a plugin's world doesn't import `measure`, it can't call it (WASM safety)
- If the world DOES import it, the host impl is always available (no per-domain
  or per-kind gating needed, unlike query/http)

**Decision: No GrantedCapabilities changes needed.** Unlike `host_query` (which
gates per-kind) and `host_http` (which gates per-domain), `host_measure` has
no sub-capabilities. The verb validation happens inside `record_measurement()`,
not through the capabilities system.

## Step 6: SDK Re-exports

Each world's SDK module gets a `measure` re-export.

**File:** `plugins/sdk/src/command.rs`

Add after the `query` module:

```rust
/// Measurement reporting — record metrics from plugin execution.
///
/// Requires `host_measure = true` in plugin.toml capabilities.
/// The host validates verb, checks metrics are numeric JSON, and
/// writes to eventlog with the plugin name as source.
pub mod measure {
    /// Record a measurement event.
    ///
    /// - `verb`: protocol verb (capture, index, search, believe, evolve)
    /// - `tool`: tool name (e.g., "doctor")
    /// - `mode`: sub-mode (e.g., "freshness-check")
    /// - `metrics_json`: JSON object with numeric values (e.g., `{"score": 0.95}`)
    pub fn record_measurement(
        verb: &str,
        tool: &str,
        mode: &str,
        metrics_json: &str,
    ) -> Result<(), String> {
        super::patina::host::measure::record_measurement(verb, tool, mode, metrics_json)
    }
}
```

**File:** `plugins/sdk/src/mother_child.rs` — same `pub mod measure` block.

**File:** `plugins/sdk/src/task.rs` — same `pub mod measure` block.

## Step 7: Core Helper Module

**File:** `src/measure.rs` (new file)

This is the compiled-in equivalent of the WIT interface. Core tools (eval,
bench, scrape, oxidize) call this instead of the WIT function.

```rust
//! Measurement emission for compiled-in core tools.
//!
//! This module is the core-side equivalent of the WIT `patina:host/measure`
//! interface. Core tools call `emit()` to write measurement events to the
//! eventlog. WASM plugins use the WIT interface instead.
//!
//! Both paths produce identical event schemas in the eventlog.

use anyhow::Result;
use rusqlite::Connection;

/// Valid protocol verbs for measurement events.
pub const VALID_VERBS: &[&str] = &["capture", "index", "search", "believe", "evolve"];

/// Emit a measurement event to the eventlog.
///
/// Core tools call this after computing metrics. The event lands in the
/// eventlog with `event_type = "measure.<verb>"` and `source = "core"`.
///
/// # Arguments
/// - `conn` — open connection to patina.db (caller manages lifecycle)
/// - `verb` — protocol verb: capture, index, search, believe, evolve
/// - `tool` — tool name: eval, bench, scrape, oxidize, etc.
/// - `mode` — tool-specific sub-mode: nl, feedback, ablation, etc.
/// - `metrics` — JSON object with numeric values
///
/// # Errors
/// Returns error if verb is invalid or eventlog write fails.
pub fn emit(
    conn: &Connection,
    verb: &str,
    tool: &str,
    mode: &str,
    metrics: &serde_json::Value,
) -> Result<()> {
    anyhow::ensure!(
        VALID_VERBS.contains(&verb),
        "invalid verb '{}': must be one of {:?}",
        verb,
        VALID_VERBS
    );

    let event_type = format!("measure.{}", verb);
    let source_id = format!("{}:{}", tool, mode);
    let timestamp = chrono::Utc::now().to_rfc3339();

    let data = serde_json::json!({
        "verb": verb,
        "tool": tool,
        "mode": mode,
        "metrics": metrics,
        "source": "core",
    });

    crate::eventlog::insert_event(
        conn,
        &event_type,
        &timestamp,
        &source_id,
        None,
        &data.to_string(),
    )?;

    Ok(())
}
```

**File:** `src/lib.rs` — add `pub mod measure;` to the module list.

**Design decisions:**
- Takes `&Connection` (not `&Path`) — core tools already have an open DB
  connection. The host_support function opens its own because plugins don't
  have DB access.
- `source` is always `"core"` — hardcoded, not parameterized. This pairs with
  the plugin path where source is always the plugin name.
- Uses `anyhow::Result` (not `Result<_, String>`) because this is library code,
  not WIT boundary code. The WIT path uses `Result<_, String>` because that's
  what WIT generates.
- `VALID_VERBS` is pub so tests and future code can reference it.

## Step 8: Doctor Plugin Migration

**File:** `plugins/doctor/plugin.toml`

Add measurement capability:

```toml
[capabilities]
host_log = true
host_layer = true
host_query = ["context"]
host_measure = true          # NEW
```

**File:** `plugins/doctor/src/lib.rs`

Add measurement emission at the end of `run()`, after computing health status:

```rust
use patina_sdk::command::{layer, query, measure};  // add measure

// At the end of run(), before returning exit code:
fn run(&mut self, args: &[String]) -> i32 {
    // ... existing health check logic ...

    // Emit measurement events
    let capture_metrics = serde_json::json!({
        "missing_tools": health.missing_tools.len(),
        "new_tools": health.new_tools.len(),
        "layer_patterns": health.layer_patterns,
        "sessions": health.sessions,
        "beliefs": health.beliefs.unwrap_or(0),
    });

    if let Err(e) = measure::record_measurement(
        "capture",
        "doctor",
        "health-check",
        &capture_metrics.to_string(),
    ) {
        eprintln!("Warning: failed to record measurement: {}", e);
    }

    // Exit code unchanged
    match health.status.as_str() { ... }
}
```

**What doctor measures:**
- `missing_tools` — count of tools that disappeared since init (capture health)
- `new_tools` — count of new tools detected (capture freshness)
- `layer_patterns` — pattern count (believe proxy)
- `sessions` — session count (evolve proxy)
- `beliefs` — belief count (believe proxy)

Doctor uses the `capture` verb because it's checking project foundation health —
whether the capture infrastructure (tools, layer, config) is intact.

## Step 9: MCP Tool Description

**File:** `src/mcp/server/tools.rs`

No new MCP tool in Phase 1. The `measure` MCP tool comes in Phase 3 (consumer
views). Phase 1 only builds the write side.

However, update the `spec_set` tool description to note `host_measure` is a
known capability, so MCP consumers can recommend it to plugin authors.

## Verification

After implementation, verify with:

```bash
# Build and install
cargo build --release && cargo install --path .

# Run doctor — should emit measurement event
patina doctor

# Check eventlog for measurement events
sqlite3 .patina/local/data/patina.db \
  "SELECT event_type, source_id, data FROM eventlog WHERE event_type LIKE 'measure.%' ORDER BY seq DESC LIMIT 5;"

# Verify schema correctness
sqlite3 .patina/local/data/patina.db \
  "SELECT COUNT(*) FROM eventlog WHERE event_type LIKE 'measure.%' AND (json_extract(data, '$.verb') IS NULL OR json_extract(data, '$.tool') IS NULL OR json_extract(data, '$.metrics') IS NULL);"
# Expected: 0

# Pre-push checks
./resources/git/pre-push-checks.sh
```

## Phase 2 Guidance: Connection Management

The Phase 1 design has two connection strategies:

- **WIT path (plugins):** Host opens patina.db per `record_measurement()` call.
  Fine for Phase 1 — doctor emits 1 event per run.
- **Core path (compiled-in tools):** `measure::emit()` takes `&Connection`.
  Caller manages lifecycle. Fine for any volume.

For Phase 2, core tools (eval, bench, scrape, oxidize) already open patina.db
for their own purposes. They pass the existing connection to `measure::emit()`.
No new DB opens, no I/O regression.

If a future plugin emits many events per invocation (unlikely for measurement —
it's one event per tool run, not per query), the host_support function could
be refactored to accept an optional cached connection on HostState. But this
is a Phase 2+ concern and the current design supports it without breaking
changes — just add an `Option<Connection>` field to HostState and check it
before opening a new one.

## Project-Root Requirement

Both paths require a project root to locate patina.db. In contexts where no
project root exists (theoretical `--global` flag, remote analysis):

- **WIT path:** Returns `Err("no project root")` — plugin gets an error, can
  log and continue. Doctor already handles this (line 86: returns exit code 1).
- **Core path:** Caller doesn't call `measure::emit()` if there's no DB.

This is intentional. Patina is project-scoped (see `layer/core/safety-boundaries.md`).
Measurement data belongs to a project. A future global metrics store (e.g., in
Mother's graph.db) would be a separate interface, not a change to this one.

## Files Changed (Summary)

| File | Change |
|---|---|
| `wit/deps/patina-host/host.wit` | Add `interface measure` |
| `wit/command/command.wit` | Add `import patina:host/measure@0.1.0` |
| `wit/mother-child/mother-child.wit` | Add `import patina:host/measure@0.1.0` |
| `wit/task/task.wit` | Add `import patina:host/measure@0.1.0` |
| `src/plugin/internal/host_support.rs` | Add `record_measurement()` |
| `src/plugin/internal/command.rs` | Add `measure::Host` trait impl |
| `src/plugin/internal/mother_child.rs` | Add `measure::Host` trait impl |
| `src/plugin/internal/task.rs` | Add `measure::Host` trait impl |
| `src/plugin/internal/mod.rs` | Add `host_measure` to allowed capabilities |
| `plugins/sdk/src/command.rs` | Add `pub mod measure` re-export |
| `plugins/sdk/src/mother_child.rs` | Add `pub mod measure` re-export |
| `plugins/sdk/src/task.rs` | Add `pub mod measure` re-export |
| `src/measure.rs` | New: core helper module |
| `src/lib.rs` | Add `pub mod measure` |
| `plugins/doctor/plugin.toml` | Add `host_measure = true` |
| `plugins/doctor/src/lib.rs` | Add measurement emission |

16 files total. ~150 lines of new code (excluding comments/docs).
