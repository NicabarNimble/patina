---
id: adapter-pattern
status: verified
verification_date: 2025-08-02
oxidizer: nicabar
references: [core/constraints.md, topics/architecture/three-dimensions.md, core/black-box-boundaries.md]
tags: [architecture, patterns, adapters]
---

# Adapter Pattern

Patina uses trait-based adapters to remain LLM-agnostic while providing rich integrations.


## The Pattern

All LLM integrations implement a common trait:

```rust
pub trait LLMAdapter {
    fn name(&self) -> &'static str;
    fn init_project(&self, project_path: &Path, design: &Value, 
                    environment: &Environment) -> Result<()>;
    fn generate_context(&self, patterns: &[Pattern], 
                       environment: &Environment) -> Result<String>;
}
```

## Implementation

1. **Core defines the contract** - The Adapter trait
2. **Adapters implement specifics** - Claude, Gemini, etc.
3. **Commands use traits** - Never concrete types
4. **Runtime selection** - Based on user choice

## Consequences

- New LLMs added without changing core
- Each adapter optimizes for its LLM
- Clean testing with mock adapters
- Future-proof architecture