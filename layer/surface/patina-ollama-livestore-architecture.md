# Patina: Mac-Native Knowledge Server with LiveStore Events

**Model**: Ollama-style server (Mac Metal/MLX) + Container development + LiveStore event sourcing
**Use Case**: Hackathon accelerator for blockchain games (Dust, Death Mountain, Starknet/Ethereum)

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────┐
│  Mac Studio - Patina Knowledge Server                   │
│  ────────────────────────────────────────────────       │
│                                                          │
│  📦 Event Store (LiveStore Pattern)                     │
│     ~/.patina/events/                                    │
│     ├── dust/                                            │
│     │   ├── 2025-11-18-001-ecs-pattern.json             │
│     │   └── 2025-11-18-002-gas-optimization.json        │
│     ├── death-mountain/                                  │
│     │   └── 2025-11-15-001-hook-architecture.json       │
│     └── daydreams/                                       │
│         └── 2025-11-10-001-agent-pattern.json           │
│                                                          │
│  💾 Materialized Views (SQLite)                         │
│     ~/.patina/knowledge.db                              │
│     - observations (all projects, all time)             │
│     - patterns (recurring solutions)                    │
│     - domains (auto-tagged: ecs, gas-opt, hooks)        │
│     - project_connections (what relates to what)        │
│                                                          │
│  🧠 Embeddings (Metal/MLX Accelerated)                  │
│     - E5-base-v2 model loaded once                      │
│     - 768-dim vectors, <10ms search                     │
│     - Cross-project semantic search                     │
│                                                          │
│  🔌 API Server (gRPC :50051)                            │
│     - /events/append (projects emit events)             │
│     - /query/semantic (search across all projects)      │
│     - /patterns/for-domain (get patterns for domain)    │
│     - /materialize (rebuild SQLite from events)         │
│                                                          │
└──────────────────┬──────────────────────────────────────┘
                   │ host.docker.internal:50051
         ──────────┴─────────────────────────
         │                │                 │
    ┌────▼─────┐    ┌────▼──────┐    ┌─────▼────────┐
    │ Dust     │    │ Death Mtn │    │ New Hackathon│
    │ (Solidity│    │ (Solidity)│    │ (Starknet)   │
    │  + ECS)  │    │           │    │              │
    │          │    │           │    │              │
    │ Claude   │    │ Gemini    │    │ Claude       │
    │ .patina/ │    │ .patina/  │    │ .patina/     │
    └──────────┘    └───────────┘    └──────────────┘
    container       container         container
```

---

## LiveStore Event Pattern

### Event Store Structure

**Events are immutable JSON files, organized by project:**

```
~/.patina/events/
├── dust/
│   ├── 2025-11-18-001-observation-captured.json
│   ├── 2025-11-18-002-pattern-identified.json
│   ├── 2025-11-18-003-decision-made.json
│   └── manifest.json (metadata: last_sequence, project_info)
│
├── death-mountain/
│   ├── 2025-11-15-001-observation-captured.json
│   └── manifest.json
│
└── daydreams/
    ├── 2025-11-10-001-observation-captured.json
    └── manifest.json
```

### Event Schema (LiveStore-inspired)

**Base Event Format:**
```json
{
  "schema_version": "1.0.0",
  "event_id": "dust_20251118_001",
  "event_type": "observation_captured",
  "timestamp": "2025-11-18T15:30:00Z",
  "project": "dust",
  "sequence": 1,
  "author": "nicabar",
  "payload": {
    "content": "Used ECS component pattern for player inventory with gas optimization",
    "observation_type": "pattern",
    "domains": ["ecs", "gas-optimization", "solidity"],
    "source_type": "session",
    "source_id": "20251118-155141",
    "code_refs": [
      "contracts/components/Inventory.sol:42-67"
    ],
    "reliability": 0.95
  }
}
```

**Event Types:**
- `observation_captured` - Saw something happen (code pattern, decision)
- `pattern_identified` - Recurring solution across multiple observations
- `decision_made` - Explicit choice with rationale
- `challenge_encountered` - Problem + solution
- `domain_tagged` - LLM classified observation into domain

### Materialization (LiveStore Pattern)

**SQLite = Derived State (Rebuild from events anytime):**

```sql
-- ~/.patina/knowledge.db

-- Materialized view of all observations
CREATE TABLE observations (
    id TEXT PRIMARY KEY,
    project TEXT NOT NULL,
    content TEXT NOT NULL,
    observation_type TEXT,
    domains TEXT, -- JSON array
    created_at TIMESTAMP,
    event_id TEXT NOT NULL, -- References source event file
    UNIQUE(event_id)
);

-- Materialized view of patterns
CREATE TABLE patterns (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    domains TEXT, -- JSON array
    projects TEXT, -- JSON array of projects where seen
    observation_count INTEGER,
    first_seen TIMESTAMP,
    last_seen TIMESTAMP
);

-- Domain relationships (co-occurrence)
CREATE TABLE domain_connections (
    domain_a TEXT,
    domain_b TEXT,
    co_occurrence_count INTEGER,
    projects TEXT, -- JSON array
    PRIMARY KEY (domain_a, domain_b)
);

-- Track materialization state
CREATE TABLE materialization_state (
    project TEXT PRIMARY KEY,
    last_event_sequence INTEGER,
    last_materialized_at TIMESTAMP
);
```

**Rebuild anytime:**
```bash
# Delete materialized views
rm ~/.patina/knowledge.db

# Rebuild from event log
patina materialize
  ↓
Reads ALL event JSON files from ~/.patina/events/*
  ↓
Populates SQLite tables
  ↓
Builds vector indices from observations
  ↓
Done - full knowledge graph restored
```

---

## Container Development Workflow

### Project Structure (Inside Container)

```
/workspace/dust/
├── contracts/              # Solidity game contracts
├── .patina/
│   ├── config.toml         # Points to Mac server
│   └── local/              # Container-local cache
│       └── last_sync.json
└── .claude/
    └── CLAUDE.md           # Custom commands for this project
```

### Patina Config (Inside Container)

```toml
# /workspace/dust/.patina/config.toml

[project]
name = "dust"
type = "blockchain-game"
domains = ["ecs", "solidity", "ethereum", "game-design"]

[server]
# Mac server (host.docker.internal from container)
host = "host.docker.internal"
port = 50051
protocol = "grpc"

[sync]
# Emit events to Mac server in real-time
auto_emit = true
batch_size = 10
```

### LLM Integration (Baked Into Project)

**`.claude/CLAUDE.md` for Dust project:**
```markdown
# Dust - Blockchain ECS Game

## Project Context
This is a Solidity-based game using Entity-Component-System architecture.

## Knowledge Commands

Use these to query accumulated knowledge:

- `/ask-patina "gas optimization for ECS"` - Query Mac knowledge server
- `/patterns-for "hooks"` - Get hook patterns from all projects
- `/similar-to-dust` - Find related patterns from other projects

## Hackathon Mode

When starting hackathon session:
1. `/session-start "Hackathon: [feature]"`
2. Work on feature
3. `/emit-knowledge` - Send learnings to Mac server
4. `/session-end` - Archive session

All knowledge automatically flows to Patina Mac server.
```

### Workflow: Building New Hackathon Project

**Scenario:** User enters Starknet hackathon, wants to build game like Dust

```bash
# On Mac
patina init starknet-game --template=blockchain-game --based-on=dust

# Creates:
# - Container config (Dockerfile, devcontainer.json)
# - .patina/config.toml (points to Mac server)
# - .claude/CLAUDE.md (with slash commands)
# - Pulls patterns from Dust into starter code

# Start container
docker compose up -d starknet-game

# Inside container
docker exec -it starknet-game bash

# LLM (Claude) can now query Mac server
/ask-patina "How do I structure ECS components in Cairo?"

# Mac server responds:
# "In Dust (Solidity), you used this pattern: [shows code]
#  In Cairo, similar approach: [generates Cairo version]"
```

---

## Knowledge Flow: Project → Mac Server → Other Projects

### 1. Emit Events from Container

**Inside Dust container, session ends:**
```bash
# User: /session-end

# .claude/bin/session-end.sh runs:
patina emit session 20251118-155141

# patina CLI (inside container):
1. Parses session markdown
2. Extracts observations via LLM
3. Auto-tags domains: ["ecs", "gas-optimization", "solidity"]
4. Creates event JSON
5. Calls Mac server: POST /events/append
```

**gRPC call to Mac:**
```protobuf
message AppendEventRequest {
  string project = 1;
  string event_json = 2; // Full event as JSON string
}

// Mac server receives:
{
  "project": "dust",
  "event_json": "{\"event_id\":\"dust_20251118_042\", ...}"
}
```

**Mac server:**
```rust
// Receives event
1. Validates event schema
2. Assigns sequence number
3. Writes to ~/.patina/events/dust/2025-11-18-042-observation-captured.json
4. Appends to materialization queue
5. Returns ACK to container
```

### 2. Materialize Knowledge (Incremental)

**Mac server background process:**
```rust
// Every 5 minutes OR on /materialize request
fn materialize_incremental() {
    // Read materialization_state table
    for project in projects {
        let last_seq = get_last_materialized_sequence(project);

        // Read new events since last_seq
        let new_events = read_events_after(project, last_seq);

        for event in new_events {
            match event.event_type {
                "observation_captured" => {
                    insert_observation(&event);
                    update_domain_connections(&event);
                }
                "pattern_identified" => {
                    insert_pattern(&event);
                }
                _ => {}
            }
        }

        // Update state
        update_materialization_state(project, new_events.last_seq);
    }

    // Rebuild vector indices (only new observations)
    update_embeddings_incremental();
}
```

### 3. Query from Another Project

**In new hackathon container (Starknet game):**
```bash
# User asks Claude: "How should I optimize gas for ECS game?"

# Claude knows custom command from .claude/CLAUDE.md:
/ask-patina "gas optimization for ECS game"

# patina CLI (inside container):
patina query "gas optimization for ECS game"

# Calls Mac server: POST /query/semantic
```

**Mac server processes query:**
```rust
fn query_semantic(query: &str, limit: usize) -> Vec<Observation> {
    // 1. Generate query embedding (Metal/MLX accelerated)
    let query_vec = embedder.embed_query(query); // <10ms

    // 2. Search vector index (all projects)
    let results = usearch_index.search(&query_vec, limit);

    // 3. Load observations from SQLite
    let observations = results.iter()
        .map(|result| {
            db.query("SELECT * FROM observations WHERE id = ?", result.id)
        })
        .collect();

    // 4. Group by project + domain
    group_by_context(observations)
}
```

**Returns to container:**
```json
{
  "results": [
    {
      "content": "Used ECS component pattern with gas optimization by caching component lookups",
      "project": "dust",
      "domains": ["ecs", "gas-optimization", "solidity"],
      "relevance": 0.94,
      "code_ref": "contracts/components/Inventory.sol:42-67"
    },
    {
      "content": "Hook architecture for ECS systems minimizes storage reads",
      "project": "death-mountain",
      "domains": ["ecs", "hooks", "solidity"],
      "relevance": 0.87,
      "code_ref": "src/hooks/ComponentHooks.sol:15-32"
    }
  ]
}
```

**Claude receives context, generates Cairo code:**
```typescript
// Claude sees patterns from Dust + Death Mountain
// Generates Starknet/Cairo version:

@storage_var
func component_cache(entity_id: felt, component_type: felt) -> (data: felt):
end

// Pattern adapted from Dust's gas-optimized ECS
func get_component{syscall_ptr, pedersen_ptr, range_check_ptr}(
    entity_id: felt,
    component_type: felt
) -> (data: felt):
    let (cached_data) = component_cache.read(entity_id, component_type)
    if cached_data != 0:
        return (cached_data)
    end
    # Cache miss - load from storage
    # ... (pattern from Death Mountain hooks)
end
```

---

## Domain & Project Connection

### Domain Auto-Tagging (LLM-Driven)

**During event emission:**
```rust
// Inside container: patina emit session 20251118-155141

fn extract_observations(session_markdown: &str) -> Vec<Event> {
    // LLM prompt:
    let prompt = format!(r#"
        Read this development session and extract observations.
        For each observation, tag relevant domains.

        Known domains across projects:
        - ecs (entity-component-system)
        - gas-optimization
        - solidity, cairo, rust
        - hooks, events
        - game-design, blockchain

        Session:
        {}

        Return JSON array of observations with auto-tagged domains.
    "#, session_markdown);

    let response = llm.complete(prompt); // Claude/Gemini
    parse_observations(response)
}
```

**LLM auto-tags:**
```json
[
  {
    "content": "Implemented ECS component caching to reduce storage reads",
    "observation_type": "pattern",
    "domains": ["ecs", "gas-optimization", "solidity", "caching"],
    "code_refs": ["contracts/components/Inventory.sol:42-67"]
  }
]
```

### Cross-Project Domain Connections

**Mac server discovers relationships during materialization:**

```rust
// After materializing events, analyze domain co-occurrence

fn discover_domain_connections(db: &Database) {
    // Find domains that appear together
    let query = r#"
        SELECT
            d1.domain as domain_a,
            d2.domain as domain_b,
            COUNT(*) as co_occurrence,
            GROUP_CONCAT(DISTINCT project) as projects
        FROM (
            SELECT id, project, json_each.value as domain
            FROM observations, json_each(observations.domains)
        ) d1
        JOIN (
            SELECT id, json_each.value as domain
            FROM observations, json_each(observations.domains)
        ) d2 ON d1.id = d2.id AND d1.domain < d2.domain
        GROUP BY d1.domain, d2.domain
        HAVING co_occurrence >= 3
    "#;

    // Inserts into domain_connections table:
    // ("ecs", "gas-optimization", 47, ["dust", "death-mountain"])
    // ("hooks", "ecs", 23, ["dust", "death-mountain"])
    // ("solidity", "cairo", 8, ["dust", "starknet-game"])
}
```

**Query with domain expansion:**
```rust
fn query_with_domain_expansion(query: &str) -> Vec<Observation> {
    // Extract domains from query
    let query_domains = extract_domains(query); // ["ecs", "gas-optimization"]

    // Find related domains
    let related = db.query(r#"
        SELECT domain_b, co_occurrence_count
        FROM domain_connections
        WHERE domain_a IN (?, ?)
        ORDER BY co_occurrence_count DESC
        LIMIT 5
    "#, query_domains);
    // Returns: ["hooks", "caching", "storage-patterns"]

    // Expand search to include related domains
    let expanded_domains = query_domains + related.map(|r| r.domain_b);

    // Search with expanded context
    semantic_search_with_domains(query, expanded_domains)
}
```

---

## Hackathon Workflow: End-to-End

### Before Hackathon: Accumulate Knowledge

```bash
# Work on Dust for 3 months
cd ~/projects/dust
docker compose up -d
# ... sessions, commits, learnings
# Each session emits events to Mac server

# Work on Death Mountain for 2 months
cd ~/projects/death-mountain
# ... more sessions
# More events to Mac server

# Mac server now has:
# - 150+ observations from Dust (ECS, Solidity, gas-opt)
# - 80+ observations from Death Mountain (hooks, game design)
# - Domain connections discovered automatically
```

### Hackathon Day 1: Starknet Game

**Initialize new project:**
```bash
cd ~/hackathons/
patina init starknet-racing-game \
  --template=blockchain-game \
  --based-on=dust,death-mountain \
  --stack=cairo,starknet

# Patina:
1. Creates Dockerfile with Cairo tools
2. Creates .patina/config.toml (Mac server connection)
3. Analyzes Dust + Death Mountain patterns
4. Generates starter code with ECS architecture
5. Creates .claude/CLAUDE.md with domain-specific commands
```

**Starter code includes patterns from Dust:**
```cairo
# Generated src/components/PlayerComponent.cairo
# Based on Dust's Inventory.sol pattern

@storage_var
func player_components(player_id: felt, component_id: felt) -> (data: felt):
end

# Pattern: Gas-optimized ECS component access
# Source: dust/contracts/components/Inventory.sol
# Adapted for Cairo/Starknet
```

**Start development:**
```bash
docker compose up -d starknet-racing-game
docker exec -it starknet-racing-game bash

# Inside container
/session-start "Hackathon Day 1: Core ECS System"
```

**Development with LLM:**
```
User: "I need to add player stats component"

Claude: Let me check what patterns you've used before.
[calls: /ask-patina "ECS component patterns"]

Mac server returns:
- Dust: Component caching pattern (gas-optimized)
- Death Mountain: Hook-based component updates
- Domains connected: ecs + hooks + gas-optimization

Claude: Based on your Dust and Death Mountain projects, here's
the Cairo version with your gas optimization patterns...

[generates code using patterns from both projects]
```

**Emit knowledge back:**
```bash
# End of day
/session-end

# patina emit runs:
1. Parses session
2. LLM extracts: "Applied Dust's caching pattern to Cairo/Starknet"
3. Auto-tags: ["cairo", "starknet", "ecs", "gas-optimization", "pattern-reuse"]
4. Creates event
5. Sends to Mac server

# Mac server:
1. Stores event
2. Materializes: New observation in starknet-racing-game
3. Updates domain connections: ("cairo", "solidity") co-occurrence++
4. Next query can find Cairo ↔ Solidity pattern mappings
```

### Hackathon Day 2: Speed Boost

**Morning query:**
```
User: "How do I optimize batch transactions in Cairo?"

Claude: /ask-patina "batch transaction optimization"

Mac server:
1. Searches across all projects
2. Finds: Dust had batch minting pattern
3. Finds: Yesterday's Cairo work has partial pattern
4. Returns: Combined knowledge

Claude: You used batch minting in Dust's NFT system. Here's
the Cairo equivalent for your racing game...
```

**Knowledge compounds:**
- Day 1 learnings already in server
- Day 2 builds on Day 1 + Dust + Death Mountain
- Each session makes future sessions smarter

---

## Mac Server Implementation

### Server Structure

```
patina-server/
├── src/
│   ├── server/
│   │   ├── grpc.rs          # gRPC endpoints
│   │   ├── http.rs          # Optional HTTP API
│   │   └── state.rs         # Server state
│   ├── events/
│   │   ├── store.rs         # Event file management
│   │   ├── validator.rs     # Event schema validation
│   │   └── sequence.rs      # Sequence number management
│   ├── materialize/
│   │   ├── engine.rs        # Materialization engine
│   │   ├── incremental.rs   # Incremental updates
│   │   └── rebuild.rs       # Full rebuild
│   ├── embeddings/
│   │   ├── metal.rs         # Metal/MLX backend (Mac)
│   │   ├── onnx.rs          # ONNX fallback (Linux)
│   │   └── cache.rs         # Embedding cache
│   ├── query/
│   │   ├── semantic.rs      # Vector search
│   │   ├── domain.rs        # Domain-aware queries
│   │   └── patterns.rs      # Pattern matching
│   └── sync/
│       ├── project.rs       # Project registration
│       └── state.rs         # Sync state tracking
└── .patina/
    ├── events/              # Event store
    ├── knowledge.db         # Materialized SQLite
    └── vectors/             # USearch indices
```

### Key APIs

```protobuf
// patina.proto

service PatinaKnowledge {
  // Event emission
  rpc AppendEvent(AppendEventRequest) returns (AppendEventResponse);
  rpc AppendBatch(AppendBatchRequest) returns (AppendBatchResponse);

  // Queries
  rpc QuerySemantic(QueryRequest) returns (QueryResponse);
  rpc GetPatternsForDomain(DomainRequest) returns (PatternsResponse);
  rpc FindSimilarProjects(SimilarityRequest) returns (ProjectsResponse);

  // Materialization
  rpc Materialize(MaterializeRequest) returns (MaterializeResponse);
  rpc GetMaterializationState(ProjectRequest) returns (StateResponse);

  // Domain intelligence
  rpc GetDomainConnections(DomainRequest) returns (ConnectionsResponse);
  rpc AutoTagContent(TagRequest) returns (TagResponse);
}
```

### Server Startup

```rust
// patina-server/src/main.rs

#[tokio::main]
async fn main() -> Result<()> {
    println!("🚀 Starting Patina Knowledge Server");

    // 1. Load embeddings model (Metal/MLX accelerated)
    let embedder = MetalEmbedder::new("e5-base-v2")?;
    println!("✅ Embeddings loaded (Metal/MLX)");

    // 2. Open event store
    let event_store = EventStore::open("~/.patina/events")?;
    println!("✅ Event store ready");

    // 3. Open materialized database
    let db = Database::open("~/.patina/knowledge.db")?;
    println!("✅ Knowledge database ready");

    // 4. Load vector indices
    let vector_index = VectorIndex::open("~/.patina/vectors/observations.usearch")?;
    println!("✅ Vector indices loaded");

    // 5. Start background materializer
    let materializer = Materializer::start(event_store.clone(), db.clone());
    println!("✅ Background materializer started");

    // 6. Create server state
    let state = ServerState {
        event_store,
        db,
        embedder,
        vector_index,
    };

    // 7. Start gRPC server
    let addr = "0.0.0.0:50051".parse()?;
    Server::builder()
        .add_service(PatinaKnowledgeServer::new(state))
        .serve(addr)
        .await?;

    Ok(())
}
```

---

## Why This Architecture Works for Hackathons

### 1. **Instant Context Recall**
```
You: "How did I optimize gas in Dust?"
Mac: [<10ms search across 150 observations]
Claude: "Here's the caching pattern you used..."
```

### 2. **Cross-Project Learning**
```
Dust (Solidity) → Patterns → Starknet (Cairo)
"You used X in Solidity, here's Cairo equivalent"
```

### 3. **Domain Intelligence**
```
LLM auto-tags: "ecs" + "gas-optimization" + "solidity"
Mac discovers: These often co-occur
Next query expands: Search includes related patterns
```

### 4. **Container Portability**
```
Dockerfile for each hackathon stack:
- Solidity: foundry + slither
- Cairo: scarb + starknet
- Rust: cargo + cross
All call same Mac server for knowledge
```

### 5. **Accumulating Advantage**
```
Hackathon 1: Build from scratch + accumulate learnings
Hackathon 2: Start with Hackathon 1 knowledge
Hackathon 3: Start with Hackathon 1 + 2 knowledge
Each one faster than the last
```

---

## Deferred Complexity: What We're NOT Building Now

**Later (when proven necessary):**
- ❌ Prolog reasoning (just SQLite + vectors for now)
- ❌ Multi-user collaboration (single persona, multiple projects)
- ❌ Forced persona beliefs (just observations + patterns)
- ❌ Complex belief validation (simple pattern matching)
- ❌ P2P sync (local-only Mac server)

**Now (minimum viable):**
- ✅ LiveStore events (JSON files)
- ✅ SQLite materialized views
- ✅ Vector search (embeddings)
- ✅ Domain auto-tagging (LLM)
- ✅ Cross-project queries
- ✅ Container development
- ✅ Mac Metal/MLX acceleration

---

## Next Steps: Build This

### Phase 1: Mac Server (1-2 weeks)
1. Event store (JSON file management)
2. SQLite schema + materialization
3. Embeddings with Metal/MLX
4. gRPC server with basic queries

### Phase 2: Container Integration (1 week)
1. Patina CLI for containers
2. `.patina/config.toml` schema
3. Event emission from containers
4. Query commands for LLMs

### Phase 3: LLM Integration (1 week)
1. Custom slash commands (`.claude/CLAUDE.md`)
2. Auto-tagging with Claude/Gemini
3. Session extraction
4. Pattern recognition

### Phase 4: Hackathon Test (1 hackathon)
1. Real Starknet/Ethereum hackathon
2. Measure: Does it save time?
3. Iterate based on reality
4. Prove the concept works

**Total: ~1 month to hackathon-ready MVP**
