# Dependable Rust: Architecture for Long-Lived Systems

*Building software that lasts decades using Rust's safety with C89's simplicity*

## Executive Summary

This document presents an architectural philosophy for building large-scale, long-lived software systems in Rust. It synthesizes three key insights:

1. **Single-person module ownership** - Every module should be small enough for one person to fully understand and maintain
2. **Radical simplicity over clever abstractions** - APIs should be so simple they're impossible to misuse
3. **Compile-time guarantees without runtime complexity** - Use Rust's safety without sacrificing debuggability or longevity

We call this approach "Dependable Rust" - a subset of Rust that prioritizes simplicity, longevity, and maintainability over showcasing language features.

## Core Philosophy

### The Fundamental Rule
> "It's faster to write 5 lines of code today than to write 1 line today and edit it in the future."

Software for critical systems must work reliably for 20-50 years. Every clever abstraction, every external dependency, every async function is a future maintenance burden.

### The Three Pillars

#### 1. Single-Person Module Ownership
```rust
// BAD: Monolithic crate requiring multiple maintainers
// video-editor/src/lib.rs (10,000+ lines)
pub mod decoder;     // Alice maintains
pub mod encoder;     // Bob maintains  
pub mod timeline;    // Charlie maintains
pub mod effects;     // Diana maintains

// GOOD: Separate crates with single owners
// timeline-core/src/lib.rs (500 lines)
// One person can understand EVERYTHING in this crate
pub struct Timeline { ... }
pub fn add_clip(timeline: &mut Timeline, clip: Clip) -> Result<ClipId, Error> { ... }
pub fn render_frame(timeline: &Timeline, time: f64) -> Frame { ... }
```

#### 2. Black Box Abstractions
```rust
// Users don't need to know storage implementation
pub trait EventStore {
    fn store_event(&self, event: Event) -> Result<EventId, Error>;
    fn get_events(&self, filter: Filter) -> Result<Vec<Event>, Error>;
}

// Implementation completely hidden - could be SQL, files, memory
struct PostgresEventStore { ... }  // Private
struct FileEventStore { ... }      // Private
struct InMemoryEventStore { ... }  // Private

// Public factory function
pub fn create_event_store(config: &Config) -> Box<dyn EventStore> {
    // Implementation choice hidden from users
}
```

#### 3. Format-First Design
Define your data structures and APIs BEFORE implementation. The format is the contract.

```rust
// Define the format first
#[repr(C)]  // C-compatible for FFI
pub struct GameEvent {
    pub timestamp: i64,        // Always Unix timestamp UTC
    pub event_id: [u8; 32],    // SHA256 hash
    pub event_type: u32,       // Simple enum as integer
    pub actor_id: [u8; 32],    // UUID as bytes
    pub data_length: u32,      // Length of variable data
    // Variable data follows
}

// Implementation comes AFTER format design
```

## The "Dependable Rust" Subset

### Use These Features
✅ **Rayon for parallelism** - Default for processing collections
✅ **Simple structs and enums** - Data modeling basics
✅ **Functions over methods** - When methods aren't needed
✅ **Result and Option** - Explicit error handling
✅ **Thread::scope** - For bounded I/O operations
✅ **Channels** - For producer-consumer patterns
✅ **Fixed-size arrays** - When size is known
✅ **Standard collections** - Vec, HashMap, but sparingly
✅ **Modules** - For code organization
✅ **#[repr(C)]** - For FFI compatibility
✅ **Tests** - Lots of simple tests

### Avoid These Features
❌ **async/await** - Use Rayon or threads instead
❌ **Complex generics** - Write concrete types
❌ **Trait objects (dyn)** - Unless absolutely necessary
❌ **Procedural macros** - Too magical
❌ **External dependencies** - Vendor or rewrite (except Rayon)
❌ **Lifetime annotations** - Beyond the basics
❌ **Type-level programming** - Keep it simple

### Performance Through Pragmatic Choices
We achieve performance by using the right tool for each job:

```rust
// SEQUENTIAL (default): Simple operations, small datasets
pub fn get_user_names(users: &[User]) -> Vec<String> {
    users.iter()
        .map(|u| u.name.clone())
        .collect()
}

// RAYON: CPU-intensive work on large datasets  
pub fn compute_embeddings(documents: &[Document]) -> Vec<Embedding> {
    if documents.len() < 100 {
        // Small batch - sequential is fine
        documents.iter().map(generate_embedding).collect()
    } else {
        // Large batch - parallel makes sense
        use rayon::prelude::*;
        documents.par_iter().map(generate_embedding).collect()
    }
}

// THREADS: I/O operations
pub fn fetch_resources(urls: &[Url]) -> Vec<Resource> {
    thread::scope(|s| {
        urls.iter()
            .map(|url| s.spawn(|| fetch_blocking(url)))
            .collect::<Vec<_>>()
            .into_iter()
            .map(|h| h.join().unwrap())
            .collect()
    })
}
```

## Architecture Patterns

### Pattern 1: Start Simple, Add Parallelism When Needed

**Default to sequential code.** Profile first, parallelize second.

```rust
// Start with simple, correct code
pub fn process_events(events: &[Event]) -> Summary {
    events.iter()
        .filter(|e| e.is_relevant())
        .map(|e| e.extract_data())
        .fold(Summary::new(), |sum, data| sum.add(data))
}

// After profiling shows this is a bottleneck AND events.len() > 1000:
pub fn process_events_parallel(events: &[Event]) -> Summary {
    use rayon::prelude::*;
    events.par_iter()
        .filter(|e| e.is_relevant())
        .map(|e| e.extract_data())
        .reduce(Summary::new, |a, b| a.merge(b))
}
```

### When to Use Each Tool:

| Tool | Use Case | Example |
|------|----------|---------|
| **Sequential** | Default, small datasets, simple ops | `users.iter().map(...)` |
| **Rayon** | CPU-intensive, large datasets (>1000 items) | `documents.par_iter().map(embed)` |
| **Threads** | I/O operations, blocking calls | `thread::scope(\|s\| s.spawn(...))` |
| **Channels** | Producer-consumer, pipelines | `channel()` for decoupling |

### Pattern 2: Thread::scope for Bounded I/O

For I/O operations where you need parallelism with clear lifetimes:

```rust
pub fn fetch_multiple_resources(urls: &[Url]) -> Vec<Resource> {
    thread::scope(|s| {
        urls.iter()
            .map(|url| s.spawn(|| fetch_blocking(url)))
            .collect::<Vec<_>>()
            .into_iter()
            .map(|h| h.join().unwrap())
            .collect()
    })
}
```

### Pattern 3: Platform Layer Abstraction

Every application needs a platform layer. Own yours.

```rust
// platform/src/lib.rs - Your window/input abstraction
pub struct Window {
    #[cfg(target_os = "windows")]
    handle: *mut std::ffi::c_void,
    #[cfg(target_os = "linux")]
    handle: u64,
    #[cfg(target_os = "macos")]
    handle: *mut std::ffi::c_void,
}

impl Window {
    pub fn create(width: u32, height: u32, title: &str) -> Option<Window> {
        #[cfg(target_os = "windows")]
        return windows::create_window(width, height, title);
        #[cfg(target_os = "linux")]
        return linux::create_window(width, height, title);
        #[cfg(target_os = "macos")]
        return macos::create_window(width, height, title);
    }
    
    pub fn poll_event(&mut self) -> Option<Event> {
        // Platform-specific implementation
    }
}

// Write a minimal test app FIRST
// platform-test/src/main.rs
fn main() {
    let mut window = Window::create(800, 600, "Test").unwrap();
    loop {
        match window.poll_event() {
            Some(Event::Closed) => break,
            Some(Event::MouseMove(x, y)) => println!("Mouse: {},{}", x, y),
            _ => {}
        }
    }
}
```

### Pattern 4: Simple Plugin Architecture

Plugins should be so simple that anyone can write one correctly. **Performance comes from parallel execution, not complex APIs.**

```rust
// Simple C-compatible plugin interface
#[repr(C)]
pub struct PluginVTable {
    pub name: [u8; 64],
    pub version: u32,
    pub process: extern "C" fn(*const Data, *mut Data) -> i32,
}

// Host processes plugins in parallel using Rayon
pub fn run_plugins(plugins: &[Plugin], data: Vec<Data>) -> Vec<Data> {
    use rayon::prelude::*;
    
    data.par_iter()
        .map(|d| {
            let mut result = d.clone();
            for plugin in plugins {
                plugin.process(d, &mut result);
            }
            result
        })
        .collect()
}
```

### Pattern 5: Core Without UI

Separate your business logic from presentation completely. **Use Rayon for processing, not async for fake concurrency.**

```rust
// timeline-core/src/lib.rs - No UI dependencies
use rayon::prelude::*;

pub struct Timeline {
    clips: Vec<Clip>,
    sample_rate: u32,
}

impl Timeline {
    // Parallel rendering by default
    pub fn render_region(&self, start: f64, end: f64, fps: f32) -> Vec<Frame> {
        let frame_count = ((end - start) * fps) as usize;
        
        (0..frame_count)
            .into_par_iter()  // Parallel by default!
            .map(|i| {
                let time = start + (i as f64 / fps as f64);
                self.render_frame_at_time(time)
            })
            .collect()
    }
    
    pub fn analyze_clips(&self) -> ClipStatistics {
        self.clips
            .par_iter()  // Parallel analysis
            .map(|clip| clip.compute_statistics())
            .reduce(ClipStatistics::default, |a, b| a.merge(b))
    }
}
```

### Pattern 6: Async Quarantine

When you're forced to use async (WebSockets, certain APIs), quarantine it completely:

```rust
// Keep async at the absolute boundary
pub struct ApiClient {
    runtime: tokio::runtime::Runtime,
}

impl ApiClient {
    // Public API is synchronous
    pub fn fetch_batch(&self, urls: &[Url]) -> Vec<Result<Data, Error>> {
        // If we have many URLs, use Rayon for parallel fetching
        use rayon::prelude::*;
        
        urls.par_iter()
            .map(|url| self.fetch_one(url))
            .collect()
    }
    
    fn fetch_one(&self, url: &Url) -> Result<Data, Error> {
        // Async hidden inside
        self.runtime.block_on(async {
            reqwest::get(url).await?.json().await
        })
    }
}
```

## Practical Examples

### Example 1: Immortal Game World

Building a game that survives even if the company dies - players own their world.

```rust
// Game events as the primitive - deterministic and verifiable
#[repr(C)]
pub struct GameEvent {
    pub event_hash: [u8; 32],      // SHA256 of event
    pub previous: [u8; 32],        // Chain of events
    pub actor_id: [u8; 32],        // Player who did this
    pub action_type: u32,          // Move, Attack, Trade, etc.
    pub action_data: Vec<u8>,      // Serialized action details
    pub timestamp: i64,            // When it happened
    pub signature: [u8; 64],       // Cryptographic proof
}

// Simple game state management - storage agnostic
pub struct GameWorld {
    state: WorldState,
    pending_events: VecDeque<GameEvent>,
}

impl GameWorld {
    // Process events deterministically - same result everywhere
    pub fn process_event(&mut self, event: GameEvent) -> Result<(), Error> {
        // Verify signature
        if !verify_signature(&event) {
            return Err(Error::InvalidSignature);
        }
        
        // Apply deterministic game rules
        match event.action_type {
            ACTION_MOVE => self.process_move(&event)?,
            ACTION_ATTACK => self.process_attack(&event)?,
            ACTION_TRADE => self.process_trade(&event)?,
            _ => return Err(Error::UnknownAction),
        }
        
        Ok(())
    }
    
    // Sync with other nodes - network agnostic
    pub fn sync_events(&mut self, peer_events: Vec<GameEvent>) {
        // Simple: Apply events we haven't seen
        // Complex: Conflict resolution, consensus
        // But the API stays the same
        
        let new_events = peer_events.into_iter()
            .filter(|e| !self.has_event(&e.event_hash))
            .collect::<Vec<_>>();
            
        // Process in timestamp order
        for event in new_events {
            self.pending_events.push_back(event);
        }
    }
    
    // Only parallelize when it makes sense
    pub fn validate_events(&self, events: &[GameEvent]) -> Vec<bool> {
        if events.len() < 100 {
            // Small batch - sequential is fine
            events.iter().map(|e| verify_signature(e)).collect()
        } else {
            // Large batch - parallel validation
            use rayon::prelude::*;
            events.par_iter().map(|e| verify_signature(e)).collect()
        }
    }
}

// Storage backend - completely hidden
trait StorageBackend {
    fn store_event(&mut self, event: &GameEvent) -> Result<(), Error>;
    fn get_events_since(&self, timestamp: i64) -> Vec<GameEvent>;
}

// Can implement for PostgreSQL, SQLite, Blockchain, IPFS, etc.
// Game logic doesn't change when backend changes
```

### Example 2: Unkillable Knowledge Network

A Wikipedia that can't be burned - knowledge that survives censorship.

```rust
// Knowledge fragments as content-addressed primitives
#[repr(C)]
pub struct KnowledgeFragment {
    pub content_hash: [u8; 32],     // IPFS-style addressing
    pub parent_hashes: Vec<[u8; 32]>, // Builds on previous knowledge
    pub author_id: [u8; 32],        // Cryptographic identity
    pub topic_path: Vec<u32>,       // Hierarchical categorization
    pub language: u32,              // ISO language code
    pub timestamp: i64,
    pub endorsements: Vec<[u8; 64]>, // Community validation
}

// Knowledge store - works offline or online
pub struct KnowledgeBase {
    fragments: HashMap<[u8; 32], KnowledgeFragment>,
    index: TopicIndex,
}

impl KnowledgeBase {
    // Add knowledge - automatically content-addressed
    pub fn add_fragment(&mut self, content: &str, metadata: Metadata) -> [u8; 32] {
        let hash = sha256(content.as_bytes());
        
        let fragment = KnowledgeFragment {
            content_hash: hash,
            parent_hashes: metadata.references.clone(),
            author_id: metadata.author,
            topic_path: metadata.topics,
            language: metadata.language,
            timestamp: current_timestamp(),
            endorsements: vec![],
        };
        
        self.fragments.insert(hash, fragment);
        self.index.add(hash, &metadata.topics);
        
        hash
    }
    
    // Query knowledge - works even with partial data
    pub fn search(&self, query: &Query) -> Vec<KnowledgeFragment> {
        let candidates = self.index.search(&query.topics);
        
        candidates.into_iter()
            .filter_map(|hash| self.fragments.get(&hash))
            .filter(|f| f.language == query.language)
            .filter(|f| f.endorsements.len() >= query.min_endorsements)
            .take(query.limit)
            .cloned()
            .collect()
    }
    
    // Sync with peers - protocol agnostic
    pub fn sync_with_peer(&mut self, peer: &mut dyn KnowledgePeer) {
        // Exchange what we have
        let our_hashes = self.fragments.keys().cloned().collect();
        let their_hashes = peer.list_fragments();
        
        // Get what we're missing
        let missing: Vec<_> = their_hashes.difference(&our_hashes).collect();
        
        if missing.len() < 100 {
            // Few fragments - sequential fetch
            for hash in missing {
                if let Some(fragment) = peer.get_fragment(hash) {
                    // Verify content matches hash
                    if sha256(&fragment.content) == *hash {
                        self.fragments.insert(*hash, fragment);
                    }
                }
            }
        } else {
            // Many fragments - parallel fetch
            use rayon::prelude::*;
            let new_fragments: Vec<_> = missing.par_iter()
                .filter_map(|hash| {
                    peer.get_fragment(hash)
                        .filter(|f| sha256(&f.content) == **hash)
                        .map(|f| (*hash, f))
                })
                .collect();
                
            for (hash, fragment) in new_fragments {
                self.fragments.insert(hash, fragment);
            }
        }
    }
}

// Peer interface - can be HTTP, IPFS, Bluetooth, Sneakernet
trait KnowledgePeer {
    fn list_fragments(&self) -> HashSet<[u8; 32]>;
    fn get_fragment(&self, hash: &[u8; 32]) -> Option<KnowledgeFragment>;
}
```

### Example 3: Unstoppable Market Observer

See all markets forever - financial transparency that can't be hidden.

```rust
// Normalized market events across all chains
#[repr(C)]
pub struct MarketEvent {
    pub event_id: [u8; 32],         // Universal unique ID
    pub source_chain: u32,          // ETH, BTC, SOL, etc.
    pub source_proof: Vec<u8>,      // Chain-specific proof
    pub event_type: u32,            // Swap, Transfer, Mint, etc.
    pub actor: [u8; 32],            // Normalized address
    pub asset_in: AssetAmount,      // What went in
    pub asset_out: AssetAmount,     // What came out
    pub timestamp: i64,
    pub observer_sig: [u8; 64],     // Who witnessed this
}

#[repr(C)]
pub struct AssetAmount {
    pub asset_id: [u8; 32],         // Normalized asset identifier
    pub amount: u128,               // Amount in base units
}

// Market observer - chain agnostic
pub struct MarketObserver {
    events: VecDeque<MarketEvent>,
    validators: Vec<ValidatorNode>,
    max_events: usize,
}

impl MarketObserver {
    // Ingest from any chain - adapters handle specifics
    pub fn ingest_event(&mut self, raw_event: &[u8], chain: u32) -> Result<(), Error> {
        let adapter = get_chain_adapter(chain)?;
        let event = adapter.parse_event(raw_event)?;
        
        // Normalize to our format
        let market_event = MarketEvent {
            event_id: sha256(&[&event.tx_hash, &event.log_index.to_le_bytes()].concat()),
            source_chain: chain,
            source_proof: raw_event.to_vec(),
            event_type: classify_event(&event),
            actor: normalize_address(&event.from, chain),
            asset_in: normalize_asset(&event.input, chain),
            asset_out: normalize_asset(&event.output, chain),
            timestamp: event.block_timestamp,
            observer_sig: sign_observation(&event),
        };
        
        self.events.push_back(market_event);
        if self.events.len() > self.max_events {
            self.events.pop_front();
        }
        
        Ok(())
    }
    
    // Query across all chains with simple API
    pub fn query_activity(&self, filter: &Filter) -> Vec<MarketEvent> {
        self.events.iter()
            .filter(|e| filter.chains.is_empty() || filter.chains.contains(&e.source_chain))
            .filter(|e| filter.actors.is_empty() || filter.actors.contains(&e.actor))
            .filter(|e| e.timestamp >= filter.from_time)
            .filter(|e| e.timestamp <= filter.to_time)
            .cloned()
            .collect()
    }
    
    // Validate with peers - consensus without blockchain
    pub fn validate_observations(&mut self) -> Vec<MarketEvent> {
        let mut validated = Vec::new();
        
        for event in &self.events {
            let confirmations = self.validators.iter()
                .filter(|v| v.has_observed(&event.event_id))
                .count();
                
            // Simple majority consensus
            if confirmations > self.validators.len() / 2 {
                validated.push(event.clone());
            }
        }
        
        validated
    }
    
    // Analyze with parallelism when appropriate
    pub fn analyze_volume(&self, window: Duration) -> TotalVolume {
        let cutoff = current_timestamp() - window.as_secs() as i64;
        let events: Vec<_> = self.events.iter()
            .filter(|e| e.timestamp > cutoff)
            .cloned()
            .collect();
        
        // Choose approach based on data size
        let total = if events.len() > 1000 {
            // Large dataset - parallelize
            use rayon::prelude::*;
            events.par_iter()
                .filter(|e| e.event_type == EVENT_SWAP)
                .map(|e| extract_volume(e))
                .sum()
        } else {
            // Small dataset - sequential is fine
            events.iter()
                .filter(|e| e.event_type == EVENT_SWAP)
                .map(|e| extract_volume(e))
                .sum()
        };
        
        TotalVolume { amount: total, event_count: events.len() }
    }
}

// Chain adapter interface - new chains without changing core
trait ChainAdapter {
    fn parse_event(&self, raw: &[u8]) -> Result<ChainEvent, Error>;
    fn verify_proof(&self, event: &MarketEvent) -> bool;
}

// Can observe Ethereum, StarkNet, Solana, even CEX APIs
// Core logic doesn't care about chain specifics
```

## Testing Strategy

### 1. Write Test Apps First
Before building your platform layer, write the simplest possible test:

```rust
// platform-test/src/main.rs
fn main() {
    println!("Opening window...");
    let window = Window::create(800, 600, "Test");
    println!("Window created: {:?}", window.is_some());
    
    if let Some(mut w) = window {
        println!("Polling events...");
        for _ in 0..100 {
            if let Some(event) = w.poll_event() {
                println!("Event: {:?}", event);
            }
            std::thread::sleep(Duration::from_millis(16));
        }
    }
}
```

### 2. Test Data Flow with Recording/Playback
```rust
pub struct EventRecorder {
    events: Vec<(Instant, Event)>,
}

impl EventRecorder {
    pub fn record(&mut self, event: Event) {
        self.events.push((Instant::now(), event));
    }
    
    pub fn save(&self, path: &str) -> Result<(), Error> {
        let file = File::create(path)?;
        bincode::serialize_into(file, &self.events)?;
        Ok(())
    }
    
    pub fn playback(path: &str) -> Result<EventPlayer, Error> {
        let file = File::open(path)?;
        let events = bincode::deserialize_from(file)?;
        Ok(EventPlayer { events, index: 0 })
    }
}
```

### 3. Property-Based Testing for Invariants
```rust
#[test]
fn game_events_maintain_consistency() {
    let mut world = GameWorld::new();
    
    // Add random events
    for _ in 0..1000 {
        let event = generate_random_event();
        let _ = world.process_event(event);
    }
    
    // Check invariants
    assert!(world.validate_state());
    assert!(world.all_players_valid());
    assert!(world.no_duplicate_events());
}
```

## Migration Strategy

### From Existing Systems
Never do a "big bang" migration. Run both systems in parallel:

```rust
pub struct MigrationBridge {
    old_system: OldSystemAPI,
    new_system: NewSystemAPI,
}

impl MigrationBridge {
    pub fn sync(&mut self) -> Result<(), Error> {
        // Copy new events to old system
        for event in self.new_system.get_unsynced()? {
            self.old_system.store(convert_to_old(event))?;
        }
        
        // Copy old events to new system
        for event in self.old_system.get_unsynced()? {
            self.new_system.store(convert_to_new(event))?;
        }
        
        Ok(())
    }
}
```

## Tooling Requirements

Every core system needs:

1. **Recorder** - Capture all events/state changes
2. **Playback** - Replay recorded sessions
3. **Inspector** - View current state
4. **Simulator** - Generate test data
5. **Validator** - Check invariants
6. **Converter** - Import/export different formats

```rust
// Example: Game State Inspector
pub fn inspect_game_state(world: &GameWorld) {
    println!("World: {} players, {} pending events", 
             world.player_count(), 
             world.pending_events.len());
    
    for player in world.active_players() {
        println!("  Player {}: Level {}, Position ({}, {})", 
                 player.id, 
                 player.level,
                 player.x, 
                 player.y);
    }
}
```

## Performance Without Complexity

Dependable Rust achieves performance through choosing the right tool for each job, not by defaulting to complexity.

### Decision Guide

```
What type of work is it?
├─ Simple operations (<100 items or <1ms per item)
│  └─ Use sequential iterators (default)
├─ CPU-intensive work (>10ms per item AND >1000 items)
│  └─ Use Rayon .par_iter()
├─ I/O operations (network, disk, databases)
│  └─ Use thread::scope or thread pools
├─ Producer-consumer patterns
│  └─ Use channels for decoupling
└─ Forced async (WebSockets, specific SDKs)
   └─ Quarantine with runtime.block_on()
```

### The Tools

#### 1. Sequential (Default)
```rust
// Most code should be simple sequential
let results: Vec<_> = items.iter()
    .filter(|x| x.is_valid())
    .map(|x| x.process())
    .collect();
```

#### 2. Rayon (Heavy CPU Work)
```rust
use rayon::prelude::*;

// Only when you have real CPU work on large datasets
if data.len() > 1000 && per_item_cost_ms > 10 {
    data.par_iter().map(expensive_computation).collect()
} else {
    data.iter().map(expensive_computation).collect()
}
```

#### 3. Threads (I/O Operations)
```rust
// For blocking I/O operations
thread::scope(|s| {
    urls.iter()
        .map(|url| s.spawn(|| fetch_blocking(url)))
        .collect::<Vec<_>>()
        .into_iter()
        .map(|h| h.join().unwrap())
        .collect()
})
```

#### 4. Channels (Decoupling)
```rust
let (tx, rx) = channel();

thread::spawn(move || {
    while let Some(work) = get_work() {
        tx.send(process(work)).unwrap();
    }
});

thread::spawn(move || {
    while let Ok(result) = rx.recv() {
        save_result(result);
    }
});
```

### Why Rayon When Appropriate

- **Stable for a decade** - Version 1.0 since 2017
- **Simple mental model** - Just parallel iterators
- **No lifetime issues** - Unlike async's 'static requirements
- **Work-stealing** - Automatic load balancing
- **Easy to remove** - Just delete `.par_` to go back to sequential

## Common Pitfalls to Avoid

### 1. Over-Parallelizing
```rust
// BAD: Parallelizing trivial operations
fn get_names(users: &[User]) -> Vec<String> {
    use rayon::prelude::*;
    users.par_iter()  // Overhead > benefit for simple string cloning
        .map(|u| u.name.clone())
        .collect()
}

// GOOD: Keep simple things simple
fn get_names(users: &[User]) -> Vec<String> {
    users.iter()
        .map(|u| u.name.clone())
        .collect()
}
```

### 2. The Async Infection
```rust
// BAD: Async spreads through entire codebase
async fn get_user(id: u64) -> User { ... }
async fn get_permissions(user: &User) -> Permissions { ... }
async fn check_access(user: User) -> bool { ... }

// GOOD: Keep core logic synchronous
fn get_user(id: u64) -> User { ... }
fn get_permissions(user: &User) -> Permissions { ... }

// Only use parallelism where it helps
fn process_many_users(ids: &[u64]) -> Vec<UserData> {
    if ids.len() > 100 {
        use rayon::prelude::*;
        ids.par_iter().map(|id| {
            let user = get_user(*id);
            let perms = get_permissions(&user);
            UserData { user, perms }
        }).collect()
    } else {
        ids.iter().map(|id| {
            let user = get_user(*id);
            let perms = get_permissions(&user);
            UserData { user, perms }
        }).collect()
    }
}
```

### 3. Premature Optimization
```rust
// BAD: Optimizing before measuring
impl GameState {
    fn update_player(&mut self, id: PlayerId, action: Action) {
        use rayon::prelude::*;
        // Why parallelize updating a single player?
        self.players.par_iter_mut()
            .filter(|p| p.id == id)
            .for_each(|p| p.apply_action(action));
    }
}

// GOOD: Start simple, measure, then optimize
impl GameState {
    fn update_player(&mut self, id: PlayerId, action: Action) {
        if let Some(player) = self.players.get_mut(&id) {
            player.apply_action(action);
        }
    }
}
```

### 4. External Dependency Sprawl
```rust
// BAD: 47 dependencies in Cargo.toml
[dependencies]
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
reqwest = "0.11"
diesel = "2.0"
# ... 43 more

// GOOD: Minimal, vendored dependencies
[dependencies]
# Core functionality only
serde = "1.0"     # Serialization
rayon = "1.0"     # Parallelism when measured
# Vendor or rewrite everything else
```

## Conclusion

Building software that lasts decades requires pragmatic choices. The principles are:

1. **Start simple, add parallelism when measured** - Profile before parallelizing
2. **Use Rayon for CPU-intensive work on large datasets** - Not for everything
3. **Write boring code** - Clever code is hard to maintain
4. **Own your dependencies** - External changes shouldn't break your system
5. **Design formats first** - The data structure is the API
6. **Keep modules small** - One person should understand everything
7. **Test with minimal examples** - If a minimal test is hard, the API is too complex
8. **Build extensive tooling** - Recording, playback, inspection, simulation

Remember: **Performance comes from using the right tool for each job.** 
- Sequential for most things (default)
- Rayon for heavy CPU work on large datasets
- Thread::scope for I/O operations
- Channels for decoupling components
- Async only when forced, quarantined at boundaries

## Key Takeaways

- **Default to sequential** - Most code doesn't need parallelism
- **Measure before optimizing** - Use profiling to guide decisions
- **Rayon when appropriate** - Large datasets (>1000 items) with expensive operations (>10ms per item)
- **Threads for I/O** - Network, disk, and database operations
- **Quarantine async** - Only at system boundaries, never in core logic
- **Content-addressed data** - Makes systems unkillable and verifiable
- **Storage agnostic** - Can move from SQL to blockchain without changing logic

This is Dependable Rust: pragmatic performance with the simplicity to last decades.

## Resources

- [C89 Standard](https://port70.net/~nsz/c/c89/c89-draft.html) - Understanding simplicity
- [SQLite Architecture](https://www.sqlite.org/arch.html) - Example of long-lived software
- [Plan 9 Design](https://9p.io/sys/doc/9.html) - Simplicity in system design
- [Casey Muratori's Handmade Hero](https://handmadehero.org/) - Building from scratch
- [Jonathan Blow on Software Quality](https://www.youtube.com/watch?v=pW-SOdj4Kkk) - Deep engineering

## Appendix: Example Module Structure

```
project/
├── core/               # No external dependencies
│   ├── Cargo.toml     # [dependencies] is empty
│   └── src/
│       └── lib.rs     # Pure business logic
├── platform/          # Platform abstraction
│   ├── Cargo.toml     # Only libc
│   └── src/
│       └── lib.rs     # Window, input, etc.
├── plugins/           # Optional plugins
│   ├── plugin-api/    # C-compatible API
│   └── effects/       # Individual effect plugins
├── tools/             # Development tools
│   ├── recorder/      # Record system state
│   ├── player/        # Playback recordings
│   └── inspector/     # Inspect state
└── app/               # Final application
    ├── cli/           # Command-line interface
    └── gui/           # Graphical interface
```

Each directory is owned by one person. Each can be understood completely by that person. The system can run with any subset of plugins. Tools are built before the app. Platform works on all targets.

This is Dependable Rust.