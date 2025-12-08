# Spec: patina serve

**Status:** Complete (2025-12-08)
**Phase:** 4 (Core Infrastructure)
**Location:** `src/commands/serve/`, `src/mothership/`

---

## Purpose

HTTP daemon for container queries, hot model caching, and cross-project search. Follows Ollama pattern - single binary, lazy loading, REST API.

**Aggregates:** Patina projects (full RAG) + reference repos (lightweight indices) + persona (cross-project knowledge)

---

## Architecture

```
Mac (Mothership)                    Container
┌─────────────────────┐            ┌─────────────────────┐
│ patina serve        │            │ patina scry "query" │
│ localhost:50051     │◄───────────│ PATINA_MOTHERSHIP   │
│                     │   HTTP     │ =host.docker.internal│
│ ┌─────────────────┐ │            └─────────────────────┘
│ │ E5 Model (hot)  │ │
│ │ Projections     │ │
│ │ ~/.patina/repos │ │
│ └─────────────────┘ │
└─────────────────────┘
```

---

## API Endpoints

| Method | Endpoint | Purpose | Status |
|--------|----------|---------|--------|
| GET | `/health` | Health check | ✅ Done |
| POST | `/api/scry` | Query (semantic/lexical/file) | ✅ Done |
| POST | `/api/embed` | Generate embedding | Planned |
| POST | `/api/embed/batch` | Batch embeddings | Planned |
| GET | `/api/repos` | List repos | Planned |
| GET | `/api/repos/{name}` | Repo details | Planned |
| GET | `/api/model` | Model status | Planned |
| GET | `/api/persona` | Query persona | Planned (4d) |

---

## CLI Interface

```bash
# Start daemon (foreground)
patina serve

# Start on specific port
patina serve --port 8080

# Bind to all interfaces (for containers)
patina serve --host 0.0.0.0

# Background mode
patina serve --daemon
```

---

## Implementation Phases

### Phase 1: Basic Daemon ✅
- [x] Add `rouille = "3.6"` dependency
- [x] Create `src/commands/serve/` module
- [x] Implement `/health` endpoint
- [x] Add `Serve` command to CLI

### Phase 2: Model Caching + Embed API
- [ ] ServerState with `parking_lot::RwLock`
- [ ] `/api/embed` and `/api/embed/batch` endpoints
- [ ] Thread-safe embedder access
- [ ] Lazy model loading on first request

### Phase 3: Scry API + Client Detection ✅ (2025-12-08)
- [x] `/api/scry` endpoint (semantic/lexical/file)
- [x] Mothership client module in `src/mothership/`
- [x] Auto-detection: `PATINA_MOTHERSHIP` env var
- [x] Update scry command to route to daemon when available
- [ ] Persona integration (`include_persona` option) - after 4d

### Phase 4: Container Integration
- [ ] `--host 0.0.0.0` option for container access
- [ ] Update devcontainer with `PATINA_MOTHERSHIP` env var
- [ ] Test container → Mac queries

### Phase 5: Cross-Repo + Model APIs
- [ ] `/api/repos` endpoints
- [ ] `/api/model` status endpoint
- [ ] `--all-repos` flag for cross-repo queries
- [ ] Graceful shutdown (SIGTERM)

---

## Design Decisions

| Decision | Choice | Why |
|----------|--------|-----|
| HTTP library | rouille | Blocking, no async/tokio, simple |
| Pattern | Ollama-style | Single binary, subcommand, lazy loading |
| Protocol | HTTP REST | Simpler than gRPC, curl-friendly |
| Port | 50051 | Doesn't conflict with common services |

---

## File Structure

```
src/commands/serve/
├── mod.rs              # Public interface
└── internal.rs         # Server implementation

src/mothership/         # Client for daemon
├── mod.rs              # Client interface
└── internal.rs         # HTTP client
```

---

## Configuration

```toml
# ~/.patina/config.toml
[daemon]
port = 50051
host = "127.0.0.1"
keep_alive = "5m"       # Model eviction timeout
max_memory_mb = 2048
```

---

## Validation Criteria

**4e complete when:**
1. [x] `patina serve` exposes `/api/scry` endpoint
2. [x] `patina scry` detects daemon and routes queries
3. [x] Container can query Mac via `PATINA_MOTHERSHIP` env var
4. [x] `patina scry --all-repos` queries across registry
5. [ ] Model stays hot between requests (lazy loading works) - deferred to Phase 2
6. [ ] `/api/scry` supports `include_persona` option (after 4d)
