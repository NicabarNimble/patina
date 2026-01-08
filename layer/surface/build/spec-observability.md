# Spec: Observability

**Status**: Draft
**Created**: 2026-01-01
**Purpose**: Unified logging and event system for the three-layer architecture

---

## Vision

> "You can't fix what you can't see."

Observability is infrastructure. It belongs in **mother** - the layer that provides central services to all other layers.

### The Goal

Every significant operation in patina emits an event. Events flow to mothership. Developers can debug issues, analyze patterns, and understand system behavior without reading source code.

```
┌─────────────────────────────────────────────────────────────┐
│                      The Full Picture                       │
│                                                             │
│   mother              patina              awaken            │
│   (infra)        →    (know)         →    (ship)           │
│      │                   │                   │              │
│      │                   │                   │              │
│      └───────────────────┴───────────────────┘              │
│                          │                                  │
│                          ▼                                  │
│                  mothership (serve)                         │
│                          │                                  │
│              ┌───────────┼───────────┐                     │
│              ▼           ▼           ▼                     │
│          eventlog     stderr      query                    │
│        (persistent) (PATINA_LOG)  (patina logs)            │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

This is where we're going. Phase 0 validates the approach.

---

## Problem Statement

**Today's reality:**

| Scenario | What we need | What we have |
|----------|--------------|--------------|
| Keychain popup mystery | Why did it trigger? SSH vs GUI? | Nothing - had to read code |
| Retrieval is slow | Which oracle? Model loading? | Nothing - spec proposes eprintln |
| Secrets not injected | Identity found? Vault decrypted? | "No secrets" message, no detail |
| Mothership won't start | Port conflict? Crash? | "Failed to start" - no detail |

**Root cause:** No unified logging. Scattered `eprintln!` calls. Silent failures.

**Assumption to validate:** Adding structured logging will help debug these issues faster than reading code.

---

## Design

### Core Principle: Mothership as Hub

Mothership is always running when patina is running. This simplifies everything:

- No "what if mothership is down" complexity
- Single destination for all events
- Single source of truth for queries

```
┌─────────────────────────────────────────────────┐
│ Mothership (mother serve)                       │
│                                                 │
│  Receives:                                      │
│  ├── Events from patina commands               │
│  ├── Events from mother commands               │
│  └── Events from awaken commands               │
│                                                 │
│  Outputs:                                       │
│  ├── eventlog table (persistent)  [Phase 3]   │
│  ├── stderr (PATINA_LOG)          [Phase 0]   │
│  └── query endpoint               [Phase 4]   │
│                                                 │
└─────────────────────────────────────────────────┘
```

### Phase 0 Design: Minimal stderr

```bash
PATINA_LOG=debug patina secrets

# Output:
[DEBUG secrets::keychain] attempting get_generic_password
[DEBUG secrets::keychain] result: success
[INFO  secrets::identity] identity_source = Keychain
```

That's it. No struct, no persistence, no query API. Just print to stderr.

### Full Vision: Event Schema (Phase 3+)

When we need persistence, we'll define events. Starting point:

```rust
struct Event {
    timestamp: DateTime<Utc>,
    domain: String,      // secrets, retrieval, serve
    event_type: String,  // keychain.access, query.complete
    level: String,       // debug, info, warn, error
    data: Value,         // Event-specific payload
}
```

Additional fields to add **only when proven necessary**:
- `event_id` - If we need deduplication
- `layer` - If we need to filter by mother/patina/awaken
- `session_id` - If we need to correlate with dev sessions
- `project` - If we need multi-project analysis
- `duration_ms` - If we need timing analysis (or just put in `data`)

### Full Vision: Event Domains

| Layer | Domain | Example Events |
|-------|--------|----------------|
| mother | `secrets` | keychain.access, identity.resolve, vault.decrypt |
| mother | `serve` | mothership.start, mothership.health, mcp.request |
| mother | `persona` | knowledge.capture, knowledge.query |
| patina | `retrieval` | query.start, query.complete, oracle.result, cache.hit |
| patina | `scrape` | code.extract, git.extract, session.extract |
| patina | `eval` | benchmark.run, precision.compute |
| awaken | `build` | container.generate, build.start, build.complete |
| awaken | `deploy` | deploy.start, deploy.complete |

This catalog is the **vision**. Phase 0 implements only: `secrets.keychain.access`, `secrets.keychain.result`.

---

## Event Catalog (Vision)

This catalog documents the full vision. Implement events as needed, not upfront.

### secrets (mother) - Phase 0 starts here

| Event Type | Level | When | Data |
|------------|-------|------|------|
| `keychain.access` | debug | Keychain read/write attempted | `{operation, service, account}` |
| `keychain.result` | debug | Keychain operation completed | `{operation, success, error?}` |
| `identity.resolve` | info | Identity source determined | `{source: env|keychain|none}` |
| `vault.decrypt` | info | Vault decryption attempted | `{vault: global|project, success}` |
| `secrets.inject` | info | Secrets injected into command | `{count, target_command}` |

### retrieval (patina) - Phase 2

| Event Type | Level | When | Data |
|------------|-------|------|------|
| `query.start` | debug | Query begins | `{query, options}` |
| `query.complete` | info | Query finished | `{query, result_count, duration_ms}` |
| `oracle.result` | debug | Single oracle returns | `{oracle, result_count, duration_ms}` |
| `cache.hit` | debug | Query embedding cache hit | `{query_hash}` |
| `cache.miss` | debug | Query embedding cache miss | `{query_hash}` |
| `fusion.complete` | debug | RRF fusion done | `{input_count, output_count}` |

### serve (mother) - Future

| Event Type | Level | When | Data |
|------------|-------|------|------|
| `mothership.start` | info | Daemon starting | `{host, port}` |
| `mothership.ready` | info | Daemon ready | `{oracles_loaded}` |
| `mothership.stop` | info | Daemon stopping | `{reason}` |
| `mcp.request` | debug | MCP tool call received | `{method, tool}` |
| `mcp.response` | debug | MCP response sent | `{method, duration_ms}` |
| `health.check` | debug | Health endpoint called | `{status}` |

---

## Implementation

### Phase 0: Validate the Approach (MANDATORY FIRST)

> "Does structured logging actually help debug the keychain mystery?"

Before building infrastructure, prove the concept with the simplest possible implementation.

**Task 0.1:** Add PATINA_LOG to secrets/keychain.rs

Minimal change - just eprintln with a flag:

```rust
// src/secrets/keychain.rs
fn log_debug(msg: &str) {
    if std::env::var("PATINA_LOG").is_ok() {
        eprintln!("[DEBUG secrets::keychain] {}", msg);
    }
}

pub fn get_identity() -> Result<String> {
    log_debug("attempting get_generic_password");

    let result = get_generic_password(KEYCHAIN_SERVICE, KEYCHAIN_ACCOUNT);

    match &result {
        Ok(_) => log_debug("success"),
        Err(e) => log_debug(&format!("error: {}", e)),
    }

    result
}

pub fn has_identity() -> bool {
    log_debug("checking has_identity");
    let result = get_generic_password(KEYCHAIN_SERVICE, KEYCHAIN_ACCOUNT).is_ok();
    log_debug(&format!("has_identity = {}", result));
    result
}
```

**Task 0.2:** Test with real scenario

```bash
# Local Mac (with GUI)
PATINA_LOG=1 patina secrets

# SSH (no GUI)
ssh mac-studio "cd project && PATINA_LOG=1 patina secrets"
```

**Task 0.3:** Record learnings

Document in this spec:
- Did the logs reveal the keychain mystery?
- What additional info would have helped?
- Was the log format useful?

**Exit criteria:**
- Keychain mystery understood (or know what's still missing)
- Decision: proceed to Phase 1, or iterate on Phase 0

---

### Phase 1: Minimal Infrastructure

Only proceed here after Phase 0 proves value.

**Task 1.1:** Create lightweight log module

```rust
// src/log.rs (not src/observability/ - keep it simple)
use std::env;

pub fn debug(domain: &str, msg: &str) {
    if should_log("debug") {
        eprintln!("[DEBUG {}] {}", domain, msg);
    }
}

pub fn info(domain: &str, msg: &str) {
    if should_log("info") {
        eprintln!("[INFO {}] {}", domain, msg);
    }
}

fn should_log(level: &str) -> bool {
    match env::var("PATINA_LOG").as_deref() {
        Ok("debug") | Ok("trace") => true,
        Ok("info") => level != "debug",
        _ => false,
    }
}
```

**Task 1.2:** Replace Phase 0 inline logging

Convert inline `log_debug` calls to use the module.

**Task 1.3:** Instrument secrets module fully

Add logging to identity resolution, vault operations.

**Exit criteria:**
- `PATINA_LOG=debug patina secrets` shows full secrets flow
- Can diagnose keychain, identity, vault issues from logs alone

---

### Phase 2: Instrument Retrieval

Addresses spec-retrieval-optimization Phase 0 needs.

**Task 2.1:** Add timing to QueryEngine

```rust
// src/retrieval/engine.rs
pub fn query(&self, query: &str, limit: usize) -> Result<Vec<FusedResult>> {
    let start = Instant::now();
    log::debug("retrieval", &format!("query start: {}", query));

    // ... existing logic ...

    log::info("retrieval", &format!(
        "query complete: {} results in {:?}",
        results.len(),
        start.elapsed()
    ));
    Ok(results)
}
```

**Task 2.2:** Add per-oracle timing

Log each oracle's contribution and duration.

**Exit criteria:**
- Can identify slow oracles from logs
- Retrieval spec Phase 0 requirements met

---

### Phase 3: Persistence (eventlog)

Only proceed here if stderr logging proves insufficient for analysis.

**Trigger:** Need to analyze patterns across sessions, not just debug single issues.

**Task 3.1:** Define minimal Event struct

Start small, expand based on actual needs:

```rust
struct Event {
    timestamp: DateTime<Utc>,
    domain: String,      // secrets, retrieval, serve
    event_type: String,  // keychain.access, query.complete
    level: String,       // debug, info, warn, error
    data: Value,         // event-specific payload
}
```

**Task 3.2:** Write to existing eventlog table

Extend existing table (already has event_type, timestamp, data).

**Task 3.3:** Dual output

Events go to both stderr (if PATINA_LOG) and eventlog (always for info+).

---

### Phase 4: Query Interface

Only proceed here if eventlog proves useful and needs querying.

**Trigger:** Running raw SQL against eventlog is too painful.

**Task 4.1:** Add `patina logs` command

```bash
patina logs                          # Recent events
patina logs --domain secrets         # Filter by domain
patina logs --since 1h               # Time filter
patina logs --follow                 # Tail mode
```

**Task 4.2:** Add mothership query endpoint

```
GET /logs?domain=secrets&since=2026-01-01
```

For programmatic access by eval, doctor, etc.

---

## Design Principles

Anchored in layer/core values:

### unix-philosophy: One Tool, One Job

| Module | "Do X" |
|--------|--------|
| `log.rs` | Print debug info to stderr |
| `eventlog` (existing) | Persist events to SQLite |
| `patina logs` (future) | Query persisted events |

Each is a focused tool. They compose: log.rs emits → eventlog persists → patina logs queries.

### dependable-rust: Tiny Stable Interface

```rust
// External interface (stable)
pub fn debug(domain: &str, msg: &str);
pub fn info(domain: &str, msg: &str);
pub fn warn(domain: &str, msg: &str);
pub fn error(domain: &str, msg: &str);

// Everything else is internal
// - Format strings
// - Level filtering logic
// - Output destination
```

Start with 4 functions. Expand only when proven necessary.

### adapter-pattern: Trait for Outputs (Future)

If we need multiple outputs (stderr, file, network), define a trait:

```rust
pub trait EventSink {
    fn emit(&self, level: Level, domain: &str, msg: &str);
}
```

**Not needed for Phase 0-1.** Add when/if we need swappable outputs.

---

## Security

**Events must NEVER contain secret values.**

| OK to log | NOT OK to log |
|-----------|---------------|
| `keychain.access attempted` | `identity = AGE-SECRET-KEY-1...` |
| `vault.decrypt success` | `secrets = {api_key: "sk-..."}` |
| `secrets.inject count=3` | `injected GITHUB_TOKEN=ghp_...` |

The observability system is infrastructure. It may be visible to LLMs via stderr or log queries. Secret values must stay in the secrets system, never flow to logs.

---

## What NOT To Do

| Anti-Pattern | Why Avoid |
|--------------|-----------|
| Build full infrastructure before validating | Phase 0 exists to prove value first |
| Use tracing crate immediately | Simple eprintln is enough for Phase 0-1 |
| Log secret values | LLM can see stderr |
| Create HTTP endpoint for logging | Direct write is simpler (same process) |
| Define 9-field Event schema upfront | Start with 4, expand based on need |
| Skip Phase 0 | "Flying blind" about whether logging helps |

---

## Future Work (Ideas Parking Lot)

These ideas are preserved, not scheduled. Revisit when triggers occur.

| Idea | Trigger to Revisit |
|------|-------------------|
| Structured JSON output | Need machine-parseable logs |
| Log rotation | Eventlog grows too large |
| Remote log shipping | Multi-machine deployments |
| Log-based alerting | Production monitoring needs |
| Distributed tracing | Cross-service debugging |
| EventSink trait | Need swappable outputs |
| Convergence signals | Need to verify agent task completion mechanically |

### Convergence Signals (Future)

Observability isn't just for debugging—it enables **mechanical verification**.

When agents claim "done," we need data to verify. Events could capture:
- Iteration counts (how many attempts before success?)
- Failure modes (what went wrong, how often?)
- Invariant checks (did tests pass? did MRR hold?)

This transforms observability from "see what happened" to "verify what succeeded." See `spec-three-layers.md` for the authority model this supports.

---

## Migration

### Existing eventlog

The `eventlog` table already exists with schema:

```sql
CREATE TABLE eventlog (
    event_type TEXT,
    timestamp TEXT,
    source_id TEXT,
    data TEXT
);
```

**Phase 3 migration path:**
1. Add `domain` and `level` columns with defaults
2. Existing events: `domain='retrieval'` (they're all scry queries)
3. New events use full schema
4. No breaking changes to existing queries

### Existing eprintln

Audit existing `eprintln!` calls:
- Keep: User-facing output (success messages, prompts)
- Convert: Debug/diagnostic output → use log module

---

## Validation

### Phase 0 Success (2026-01-01)

| Question | Answer |
|----------|--------|
| Did logs reveal keychain mystery? | **Yes** - logs show `has_identity` (no Touch ID) vs `get_generic_password` (Touch ID) distinction clearly |
| What was missing? | Nothing critical for keychain debugging. Consider adding vault.rs logging in Phase 1. |
| Proceed to Phase 1? | **Yes** - inline logging validated the approach |

**Phase 0 Observations:**

1. **has_identity is called twice** before the actual get - this is the status check path (no Touch ID triggered)
2. **Empty PATINA_IDENTITY** edge case caught - logs show "set but empty, falling back"
3. **Log format works well** - `[DEBUG secrets::keychain]` domain prefix is clear
4. **Pattern matches retrieval** - same `if std::env::var("PATINA_LOG").is_ok()` pattern as engine.rs

### Overall Success Criteria

| Criteria | How to Measure |
|----------|----------------|
| Can debug keychain issues | `PATINA_LOG=debug patina secrets` shows flow |
| Can debug slow retrieval | `PATINA_LOG=debug patina scry` shows oracle timing |
| No secret leakage | `grep -r "AGE-SECRET-KEY" logs` returns nothing |
| Minimal overhead | Measure with/without logging, <5% difference |

---

## Phase Summary

| Phase | Trigger | Deliverable | Exit Criteria |
|-------|---------|-------------|---------------|
| 0 | Current session's keychain mystery | Inline logging in keychain.rs | Mystery solved or know what's missing |
| 1 | Phase 0 proves value | `src/log.rs` module | Full secrets debugging via logs |
| 2 | Need retrieval debugging | Instrumented QueryEngine | Retrieval spec Phase 0 met |
| 3 | Need cross-session analysis | Events in eventlog | Can query historical events |
| 4 | SQL queries too painful | `patina logs` command | CLI access to event history |

---

## References

- `spec-three-layers.md` - Architecture context (mother owns infra)
- `spec-retrieval-optimization.md` - Phase 0 instrumentation needs
- `layer/core/unix-philosophy.md` - One tool, one job
- `layer/core/dependable-rust.md` - Tiny stable interface
- `layer/core/adapter-pattern.md` - Trait pattern (for future EventSink)
