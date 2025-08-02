---
id: adapter-pattern
status: verified
verification_date: 2025-08-02
oxidizer: nicabar
references: [core/constraints.md, topics/architecture/three-dimensions.md]
tags: [architecture, patterns, adapters]
---

# Adapter Pattern

Patina uses trait-based adapters to remain LLM-agnostic while providing rich integrations.

## Verification

```bash
#!/bin/bash
# Verify adapter pattern implementation:

echo "Checking adapter pattern..."

# Core adapter trait exists
grep -q "pub trait LLMAdapter" src/adapters/mod.rs || exit 1

# Required adapter methods
grep -q "fn name(&self)" src/adapters/mod.rs || exit 1
grep -q "fn init_project" src/adapters/mod.rs || exit 1
grep -q "fn generate_context" src/adapters/mod.rs || exit 1

# Concrete adapters implement the trait
grep -q "impl LLMAdapter for ClaudeAdapter" src/adapters/claude.rs || exit 1
grep -q "impl LLMAdapter for GeminiAdapter" src/adapters/gemini.rs || exit 1

# No adapter-specific code in core commands
if grep -r "ClaudeAdapter" src/commands/ 2>/dev/null | grep -v "test"; then
    echo "✗ Adapter-specific code found in commands"
    exit 1
fi

# Trait objects used for dynamic dispatch
grep -q "Box<dyn LLMAdapter>" src/ -r || grep -q "&dyn LLMAdapter" src/ -r || exit 1

echo "✓ Adapter pattern verified"
```

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