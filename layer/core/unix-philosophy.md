---
id: unix-philosophy
layer: core
status: active
created: 2025-08-02
tags: [architecture, philosophy, decomposition, core-principle]
references: [dependable-rust, adapter-pattern]
---

# Unix Philosophy

**Purpose:** Decompose complex systems into simple, single-purpose tools that do one thing well and compose cleanly.

---

## Core Principle

Patina follows Unix philosophy: **one tool, one job, done well**. Each component has a single, clear responsibility. Complex functionality emerges from composition of simple tools, not from monolithic systems.

## When to Use

Apply this principle when:
- Designing new CLI commands
- Extracting functionality from monolithic code
- Planning module boundaries
- Deciding what belongs in a component

## How to Apply

### 1. Single Responsibility Per Component

Each Patina component has one clear job:

```
layer/          → Manages knowledge storage and retrieval
adapters/       → Handles LLM-specific integration
commands/       → Implements user-facing CLI actions
environment/    → Detects system capabilities
```

### 2. Decomposition Strategy

When facing a complex system:

1. **Identify core responsibilities** - What distinct jobs need doing?
2. **Create focused modules** - One module per responsibility
3. **Apply dependable-rust** - Black-box each module
4. **Compose functionality** - Combine modules to create features

**Example:** Workspace service decomposition (see `modular-architecture-plan.md`)

```
Monolithic workspace/ →  environment-provider/  (create containers)
                         environment-registry/  (track environments)
                         code-executor/        (run commands)
                         git-manager/          (git operations)
```

### 3. Tools vs Systems

**Tools (build these):**
- Single primary operation
- Transform input → output predictably
- Don't maintain complex state
- Context-independent behavior

**Systems (decompose into tools):**
- Coordinate multiple operations
- Maintain complex state
- Depend on context/environment
- Require cross-interaction mental model

### 4. Composition Over Monolith

```rust
// ❌ Bad: monolithic command doing everything
pub fn init_project(path: &Path) -> Result<()> {
    // 500 lines: detect env, copy templates, init git,
    // configure adapters, generate docs...
}

// ✅ Good: composed from focused tools
pub fn init_project(path: &Path) -> Result<()> {
    let env = environment::detect()?;
    let templates = templates::load(&env)?;
    git::init(path)?;
    adapters::configure(path, &env)?;
    Ok(())
}
```

Each function is a tool doing one thing. The command coordinates them.

## Manifestation in Patina

This philosophy appears throughout:

1. **Modular architecture** - Each module can be understood in isolation
2. **Composable commands** - Commands can be piped and combined
3. **Text interfaces** - All output is text, parseable by other tools
4. **No feature creep** - New functionality = new commands, not new flags

## Common Mistakes

**1. Building systems when you need tools**
```rust
// ❌ Bad: "workspace manager" (what does it manage?)
struct WorkspaceManager { /* everything */ }

// ✅ Good: specific tools
fn create_workspace(...) -> Result<Workspace>
fn list_workspaces(...) -> Result<Vec<Workspace>>
fn execute_in_workspace(...) -> Result<Output>
```

**2. Adding flags instead of commands**
```bash
# ❌ Bad: flag soup
patina init --with-git --llm=claude --env=docker --copy-templates

# ✅ Good: separate commands
patina init              # minimal setup
patina git init          # if you want git
patina template apply    # if you want templates
```

**3. Tight coupling between components**
```rust
// ❌ Bad: adapter knows about storage internals
impl ClaudeAdapter {
    fn save_pattern(&self) {
        self.storage.internal.database.insert(...);  // ❌
    }
}

// ✅ Good: use public interface only
impl ClaudeAdapter {
    fn save_pattern(&self) {
        self.storage.add_pattern(&pattern)?;  // ✅
    }
}
```

## Benefits

When you follow Unix philosophy:
- ✅ Easy to test individual components
- ✅ Clear mental model for users
- ✅ Natural composition of functionality
- ✅ Predictable behavior
- ✅ Replace components without breaking others

## References

- [Dependable Rust](./dependable-rust.md) - How to structure each module as a black box
- [Adapter Pattern](./adapter-pattern.md) - Tool pattern for external system bridges
- [Why Rust for LLM Development](../surface/why-rust-for-llm-development.md) - How decomposition helps LLMs
