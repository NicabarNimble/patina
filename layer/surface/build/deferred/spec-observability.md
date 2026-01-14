---
id: spec-observability
status: deferred
created: 2026-01-13
extracted-from: spec-init-hardening (Phase 4)
category: backlog
tags: [spec, observability, metrics, events]
---

# Spec: Command Observability

**Problem:** No visibility into how users interact with patina commands. Can't measure friction or identify common failure modes.

**Solution:** Event logging to local SQLite database with `patina stats` command.

**Status:** Deferred - core functionality works without it. Implement when we need data-driven UX improvements.

---

## Design (from spec-init-hardening)

### Events Database

Schema in `.patina/local/events.db`:

```sql
CREATE TABLE events (
    id INTEGER PRIMARY KEY,
    timestamp TEXT NOT NULL,
    event_type TEXT NOT NULL,
    adapter TEXT,
    data TEXT  -- JSON
);
```

### Event Types

| Event | When | Data |
|-------|------|------|
| `init_completed` | After successful init | `{"reinit": bool}` |
| `adapter_added` | After adapter add | `{"adapter": string}` |
| `adapter_refreshed` | After adapter refresh | `{"adapter": string}` |
| `adapter_removed` | After adapter remove | `{"adapter": string}` |
| `mcp_config_deferred` | MCP config failed in add | `{"adapter": string, "error": string}` |
| `launch_started` | Before exec into LLM | `{"adapter": string, "version": string}` |
| `launch_failed` | Launch error | `{"adapter": string, "reason": string}` |
| `error` | Any command error | `{"type": string, "message": string}` |

### Key Metrics

| Metric | What It Measures |
|--------|------------------|
| `init_completed` → `adapter_added` time | Multi-step flow friction |
| `adapter_refreshed` count | Backup/restore usage |
| `launch_failed` by reason | What blocks users |
| `--force` usage frequency | Escape hatch overuse (design smell) |

### `patina stats` Command

```bash
patina stats              # Summary of recent activity
patina stats --detailed   # Full event log
patina stats --since 7d   # Last 7 days
```

---

## Implementation

1. Create `src/db/events.rs` with SQLite schema
2. Add `log_event()` helper function
3. Add event logging calls to init, adapter, launch commands
4. Create `patina stats` command

---

## Why Deferred

- Core commands work without observability
- Need real usage data to know what metrics matter
- Can add incrementally when specific questions arise
- Local-only (no cloud telemetry concerns)
