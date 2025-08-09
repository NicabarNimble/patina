# Dependable Rust: Architecture for Long-Lived Systems

*Building software that lasts decades using Rust's safety with C89's simplicity*

## Executive Summary

This document presents an architectural philosophy for building large-scale, long-lived software systems in Rust. It synthesizes four key insights:

1. **Single-person contract ownership** - Every public API should be small enough for one person to fully understand and maintain
2. **Black-box boundaries** - Small, stable contracts with freedom inside the implementation
3. **Radical simplicity over clever abstractions** - APIs should be so simple they're impossible to misuse
4. **Compile-time guarantees without runtime complexity** - Use Rust's safety without sacrificing debuggability or longevity

We call this approach "Dependable Rust" - a subset of Rust that prioritizes simplicity, longevity, and maintainability over showcasing language features.

## Core Philosophy

### The Fundamental Rule
> "It's faster to write 5 lines of code today than to write 1 line today and edit it in the future."

Software for critical systems must work reliably for 20-50 years. Every clever abstraction, every external dependency, every async function is a future maintenance burden.

### The Four Pillars

#### 1. Single-Person Black-Box Ownership
```rust
// BAD: Large public surface area with no clear owner
// patina/src/adapters/claude.rs (900+ lines, all public)
pub struct Claude { ... }
pub fn init_project(...) { ... }
pub fn generate_context(...) { ... }
pub fn create_session(...) { ... }
// 50+ more public functions...

// GOOD: One dev owns this black box (API + implementation)
// patina/src/adapters/claude/mod.rs - The "header" (like .h in C)
pub trait ClaudeAdapter: Send {
    fn capability(&self) -> Capability;
    fn init_project(&mut self, config: &Config) -> Result<(), Error>;
    fn generate_context(&self) -> Result<Context, Error>;
}

// The trait IS the interface - the owner decides what belongs here
// When someone needs a feature, they ask the owner
// Owner decides: "yes, this fits" or "no, that should be a different box"

// patina/src/adapters/claude/impl_claude.rs (900 lines - private)
// Implementation is private - owner has complete freedom here
```

#### 2. Black Box Abstractions
```rust
// Patina's navigation system - users don't know if it's SQLite, CRDT, or both
pub trait NavigationStore: Send {
    fn capability(&self) -> Capability;
    fn index(&mut self, path: &Path) -> Result<(), NavError>;
    fn search(&self, query: &Query) -> Result<Vec<Match>, NavError>;
    fn sync(&mut self) -> Result<(), NavError>;
}

// Implementation completely hidden - hybrid SQLite + CRDT
mod impl_hybrid {  // Private module
    struct HybridStore {
        sqlite: SqliteBackend,  // Fast local queries
        crdt: AutomergeBackend, // Distributed sync
    }
}

// Public factory function
pub fn create_navigation() -> impl NavigationStore {
    impl_hybrid::HybridStore::new()
}
```

#### 3. Format-First Design
Define your data structures and APIs BEFORE implementation. The format is the contract.

```rust
// Patina's layer pattern format - defined before implementation
#[derive(Serialize, Deserialize)]
pub struct Pattern {
    pub name: String,           // Human-readable identifier
    pub layer: Layer,           // Core, Surface, or Dust
    pub content: String,        // Markdown content
    pub metadata: Metadata,     // Timestamps, oxidizer, etc.
    pub domain: Option<String>, // e.g., "dagger", "testing"
}

#[derive(Serialize, Deserialize)]
pub enum Layer {
    Core,            // Actively used in codebase
    Surface,         // Forming patterns, experiments
    Dust,            // Archived wisdom, deprecated
}

// Implementation comes AFTER format design
```

#### 4. Contract Size Limits
Keep public API surfaces small and comprehensible.

```rust
// The "150 Line Rule" for public contracts
// patina/src/adapters/claude/api.rs
pub trait ClaudeAdapter: Send {            // Line 1
    fn capability(&self) -> Capability;    // Line 10
    fn init_project(...) -> Result<()>;    // Line 20
    fn generate_context(...) -> Result<Context>; // Line 30
}                                           // Line 40

pub struct Capability { ... }              // Lines 50-70
pub struct Context { ... }                 // Lines 80-100
pub enum ClaudeError { ... }              // Lines 110-140

// Total: ~150 lines MAX for public API
// Implementation can be 1000+ lines (private)
```

## The Rust Black-Box Pattern

### Traits Define Boundaries
In Rust, traits are the natural way to define what a black box can do:

```rust
// The trait IS the black box's public interface
pub trait Decoder: Send {
    fn decode(&mut self, data: &[u8]) -> Result<Frame, Error>;
    fn seek(&mut self, timestamp: u64) -> Result<(), Error>;
}

// The module's public items define what's exposed
pub use self::api::{Decoder, Frame, DecoderError};

// Everything else is private implementation detail
mod impl_ffmpeg;  // Hidden
mod cache;        // Hidden
```

### Module Ownership
Each black box module has one owner who:
- Defines the public trait interface
- Guards what belongs in their module
- Has freedom to change private implementation
- Decides feature boundaries

When someone requests functionality:
```rust
// "Can your decoder handle subtitles?"
// Decoder owner: "No, that should be a separate SubtitleExtractor trait"
// Result: Each box stays focused on one responsibility
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

## Black-Box Refactoring Methodology

### The Core Principle
> "You don't have to make files smaller to gain modularity. You have to make surfaces smaller."

Large implementation files are acceptable when hidden behind small, stable trait definitions. This approach enables:
- Single ownership (one black box ↔ one owner)
- Implementation freedom (rewrite internals without breaking callers)
- Better testing (surface tests at the boundary)
- Lower risk over time (fewer call-site changes)

### Black-Box Boundary Template

```rust
// mod.rs - The public interface (~150 lines max)
use thiserror::Error;

/// Public types (minimal fields)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Output { /* minimal public fields */ }

/// Typed errors at the boundary
#[derive(Error, Debug)]
pub enum BoxError {
    #[error("invalid input: {0}")] 
    Invalid(String),
    #[error("operation failed: {0}")] 
    Failed(String),
    #[error(transparent)] 
    Io(#[from] std::io::Error),
}

/// Runtime capability discovery
#[derive(Clone, Debug)]
pub struct Capability {
    pub id: &'static str,         // "patina.navigation"
    pub version: (u16, u16, u16), // semver
    pub features: u64,            // feature flags
}

/// The trait defines what this black box can do
pub trait BlackBox: Send {
    fn capability(&self) -> Capability;
    fn execute(&mut self, input: Input) -> Result<Output, BoxError>;
}

// Hide the implementation
mod impl_primary;  // 1000+ lines, private

// Factory function exposes only the trait
pub fn create() -> impl BlackBox { 
    impl_primary::Implementation::new() 
}
```

### Refactoring Large Files: The Patina Example

```rust
// BEFORE: src/adapters/claude.rs (902 lines, all public)
pub struct Claude { 
    templates: HashMap<String, String>,
    version: String,
    // ... many fields exposed
}

impl Claude {
    pub fn new() -> Self { ... }
    pub fn init_project(...) { ... }
    pub fn generate_context(...) { ... }
    // ... 50+ public methods
}

// AFTER: src/adapters/claude/
// mod.rs (100 lines - only the public trait)
pub trait ClaudeAdapter: Send {
    fn capability(&self) -> Capability;
    fn init_project(&mut self, name: &str) -> Result<(), ClaudeError>;
    fn generate_context(&self) -> Result<Context, ClaudeError>;
}

// impl_claude.rs (900 lines - private, free to change)
struct ClaudeImpl {
    // All implementation details hidden
}

// Factory function in mod.rs
pub fn create_claude() -> impl ClaudeAdapter {
    impl_claude::ClaudeImpl::new()
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

### Pattern 3: Development Environment Abstraction

Patina's approach: abstract development environments behind traits.

```rust
// dev_env/api.rs - Development environment contract
pub trait DevEnvironment: Send {
    fn capability(&self) -> Capability;
    fn build(&mut self, project: &Path) -> Result<(), BuildError>;
    fn test(&mut self, args: &[String]) -> Result<TestResults, BuildError>;
    fn package(&mut self) -> Result<PathBuf, BuildError>;
}

// dev_env/impl_dagger.rs - Dagger implementation (private)
struct DaggerEnv {
    workspace_path: PathBuf,
    config: DaggerConfig,
}

// dev_env/impl_docker.rs - Docker fallback (private)
struct DockerEnv {
    dockerfile: PathBuf,
}

// Smart factory with fallback
pub fn create_dev_env() -> impl DevEnvironment {
    if has_dagger() && has_go() {
        impl_dagger::DaggerEnv::new()
    } else {
        impl_docker::DockerEnv::new()  // Escape hatch
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

### Pattern 6: Async Quarantine with Black-Box Boundaries

When you're forced to use async (file watching, database sync), quarantine it behind traits:

```rust
// monitoring/api.rs - Synchronous public contract
pub trait Monitor: Send {
    fn start(&mut self, path: &Path) -> Result<(), MonitorError>;
    fn poll(&mut self) -> Result<Option<Event>, MonitorError>;
    fn stop(&mut self) -> Result<(), MonitorError>;
}

// monitoring/impl_async.rs - Async quarantined in private implementation
struct AsyncMonitor {
    runtime: tokio::runtime::Runtime,  // Hidden from callers
    rx: Option<Receiver<Event>>,
}

impl Monitor for AsyncMonitor {
    fn poll(&mut self) -> Result<Option<Event>, MonitorError> {
        // Async complexity completely hidden
        self.runtime.block_on(async {
            self.rx.as_mut()
                .ok_or(MonitorError::NotStarted)?
                .recv()
                .await
                .ok_or(MonitorError::Disconnected)
        })
    }
}

// Callers never see async - they just see Monitor trait
let mut monitor = create_monitor();  // Returns impl Monitor
monitor.start(&path)?;
while let Some(event) = monitor.poll()? {
    // Process synchronously
}
```

## Practical Examples

### Example 1: Patina's Layer System - Knowledge That Oxidizes and Evolves

Building a knowledge system that naturally accumulates wisdom like patina on metal.

```rust
// layer/mod.rs - The trait for pattern storage
pub trait LayerSystem: Send {
    fn capability(&self) -> Capability;
    fn capture_raw(&mut self, session: RawSession) -> Result<SessionId, LayerError>;
    fn extract_patterns(&mut self, session_id: SessionId) -> Result<Vec<Pattern>, LayerError>;
    fn promote_to_core(&mut self, pattern: Pattern) -> Result<(), LayerError>;
    fn scrape_to_dust(&mut self, pattern_id: PatternId) -> Result<(), LayerError>;
}

// Pattern format with oxidation lifecycle
#[derive(Serialize, Deserialize, Clone)]
pub struct Pattern {
    pub id: PatternId,
    pub content: String,           // Markdown, always
    pub layer: Layer,              // Core, Surface, or Dust
    pub domain: Option<String>,    // e.g., "dagger", "testing"
    pub metadata: Metadata,
    pub usage_count: u32,          // Track active use
    pub last_accessed: DateTime,   // For oxidation
}

#[derive(Serialize, Deserialize, Clone)]
pub enum Layer {
    Core,           // Active in codebase
    Surface(String), // Forming (with domain)
    Dust,           // Archived wisdom
}

// layer/impl_filesystem.rs - Current implementation (private)
struct FileSystemLayers {
    base_path: PathBuf,
    index: HashMap<PatternId, PatternLocation>,
}

impl LayerSystem for FileSystemLayers {
    fn promote_to_core(&mut self, pattern: Pattern) -> Result<(), LayerError> {
        // Only promote patterns that are actively used
        if pattern.usage_count < 3 {
            return Err(LayerError::NotProven);
        }
        
        // Move from surface to core
        let surface_path = self.base_path.join("surface").join(&pattern.domain);
        let core_path = self.base_path.join("core").join(&pattern.name);
        fs::rename(&surface_path, &core_path)?;
        
        Ok(())
    }
    
    fn scrape_to_dust(&mut self, pattern_id: PatternId) -> Result<(), LayerError> {
        // Move unused patterns to dust
        let pattern = self.get_pattern(pattern_id)?;
        
        // Check if pattern is stale
        let days_unused = (Utc::now() - pattern.last_accessed).num_days();
        if days_unused < 30 {
            return Err(LayerError::StillActive);
        }
        
        // Move to dust for archival
        let current_path = self.pattern_to_path(&pattern);
        let dust_path = self.base_path.join("dust").join(&pattern.name);
        fs::rename(&current_path, &dust_path)?;
        
        Ok(())
    }
}

// Future: dust can blow to database for cross-project wisdom
// But the LayerSystem trait remains stable
```

### Example 2: Patina's Navigation System - Git-Aware Code Intelligence

```rust
// navigation/mod.rs - Git-aware navigation trait
pub trait NavigationSystem: Send {
    fn capability(&self) -> Capability;
    fn index(&mut self, repo: &Path) -> Result<IndexStats, NavError>;
    fn search(&self, query: &SearchQuery) -> Result<Vec<CodeMatch>, NavError>;
    fn get_context(&self, file: &Path, line: u32) -> Result<Context, NavError>;
    fn track_change(&mut self, change: FileChange) -> Result<(), NavError>;
}

// Git-aware code location
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeMatch {
    pub file: PathBuf,
    pub line: u32,
    pub column: u32,
    pub symbol: String,
    pub kind: SymbolKind,
    pub git_status: GitStatus,  // Modified, staged, committed
}

// navigation/impl_hybrid.rs - SQLite + CRDT implementation (private)
struct HybridNavigation {
    sqlite: SqliteBackend,      // Fast local queries
    crdt: Option<AutomergeCrdt>, // Distributed sync when needed
    git_cache: GitStateCache,    // Track git state
}

impl NavigationSystem for HybridNavigation {
    fn index(&mut self, repo: &Path) -> Result<IndexStats, NavError> {
        // Detect git repository
        let git_info = self.git_cache.analyze(repo)?;
        
        // Index based on file count
        let files = self.find_source_files(repo)?;
        
        if files.len() < 1000 {
            // Small repo - sequential indexing
            for file in files {
                self.index_file(&file, &git_info)?;
            }
        } else {
            // Large repo - parallel with Rayon
            use rayon::prelude::*;
            files.par_iter()
                .try_for_each(|file| {
                    self.index_file(file, &git_info)
                })?;
        }
        
        Ok(IndexStats { 
            files_indexed: files.len(),
            symbols_found: self.sqlite.symbol_count()?,
        })
    }
    
    fn track_change(&mut self, change: FileChange) -> Result<(), NavError> {
        // Update SQLite immediately for local performance
        self.sqlite.update_file(&change)?;
        
        // Queue for CRDT sync if distributed
        if let Some(ref mut crdt) = self.crdt {
            crdt.queue_change(change)?;
        }
        
        Ok(())
    }
}

// The implementation complexity (SQLite + CRDT) is hidden
// Callers just see a simple NavigationSystem trait
```

### Example 3: Patina's LLM Adapter System - Future-Proof AI Integration

```rust
// adapters/mod.rs - Stable trait for all LLMs
pub trait LLMAdapter: Send {
    fn capability(&self) -> Capability;
    fn init_project(&mut self, config: ProjectConfig) -> Result<(), AdapterError>;
    fn generate_context(&self) -> Result<ContextFile, AdapterError>;
    fn create_session(&mut self, name: &str) -> Result<SessionId, AdapterError>;
}

// Normalized context format - works across all LLMs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextFile {
    pub format: ContextFormat,  // Claude.md, Gemini.yaml, etc.
    pub sections: Vec<Section>,
    pub metadata: Metadata,
}

// adapters/claude/impl_claude.rs - Claude-specific (private)
struct ClaudeAdapter {
    templates: TemplateEngine,
    session_manager: SessionManager,
    mcp_support: Option<McpConfig>,
}

impl LLMAdapter for ClaudeAdapter {
    fn generate_context(&self) -> Result<ContextFile, AdapterError> {
        // Claude generates CLAUDE.md format
        let mut context = ContextFile {
            format: ContextFormat::ClaudeMarkdown,
            sections: vec![],
            metadata: self.create_metadata(),
        };
        
        // Add Claude-specific sections
        context.sections.push(self.generate_environment_section()?);
        context.sections.push(self.generate_brain_patterns()?);
        context.sections.push(self.generate_session_commands()?);
        
        Ok(context)
    }
}

// adapters/gemini/impl_gemini.rs - Gemini-specific (private)
struct GeminiAdapter {
    config: GeminiConfig,
    formatter: YamlFormatter,
}

// adapters/local/impl_ollama.rs - Local LLM (private)
struct OllamaAdapter {
    model: String,
    context_window: usize,
}

// Factory functions hide implementation choice
pub fn create_adapter(llm_type: &str) -> Result<Box<dyn LLMAdapter>, Error> {
    match llm_type {
        "claude" => Ok(Box::new(ClaudeAdapter::new())),
        "gemini" => Ok(Box::new(GeminiAdapter::new())),
        "ollama" => Ok(Box::new(OllamaAdapter::new())),
        _ => Err(Error::UnknownLLM),
    }
}

// The adapter system ensures Patina works with:
// - Today's LLMs (Claude, GPT, Gemini)
// - Tomorrow's LLMs (unknown)
// - Local models (Ollama, llama.cpp)
// - Future protocols (MCP evolution)
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

### 3. Surface Testing at Trait Boundaries
```rust
#[test]
fn navigation_sync_is_idempotent() {
    // Test only through the public trait
    let mut nav: Box<dyn NavigationSystem> = create_navigation();
    
    nav.index(&test_repo).unwrap();
    let before = nav.search(&query).unwrap();
    
    nav.sync().unwrap();
    let after = nav.search(&query).unwrap();
    
    // Verify invariant: sync doesn't change local state
    assert_eq!(before, after);
}
```

## Migration Strategy: Black-Box Refactoring Path

### From Large Files to Black Boxes (Gradual)
Never rewrite everything. Wrap existing code behind traits first:

```rust
// Step 1: Define the trait (mod.rs)
pub trait NavigationStore: Send {
    fn index(&mut self, path: &Path) -> Result<(), Error>;
    fn search(&self, query: &Query) -> Result<Vec<Match>, Error>;
}

// Step 2: Wrap existing code (impl_legacy.rs)
mod impl_legacy {
    use super::*;
    
    // Your existing 600+ line implementation
    pub struct LegacyNavigation {
        // Existing fields...
    }
    
    impl NavigationStore for LegacyNavigation {
        // Wrap existing methods
        fn index(&mut self, path: &Path) -> Result<(), Error> {
            self.existing_index_method(path) // Just delegate
        }
    }
}

// Step 3: Use trait at call sites
fn process_repo(nav: &mut impl NavigationStore) {
    nav.index(&repo_path)?;
}

// Step 4: Now you can refactor internals freely
// The 600+ lines can stay in one file or be split
// Callers don't care - they use the trait
```

### Applying to Patina's Current Violations

```rust
// Week 1: Define traits for each large module
// patina/src/adapters/claude/mod.rs - Add trait definition
// patina/src/commands/init/mod.rs - Add trait definition
// patina/src/indexer/navigation/mod.rs - Add trait definition

// Week 2: Move implementations behind traits
// Just wrap existing code - no rewriting yet

// Week 3: Refactor internals as needed
// Now you can split, optimize, or rewrite freely
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

1. **Single ownership of black boxes** - One dev owns each module (trait + implementation)
2. **Small trait surfaces, not small files** - Keep public APIs under 150 lines; implementations can be larger
3. **Black-box boundaries** - Hide complexity behind stable traits
4. **Start simple, add parallelism when measured** - Profile before parallelizing
5. **Use Rayon for CPU-intensive work on large datasets** - Not for everything
6. **Quarantine async behind traits** - Never let it infect core logic
7. **Write boring code** - Clever code is hard to maintain
8. **Own your dependencies** - External changes shouldn't break your system
9. **Design formats first** - The data structure is the API
10. **Test at trait boundaries** - Surface tests matter more than unit tests
11. **Build extensive tooling** - Recording, playback, inspection, simulation

Remember: **Modularity comes from small trait surfaces, not small files.**

The Black-Box methodology shows us how to achieve Dependable Rust:
- Each black box has one owner (who owns both trait and implementation)
- Owner guards what belongs in their box vs elsewhere
- Trait definitions in mod.rs serve as the public interface
- Implementation freedom inside, stability outside
- Async/complex dependencies stay hidden in private modules

**Performance comes from using the right tool for each job:** 
- Sequential for most things (default)
- Rayon for heavy CPU work on large datasets
- Thread::scope for I/O operations
- Channels for decoupling components
- Async only when forced, quarantined in black boxes

## Key Takeaways

- **Trait surface size matters, not file size** - 150-line public APIs, any size private implementation
- **One owner per black box** - Single dev owns trait definition and implementation
- **Black-box refactoring enables gradual improvement** - Wrap first, refactor later
- **Default to sequential** - Most code doesn't need parallelism
- **Measure before optimizing** - Use profiling to guide decisions
- **Rayon when appropriate** - Large datasets (>1000 items) with expensive operations (>10ms per item)
- **Threads for I/O** - Network, disk, and database operations
- **Quarantine async in black boxes** - Hide behind traits, never in public APIs
- **Test at trait boundaries** - Surface tests catch real issues
- **Implementation freedom** - Change internals without breaking callers

This is Dependable Rust: small trait surfaces with implementation freedom, pragmatic performance with the simplicity to last decades.

## Resources

- [C89 Standard](https://port70.net/~nsz/c/c89/c89-draft.html) - Understanding simplicity
- [SQLite Architecture](https://www.sqlite.org/arch.html) - Example of long-lived software
- [Plan 9 Design](https://9p.io/sys/doc/9.html) - Simplicity in system design
- [Casey Muratori's Handmade Hero](https://handmadehero.org/) - Building from scratch
- [Jonathan Blow on Software Quality](https://www.youtube.com/watch?v=pW-SOdj4Kkk) - Deep engineering

## Appendix: Black-Box Module Structure for Patina

```
patina/
├── layer/                    # Knowledge accumulation system
│   ├── core/                # Active patterns in use
│   ├── surface/             # Forming patterns & experiments
│   │   ├── raw/            # Unprocessed sessions
│   │   └── */              # Domain-specific patterns
│   └── dust/               # Archived wisdom
├── src/
│   ├── adapters/
│   │   ├── mod.rs           # Public traits for all adapters
│   │   ├── claude/
│   │   │   ├── mod.rs       # ClaudeAdapter trait (100 lines PUBLIC)
│   │   │   └── impl.rs      # Implementation (900 lines PRIVATE)
│   │   └── gemini/
│   │       ├── mod.rs       # GeminiAdapter trait (100 lines PUBLIC)
│   │       └── impl.rs      # Implementation (PRIVATE)
│   ├── layer/
│   │   ├── mod.rs           # LayerSystem trait (150 lines PUBLIC)
│   │   └── impl_fs.rs       # FileSystem implementation (PRIVATE)
│   ├── navigation/
│   │   ├── mod.rs           # NavigationSystem trait (150 lines PUBLIC)
│   │   ├── impl_hybrid.rs   # SQLite+CRDT (600 lines PRIVATE)
│   │   └── impl_simple.rs   # Simple alternative (PRIVATE)
│   └── dev_env/
│       ├── mod.rs           # DevEnvironment trait (100 lines PUBLIC)
│       ├── impl_dagger.rs   # Dagger implementation (PRIVATE)
│       └── impl_docker.rs   # Docker fallback (PRIVATE)
```

**Key Points:**
- Each `mod.rs` defines the public trait (the black box's interface)
- One dev owns each module (trait + implementation)
- Implementation files can be large (hidden complexity)
- Async code quarantined in `impl_*.rs` files
- Public trait surface < 150 lines per module
- Test at the trait boundary (through mod.rs interface)

This structure achieves Dependable Rust through Black-Box boundaries while patterns naturally oxidize through the layer system.