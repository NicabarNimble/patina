---
id: adapter-pattern
layer: core
status: active
created: 2025-08-02
tags: [architecture, patterns, adapters, traits, external-systems]
references: [dependable-rust, unix-philosophy]
---

# Adapter Pattern

**Purpose:** Use trait-based adapters to remain agnostic to external systems while providing rich, system-specific integrations.

---

## Core Principle

Patina uses trait-based adapters to integrate with external systems (LLMs, databases, build tools) without coupling core logic to any specific implementation. Each adapter implements a common trait, enabling runtime selection and easy testing with mocks.

## When to Use

Apply this pattern when:
- Integrating with external systems (LLMs, APIs, databases)
- Multiple implementations exist for the same behavior
- You want to remain agnostic to vendor/tool choice
- Testing requires mock implementations

**Common use cases in Patina:**
- LLM adapters (Claude, Gemini, future providers)
- Build system adapters (Docker, Dagger, native)
- Storage backends (SQLite, PostgreSQL, in-memory)

## How to Apply

### 1. Define the Trait Contract

Core defines the contract - what all adapters must provide:

```rust
// In core (not adapter-specific code)
pub trait LLMAdapter {
    /// Adapter name for user-facing messages
    fn name(&self) -> &'static str;

    /// Initialize project with adapter-specific setup
    fn init_project(
        &self,
        project_path: &Path,
        design: &Value,
        environment: &Environment,
    ) -> Result<()>;

    /// Generate context document for this LLM
    fn generate_context(
        &self,
        patterns: &[Pattern],
        environment: &Environment,
    ) -> Result<String>;
}
```

**Trait design principles:**
- Keep interface minimal (3-7 methods typical)
- Return `Result<T>` for fallible operations
- Accept borrowed data (`&Path`, `&[Pattern]`) when possible
- Use domain types, not adapter-specific types

### 2. Implement Adapter-Specific Logic

Each adapter implements the trait with vendor-specific details:

```rust
// In src/adapters/claude/mod.rs
pub struct ClaudeAdapter {
    version: String,
    template_dir: PathBuf,
}

impl LLMAdapter for ClaudeAdapter {
    fn name(&self) -> &'static str {
        "claude"
    }

    fn init_project(
        &self,
        project_path: &Path,
        design: &Value,
        environment: &Environment,
    ) -> Result<()> {
        // Claude-specific: .claude/ directory structure
        self.create_claude_dir(project_path)?;
        self.copy_templates(project_path)?;
        self.generate_claude_md(design, environment)?;
        Ok(())
    }

    fn generate_context(
        &self,
        patterns: &[Pattern],
        environment: &Environment,
    ) -> Result<String> {
        // Claude-specific: format for CLAUDE.md
        let mut ctx = String::new();
        ctx.push_str("# Project Patterns\n\n");
        for pattern in patterns {
            ctx.push_str(&self.format_pattern_claude(pattern));
        }
        Ok(ctx)
    }
}
```

### 3. Use Traits, Not Concrete Types

Commands use trait objects, never concrete adapter types:

```rust
// ❌ Bad: coupled to Claude
pub fn init(path: &Path, adapter: ClaudeAdapter) -> Result<()> {
    adapter.init_project(path, &design, &env)?;
}

// ✅ Good: works with any adapter
pub fn init(path: &Path, adapter: &dyn LLMAdapter) -> Result<()> {
    adapter.init_project(path, &design, &env)?;
}

// ✅ Better: generic constraint
pub fn init<A: LLMAdapter>(path: &Path, adapter: &A) -> Result<()> {
    adapter.init_project(path, &design, &env)?;
}
```

### 4. Runtime Selection

Let users choose adapter at runtime:

```rust
pub fn select_adapter(name: &str) -> Result<Box<dyn LLMAdapter>> {
    match name {
        "claude" => Ok(Box::new(ClaudeAdapter::new()?)),
        "gemini" => Ok(Box::new(GeminiAdapter::new()?)),
        _ => Err(Error::UnknownAdapter(name.to_string())),
    }
}
```

### 5. Combine with Dependable-Rust Pattern

Each adapter is a black-box module:

```
src/adapters/claude/
├── mod.rs          # Public LLMAdapter impl
└── internal.rs     # Claude-specific logic (templates, formatting)
```

## Versioning Strategy

External systems evolve - adapters should track versions:

```rust
pub trait LLMAdapter {
    fn name(&self) -> &'static str;
    fn version(&self) -> &str;  // "claude-3.5" or "gemini-2.0"
    // ... other methods
}

impl ClaudeAdapter {
    pub fn new() -> Result<Self> {
        Ok(Self {
            version: "3.5".to_string(),  // Claude API version
            // ...
        })
    }
}
```

**Changelog best practice:**
```markdown
# Claude Adapter Changelog

## v3.5 (2024-11-15)
- Added artifacts support
- Updated context format for extended thinking

## v3.0 (2024-03-01)
- Initial Claude 3 support
```

## Testing Strategy

Adapters enable clean testing:

**1. Mock adapter for tests:**
```rust
struct MockAdapter {
    calls: RefCell<Vec<String>>,
}

impl LLMAdapter for MockAdapter {
    fn init_project(&self, path: &Path, ...) -> Result<()> {
        self.calls.borrow_mut().push(format!("init: {:?}", path));
        Ok(())
    }
}

#[test]
fn test_init_command() {
    let adapter = MockAdapter::new();
    init("/tmp/test", &adapter).unwrap();
    assert_eq!(adapter.calls.borrow()[0], "init: \"/tmp/test\"");
}
```

**2. Integration tests per adapter:**
```rust
// tests/claude_adapter_test.rs
#[test]
fn test_claude_generates_valid_context() {
    let adapter = ClaudeAdapter::new().unwrap();
    let patterns = load_test_patterns();
    let ctx = adapter.generate_context(&patterns, &env).unwrap();
    assert!(ctx.contains("# Project Patterns"));
}
```

## Common Mistakes

**1. Leaking adapter-specific types into trait**
```rust
// ❌ Bad: trait exposes Claude-specific type
trait LLMAdapter {
    fn get_config(&self) -> ClaudeConfig;  // ❌
}

// ✅ Good: trait uses generic type
trait LLMAdapter {
    fn get_config(&self) -> Value;  // ✅ or generic Config
}
```

**2. Not using trait objects in commands**
```rust
// ❌ Bad: command knows about all adapters
pub fn init(path: &Path, adapter_name: &str) -> Result<()> {
    if adapter_name == "claude" {
        ClaudeAdapter::new()?.init_project(...)?;
    } else if adapter_name == "gemini" {
        GeminiAdapter::new()?.init_project(...)?;
    }
}

// ✅ Good: command uses trait
pub fn init(path: &Path, adapter: &dyn LLMAdapter) -> Result<()> {
    adapter.init_project(...)?;
}
```

**3. Making trait too large**
```rust
// ❌ Bad: 20+ methods in trait
trait LLMAdapter {
    fn init_project(...) -> Result<()>;
    fn update_config(...) -> Result<()>;
    fn validate_setup(...) -> Result<()>;
    fn generate_docs(...) -> Result<()>;
    fn format_pattern(...) -> String;
    fn format_session(...) -> String;
    fn format_insight(...) -> String;
    // ... 15 more methods
}

// ✅ Good: focused trait
trait LLMAdapter {
    fn name(&self) -> &'static str;
    fn init_project(...) -> Result<()>;
    fn generate_context(...) -> Result<String>;
}
// Formatting methods are internal to adapter
```

## Benefits

When you use adapter pattern:
- ✅ New adapters added without changing core
- ✅ Each adapter optimizes for its system
- ✅ Clean testing with mock adapters
- ✅ Future-proof architecture
- ✅ User choice at runtime

## References

- [Dependable Rust](./dependable-rust.md) - How to structure each adapter as a black box
- [Unix Philosophy](./unix-philosophy.md) - Adapters as focused tools
- Rust Book: Trait Objects - https://doc.rust-lang.org/book/ch17-02-trait-objects.html
