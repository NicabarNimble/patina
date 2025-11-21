# Spec: Mothership Service

## Overview
Mothership is an Ollama-style daemon that provides embedding generation, unified queries (scry), and project registry. It runs locally on each Mac, manages persona beliefs, and enables cross-project knowledge queries.

**What mothership does:**
- Generates embeddings (shared model, loaded once)
- Unified scry queries (project + persona)
- Manages project registry
- Tracks recipe versions

**What mothership does NOT do:**
- Watch for file changes (scrape does that)
- Sync events (git does that)
- Aggregate across users (each user has own mothership)

## Architecture
```
┌─────────────────────────────────────────────────────────┐
│  patina serve (daemon on :50051)                        │
│                                                          │
│  • Runs as background daemon (like ollama serve)        │
│  • ~/.patina/ is data directory                         │
│  • REST API (axum)                                      │
│                                                          │
│  Endpoints:                                              │
│  • POST /embed         - generate embeddings            │
│  • POST /scry          - unified query (project+persona)│
│  • GET  /projects      - list registered projects       │
│  • POST /projects      - register project               │
│  • GET  /health        - health check                   │
│  • GET  /status        - recipe versions, build state   │
│                                                          │
└─────────────────────────────────────────────────────────┘
```

## Directory Structure
```
~/.patina/                       # Mothership data directory
├── projects.registry            # YAML: known projects
├── persona/
│   ├── events/                  # Mothership's OWN events
│   ├── beliefs.db               # Materialized beliefs
│   └── beliefs.usearch          # Persona embeddings
├── cache/
│   └── models/                  # Downloaded ONNX models
├── config.toml                  # Service configuration
└── patina.sock                  # Unix socket (optional)
```

## Components

### 1. Service Daemon
**Command:** `patina serve`

**Location:** `src/commands/serve/mod.rs`

**Implementation:**
```rust
// src/commands/serve/mod.rs
use axum::{Router, routing::{get, post}};

pub async fn serve(port: u16) -> Result<()> {
    let app = Router::new()
        .route("/health", get(health))
        .route("/embed", post(embed))
        .route("/persona/query", post(persona_query))
        .route("/projects", get(list_projects).post(register_project));

    let addr = format!("127.0.0.1:{}", port);
    axum::Server::bind(&addr.parse()?)
        .serve(app.into_make_service())
        .await?;
    Ok(())
}
```

**CLI:**
```bash
patina serve                    # Start on default port 50051
patina serve --port 8080        # Custom port
patina serve --background       # Daemonize
```

**Lifecycle:**
- macOS: `brew services start patina` (launchd)
- Linux: systemd unit file
- Manual: `patina serve &`

### 2. Embed Endpoint
**Endpoint:** `POST /embed`

**Request:**
```json
{
  "texts": ["function to calculate fibonacci", "error handling pattern"],
  "model": "e5-base-v2"  // optional, default
}
```

**Response:**
```json
{
  "embeddings": [
    [0.123, -0.456, ...],  // 768 floats
    [0.789, -0.012, ...]
  ],
  "model": "e5-base-v2",
  "dimensions": 768
}
```

**Implementation:**
- Uses `src/embeddings/onnx.rs` (existing)
- Model loaded once at startup, reused
- Batch processing for efficiency

### 3. Scry Endpoint (Unified Query)
**Endpoint:** `POST /scry`

**Request:**
```json
{
  "query": "error handling patterns",
  "project": "livestore",        // optional, defaults to cwd project
  "include_persona": true,       // optional, default true
  "limit": 10,
  "threshold": 0.7
}
```

**Response:**
```json
{
  "results": [
    {
      "content": "TypeScript prefers Result types here",
      "source": "project",
      "tag": "[PROJECT]",
      "similarity": 0.92,
      "metadata": {
        "session_id": "20251121-065812",
        "domains": ["typescript", "error-handling"]
      }
    },
    {
      "content": "Always use Result<T, E> over panics",
      "source": "persona",
      "tag": "[PERSONA]",
      "similarity": 0.89,
      "metadata": {
        "domains": ["rust", "error-handling"]
      }
    }
  ],
  "sources": {
    "project_count": 3,
    "persona_count": 2
  }
}
```

**Implementation:**
```rust
pub async fn scry(request: ScryRequest) -> Result<ScryResponse> {
    let mut results = Vec::new();

    // 1. Query project
    if let Some(project) = get_project(&request.project)? {
        let project_results = query_project_vectors(
            &project.path,
            &request.query,
            request.limit,
        )?;
        for r in project_results {
            results.push(TaggedResult {
                content: r.content,
                source: "project",
                tag: "[PROJECT]",
                similarity: r.similarity,
                metadata: r.metadata,
            });
        }
    }

    // 2. Query persona
    if request.include_persona {
        let persona_results = query_persona_vectors(
            &request.query,
            request.limit,
        )?;
        for r in persona_results {
            results.push(TaggedResult {
                content: r.content,
                source: "persona",
                tag: "[PERSONA]",
                similarity: r.similarity * 0.95,  // slight penalty for non-local
                metadata: r.metadata,
            });
        }
    }

    // 3. Sort by similarity
    results.sort_by(|a, b| b.similarity.partial_cmp(&a.similarity).unwrap());
    results.truncate(request.limit);

    Ok(ScryResponse { results, sources })
}
```

### 4. Persona Query Endpoint (Direct)
**Endpoint:** `POST /persona/query`

For querying persona only (without project context).

**Request:**
```json
{
  "query": "error handling patterns",
  "limit": 10,
  "threshold": 0.7
}
```

**Response:**
```json
{
  "results": [
    {
      "content": "Always use Result<T, E> over panics",
      "domains": ["rust", "error-handling"],
      "similarity": 0.89
    }
  ]
}
```

**Implementation:**
- Query `~/.patina/persona/beliefs.usearch`
- Join with `beliefs.db` for metadata

### 5. Projects Registry
**Location:** `~/.patina/projects.registry`

**Format (YAML):**
```yaml
projects:
  patina:
    path: /Users/nicabar/Projects/patina
    type: primary
    last_indexed: 2025-11-21T06:32:28Z
    patina_thickness: working

  livestore:
    path: /Users/nicabar/Projects/livestore
    type: reference
    last_indexed: 2025-11-20T10:00:00Z
    patina_thickness: fresh
```

**Endpoints:**
- `GET /projects` - list all registered
- `POST /projects` - register new project
- `DELETE /projects/{name}` - unregister

**Auto-registration:**
- `patina init` registers project with mothership
- Updates `last_indexed` on `patina materialize`

### 6. Configuration
**Location:** `~/.patina/config.toml`

```toml
[service]
port = 50051
bind = "127.0.0.1"

[models]
default = "e5-base-v2"
cache_dir = "~/.patina/cache/models"

[persona]
events_dir = "~/.patina/persona/events"
```

## Dependencies
```toml
# Cargo.toml additions
axum = "0.7"
tokio = { version = "1", features = ["full"] }
tower = "0.4"
tower-http = { version = "0.5", features = ["cors"] }
```

## CLI Integration
```rust
// src/main.rs - add to CLI
Commands::Serve { port, background } => {
    if background {
        daemonize()?;
    }
    serve::serve(port.unwrap_or(50051)).await
}
```

## Acceptance Criteria
- [ ] `patina serve` starts daemon on :50051
- [ ] `curl localhost:50051/health` returns OK
- [ ] `POST /embed` returns embeddings for given texts
- [ ] `POST /scry` returns combined project + persona results
- [ ] Results correctly tagged as [PROJECT] or [PERSONA]
- [ ] `patina init` registers project in registry
- [ ] `GET /projects` lists all registered projects
- [ ] `GET /status` shows recipe versions and build state
- [ ] Containers can query via `host.docker.internal:50051`
