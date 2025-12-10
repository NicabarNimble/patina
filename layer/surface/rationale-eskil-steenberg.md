---
id: rationale-eskil-steenberg
status: active
created: 2025-08-12
updated: 2025-12-09
oxidizer: nicabar
tags: [rationale, rust, philosophy, black-box, eskil-steenberg]
references: [dependable-rust]
---

# Building Software That Lasts Forever - Rust Edition

**Based on Eskil Steenberg's philosophy**: Build modules as black boxes with stable APIs that never change.

---

## Core Philosophy

"It's faster to write 5 lines of code today than to write 1 line today and edit it later."

Your goal: Write code once, have it work forever. Never come back to fix it.

## The Black Box Module

Every module should be:
- **Owned by one person** - they design it, build it, finish it
- **Accessed only through its API** - nobody looks inside
- **Replaceable** - can rewrite the internals without changing the API
- **Finished** - once it works, you never touch it again

## How to Structure a Rust Module

```rust
//! Image processing module - handles all image operations
//! 
//! This module is finished. It will work in 2045 just like it works today.

use std::path::Path;

/// Configuration for image operations
/// This struct is part of our permanent API
#[derive(Clone, Debug)]
pub struct Config {
    pub max_width: u32,
    pub max_height: u32,
    pub quality: f32,
}

/// Main image processor
/// Users never see inside this
pub struct Processor {
    // Private fields - can change anytime
    config: Config,
    cache: Cache,
    engine: Engine,
}

impl Processor {
    /// Create a new processor
    /// This function signature is permanent
    pub fn new(config: Config) -> Result<Self> {
        Ok(Self {
            config,
            cache: Cache::new(),
            engine: Engine::initialize()?,
        })
    }
    
    /// Process an image file
    /// This function signature is permanent
    pub fn process(&mut self, path: &Path) -> Result<ProcessedImage> {
        // Implementation can be completely rewritten
        // API stays the same forever
        self.engine.load(path)?
            .resize(&self.config)
            .apply_filters()
            .into()
    }
}

/// Errors that can occur
/// These are permanent - choose wisely
#[derive(Debug)]
pub enum Error {
    FileNotFound,
    InvalidFormat(String),
    ProcessingFailed(String),
}

// Private implementation - hidden from users
struct Cache { /* ... */ }
struct Engine { /* ... */ }

impl Engine {
    fn initialize() -> Result<Self> {
        // Complex implementation
        // Can be changed anytime
    }
    
    fn load(&mut self, path: &Path) -> Result<LoadedImage> {
        // Nobody outside this module sees this
    }
}
```

## Design Your API for the Future

```rust
pub fn render_text(
    text: &str,
    font: Option<FontId>,    // Optional today, but parameter exists
    size: f32,
    spacing: f32,           // Might do nothing today
    color: Color,
) -> Result<RenderedText> {
    // Today: basic bitmap font
    // Tomorrow: TrueType with kerning
    // API: never changes
}
```

Start simple, keep the same API, improve the implementation later.

## Module Size and Splitting

**One module = One person's mental capacity**

- 100 lines? Great.
- 1,000 lines? Still fine if one person understands it all.
- 10,000 lines? Too much for one person - split it.

Split based on ownership, not arbitrary size limits.

## The Format Philosophy

Your API is a format - a contract for how modules communicate:

- **Keep it small** - fewer functions are easier to implement correctly
- **Make it complete** - cover all use cases, but no more
- **Make it implementable** - if it's too complex, nobody will implement it right

## Testing Your Design

Ask yourself:
1. **Can someone use this module without reading the source?**
2. **If I'm hit by a bus, can someone else rewrite the internals?**
3. **Will this API still make sense in 20 years?**
4. **Have I handled all the edge cases inside the black box?**

## Practical Example: Network Client

```rust
//! A network client that will outlive us all

pub struct Client {
    url: String,
    timeout: Duration,
    // Private implementation details
    inner: ClientImpl,
}

impl Client {
    /// Connect to a service
    /// This signature is now permanent
    pub fn connect(url: &str) -> Result<Self> {
        // Can switch from HTTP to QUIC to whatever
        // API doesn't change
    }
    
    /// Make a request
    /// This signature is now permanent  
    pub fn request(&mut self, data: &Request) -> Result<Response> {
        // Implementation can be completely replaced
        // Users never know or care
    }
}
```

## Common Mistakes to Avoid

1. **Exposing implementation details in your API**
   ```rust
   // Bad: locks you into PostgreSQL
   pub fn execute_sql(&self, query: &str) -> PostgresResult
   
   // Good: hides storage implementation
   pub fn get_user(&self, id: UserId) -> Result<User>
   ```

2. **Making everything public "for flexibility"**
   ```rust
   // Bad: can never change these without breaking users
   pub struct Internal {
       pub cache: HashMap<String, Value>,
       pub connection: TcpStream,
   }
   
   // Good: completely hidden
   struct Internal {
       cache: Cache,
       connection: Connection,
   }
   ```

3. **Changing APIs after release**
   - Once public, it's public forever
   - Deprecation is admission of design failure
   - Get it right the first time

## The Power of Constraints

Constraints in your module guarantee behavior:
- "This never returns negative numbers"
- "This always returns valid UTF-8"
- "This never takes longer than the timeout"

Users can rely on these. The black box ensures them.

## Summary

Write modules that:
1. **Have one owner** who understands everything inside
2. **Hide all implementation** behind a simple API
3. **Never change their API** once released
4. **Can be rewritten internally** without users knowing
5. **Work forever** without maintenance

This is how you build software that lasts 50 years.

Remember: The best code is code you never have to change.