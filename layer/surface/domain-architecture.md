---
id: domain-architecture
version: 1
status: draft
created_date: 2025-08-02
updated_date: 2025-08-02
oxidizer: nicabar
tags: [architecture, domains, federation, multiplayer, local-first]
---

# Patina Domain Architecture

Patina creates self-contained knowledge domains that can optionally connect to form knowledge networks.

## Core Concept: A Domain = A Complete Knowledge Tool

```
┌─────────────────────────────────┐
│      PATINA DOMAIN              │
│   (Self-contained tool)         │
├─────────────────────────────────┤
│ • Has its own layer/            │
│ • Has its own indexer           │
│ • Has its own rqlite            │
│ • Domain-specific patterns      │
│ • Works completely standalone   │
└─────────────────────────────────┘
              │
              │ CAN optionally connect to...
              ▼
┌─────────────────────────────────┐
│   ANOTHER PATINA DOMAIN         │
│   (shared knowledge)            │
└─────────────────────────────────┘
```

## Design Principles

### 1. Domain Independence
Each domain is a complete, functional Patina instance:
- Full layer structure (core/surface/dust)
- Own indexer for navigation
- Own rqlite for persistence
- No external dependencies required

### 2. Local-First Operation
- Everything works offline
- No network required for core functionality
- Knowledge persists locally
- Privacy by default

### 3. Optional Federation
When connected, domains can:
- Share patterns with peers
- Query other domain indexes
- Subscribe to knowledge updates
- Form knowledge networks

## Domain Types

### Development Domains
```bash
patina init my-rust-project --type=tool
# Creates domain for Rust development patterns
# Inherits from rust-patterns domain if available
```

### Data Integration Domains
```bash
patina init game-analytics --type=tool
# Domain that pulls from external sources
# Transforms live data into patterns
# Example: blockchain → knowledge
```

### Personal Domains
```bash
patina init my-notes --type=personal
# Private knowledge management
# Never federates unless explicit
```

## Architecture Components

### 1. Domain Core
```rust
pub struct Domain {
    name: String,
    layer: LayerSystem,
    indexer: PatternIndexer,
    db: RqliteInstance,
    adapters: Vec<Box<dyn DataAdapter>>,
}
```

### 2. Data Adapters
Domains can have adapters for external data:
```rust
pub trait DataAdapter {
    // Pull data from source
    fn fetch(&self) -> Result<RawData>;
    
    // Transform to patterns
    fn transform(&self, data: RawData) -> Result<Vec<Pattern>>;
    
    // Write to layer
    fn persist(&self, patterns: Vec<Pattern>) -> Result<()>;
}
```

### 3. Federation Protocol
```rust
pub trait Federation {
    // Connect to peer domain
    fn connect(&mut self, peer: &str) -> Result<()>;
    
    // Query across domains
    fn federated_query(&self, query: &str) -> NavigationResponse;
    
    // Subscribe to patterns
    fn subscribe(&mut self, domain: &str, pattern: &str) -> Result<()>;
}
```

## Usage Examples

### Single Domain (Most Common)
```bash
# Initialize domain
patina init my-project --type=app

# Work within domain
cd my-project
patina add pattern authentication-flow
patina commit -m "Add JWT auth pattern"

# Domain is fully self-contained
# No external connections needed
```

### Federated Domains (Advanced)
```bash
# Connect to another domain
patina connect team-patterns.local

# Now queries search both domains
patina navigate "How to handle caching?"
# Results from: local + team-patterns

# Selective sharing
patina publish pattern auth-flow --to team-patterns
```

### Data Integration Domain
```rust
// In a blockchain game domain
impl BlockchainAdapter for GameData {
    fn fetch(&self) -> Result<RawData> {
        // Pull from chain/Torii
    }
    
    fn transform(&self, data: RawData) -> Result<Vec<Pattern>> {
        // Convert game state to patterns
        // "Player stats show X strategy works"
    }
}
```

## Implementation Strategy

### Phase 1: Single Domain Excellence
- Focus on making one domain work perfectly
- All current Patina development
- No federation complexity

### Phase 2: Domain Templates
- Common domain types as templates
- `patina init --template rust-web-service`
- Pre-configured adapters

### Phase 3: Federation Protocol
- Domain discovery
- Pattern sharing
- Conflict resolution
- Trust networks

## Key Insights

1. **Knowledge Locality**: Most knowledge is domain-specific
2. **Optional Complexity**: Federation only when needed
3. **Tool Agnostic**: Each domain chooses its tools
4. **Privacy First**: Share explicitly, not by default

## Relationship to Indexer

The indexer (GPS) in each domain:
- Navigates local patterns first
- Can query federated domains
- Doesn't process external data (adapters do that)
- Remains simple and focused

## Future Considerations

- **Pattern Markets**: Domains could offer patterns
- **Trust Networks**: Reputation for domain quality
- **Cross-Domain Learning**: ML across federated knowledge
- **Specialized Domains**: Financial, medical, gaming, etc.

This architecture enables Patina to scale from personal tool to global knowledge network while maintaining simplicity at each level.