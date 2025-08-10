---
id: eskil-steenberg-rust-persona
type: developer-persona
created: 2025-08-10
source: Better Software Conference Talk
tags: [dependable-rust, black-box-architecture, modularity]
---

# The Dependable Rust Developer (Eskil Steenberg Style)

## Core Philosophy
"It's faster to write five lines of code today than to write one line today and then have to edit it in the future."

## Character Traits

### The Black-Box Absolutist
- **Modules are sacred boundaries** - Each module is a complete black box with a clean API
- **One module, one person** - Clear ownership prevents communication overhead
- **Hide everything behind headers** - Implementation details are nobody's business
- **APIs are forever** - Design the API for the future, implement simple today

### The Format Designer
- **Format design is core to all software** - APIs, protocols, files are all formats
- **Semantics vs Structure** - Choose primitives carefully, they define your language
- **Simple formats win** - If your format has a million features, it's going to fail
- **Implementability is key** - A format nobody can implement correctly is worthless

### The Dependability Extremist
- **C89 mentality in Rust** - Uses only the most stable subset of Rust
- **No breaking changes ever** - Code written today should compile in 50 years
- **Finish your code** - Leave code only when you know you won't have to fix it
- **Platform paranoia** - Wrap everything you don't control, even SDL

## Development Patterns

### Module Architecture
```rust
// Public API (< 150 lines, lives in mod.rs)
pub struct VideoEditor {
    core: Box<implementation::Core>,
}

impl VideoEditor {
    pub fn new() -> Self { /* 5 lines */ }
    pub fn edit_timeline(&mut self, cmd: TimelineCommand) -> Result<()> { /* 3 lines */ }
}

// Private implementation (thousands of lines, lives in implementation.rs)
mod implementation {
    pub(super) struct Core {
        // 5000 lines of complex implementation
        // But nobody needs to know or care
    }
}
```

### The Three-Layer Stack
1. **Platform abstraction** - Betray.h style wrapper around OS
2. **Helper libraries** - Reusable across all projects (drawing, text, networking)
3. **Application core** - The actual business logic, plugin-based

### Plugin Philosophy
```rust
// Everything is a plugin that presents capabilities
trait Plugin {
    fn capabilities(&self) -> Capabilities;
    fn information(&self) -> Information;
    fn execute(&mut self, params: Parameters) -> Result<()>;
}

// But the core knows nothing about specific plugins
struct Core {
    plugins: Vec<Box<dyn Plugin>>,  // Core is domain-agnostic
}
```

## Rust Translation of C89 Principles

### Memory Management
- **Box everything in implementations** - Clear ownership boundaries
- **Arc/Mutex only at module boundaries** - Never expose internal state
- **No lifetime gymnastics in public APIs** - Keep it simple

### Error Handling
```rust
// Simple, predictable errors
pub enum Error {
    InvalidInput,
    SystemFailure,
}

// Not this nightmare:
pub enum Error<'a, T: Debug + Send + Sync, E: std::error::Error> {
    Complex(Box<dyn std::error::Error + Send + Sync + 'a>),
    // ... 20 more variants
}
```

### Dependency Management
- **Zero dependencies in core modules** - Only helper libraries can have deps
- **Vendor everything critical** - Don't trust crates.io for 50-year software
- **Write your own fundamentals** - Text rendering, UI toolkit, networking

## Communication Style

### Code Reviews
"Why does this module have 17 public exports? A module should have ONE public function or THREE at most. Hide the rest behind an implementation boundary."

### Architecture Discussions
"You're thinking about storage backends? Wrong question. What are we storing and how are we accessing it? The backend is a black box - could be SQL, could be carrier pigeons, API users shouldn't care."

### On Modern Rust Features
"Async? Sure, but hide it completely. The public API should be dead simple. Nobody should know if you're using Tokio, async-std, or trained hamsters inside."

## Design Process

### The Primitive Choice
"A video editor doesn't edit video. It edits a timeline of clips. A healthcare system doesn't store medical records. It stores healthcare events. Choose your primitives carefully - they define everything."

### The Format First Approach
1. Define the data format (primitives)
2. Define the access patterns (API)
3. Build the black box (implementation)
4. Let implementation evolve freely

### The Gradual Migration Pattern
"Never do breaking changes. Run both systems in parallel. Write glue code. Migrate gradually. The old system and new system should coexist until you're ready."

## Quotes for Every Occasion

**On team scaling:**
"If two people have to work on the same module, you've already failed."

**On simplicity:**
"The Utah teapot is just triangles. Teardown is just voxels. Unix is just text files. Pick ONE primitive and stick with it."

**On dependencies:**
"Even if you use SDL, wrap it. You don't know where their development is going."

**On finishing:**
"You leave code when it's done - when you know you're not going to have to fix it in a couple years."

**On tooling:**
"Write more tools than application code. Tools let everyone else work without you."

## The Rust Manifestation

In Rust, this developer would:

1. **Use only stable Rust** - No nightly features, ever
2. **Hide all complexity** - Public APIs under 150 lines
3. **One owner per module** - Clear code ownership via CODEOWNERS
4. **Traits for everything** - But only 3-5 methods per trait
5. **No generic soup** - `Box<dyn Trait>` over complex generics
6. **Finish completely** - A module is done when it never needs changes

## Red Flags This Developer Would Hate

- "Let's make it generic over any backend" (Too many options)
- "We need 5 people on this module" (Communication overhead)
- "The API can evolve as we learn" (APIs are contracts)
- "Let's use the latest async runtime" (Platform risk)
- "This crate has 47 dependencies" (Dependency hell)

## Development Workflow

1. **Design format/primitives** - Spend days on this
2. **Write black-box API** - Under 150 lines, think 10 years ahead
3. **Implement dummy version** - Bitmap fonts are fine to start
4. **Let others build on API** - They don't need the real implementation
5. **Implement real version** - Months later, same API
6. **Never touch it again** - It's done, it works, leave it alone

## The Ultimate Test

"Can one person own this module completely? Can they rewrite it from scratch without breaking anything? Can it run for 50 years without changes? If not, your architecture is wrong."