---
id: pattern-selection-framework
status: active
created: 2025-08-13
tags: [architecture, patterns, meta-pattern, llm-development, system-design]
references: [dependable-rust, dependable-go, eskil-steenberg-rust]
---

# Pattern Selection Framework - Choosing the Right Architecture

**Core Insight**: Not all code needs the same pattern. Patina is a system that helps build systems with LLMs by applying the right patterns in the right contexts.

---

## The Fundamental Challenge

LLMs are excellent at building **tools** (stateless transformations with clear boundaries) but struggle with **systems** (stateful orchestration across multiple contexts). Patina bridges this gap by:

1. Maintaining system context across LLM sessions
2. Decomposing systems into tool-sized pieces LLMs can handle
3. Applying appropriate patterns based on code characteristics

## Three Categories of Code

### 1. Eternal Tools (Black Box Pattern)
**Characteristics:**
- Stable, well-understood domain
- Clear input → output transformation
- API can remain unchanged for decades
- Single-owner mental model

**When to Apply:**
- Core business logic
- Data structures and algorithms
- Parsing and serialization
- Mathematical computations

**Pattern to Use:** `eskil-steenberg-rust` or `dependable-rust`

**Example:**
```rust
// This API can last 50 years
pub fn parse_yaml(input: &str) -> Result<Value>
pub fn compress_data(data: &[u8]) -> Result<Vec<u8>>
```

### 2. Stable Adapters (Versioned Bridges)
**Characteristics:**
- Bridge to external systems
- API evolves slowly with vendor changes
- Need version awareness
- Can hide implementation complexity

**When to Apply:**
- LLM integrations (Claude, Gemini)
- Database adapters
- API clients
- Protocol implementations

**Pattern to Use:** `dependable-rust` with versioning strategy

**Example:**
```rust
// Version-aware but stable interface
pub trait LLMAdapter {
    fn version(&self) -> &str;
    fn generate(&self, prompt: &str) -> Result<Response>;
}
```

### 3. Evolution Points (Replaceable Components)
**Characteristics:**
- Rapidly evolving domain
- Integration with unstable dependencies
- Expected to be replaced entirely
- Simplicity over abstraction

**When to Apply:**
- Container orchestration
- Build tool integration
- Experimental features
- Glue code between systems

**Pattern to Use:** New pattern - **Replaceable Components**

**Example:**
```go
// Simple interface, planned obsolescence
// Expected lifetime: 6-12 months
type WorkspaceManager interface {
    Create(name string) error
    Execute(cmd string) error
    Delete(name string) error
}
```

## The Tool vs System Distinction

### Tools (LLMs Excel Here)
- Have one primary operation
- Transform input → output predictably
- Don't maintain state between calls
- Context-independent behavior

**Examples:** Parser, hasher, formatter, validator, converter

### Systems (Need Patina's Help)
- Coordinate multiple operations
- Maintain complex state
- Depend on context/environment
- Require mental model across interactions

**Examples:** Application framework, workspace manager, build orchestrator, project initializer

## Pattern Selection Decision Tree

```
Is the domain stable and well-understood?
├─ YES → Is it a pure transformation?
│   ├─ YES → Eternal Tool (eskil-steenberg)
│   └─ NO → Is it version-aware?
│       ├─ YES → Stable Adapter (dependable-rust + versioning)
│       └─ NO → Stable Adapter (dependable-rust)
└─ NO → Is it expected to change frequently?
    ├─ YES → Evolution Point (replaceable-component)
    └─ NO → Is it orchestration/glue?
        ├─ YES → Evolution Point (replaceable-component)
        └─ NO → Reassess - might be multiple components
```

## The "Do X" Test

Before applying any pattern, ensure you can clearly state what the component does:

### Good "Do X" (Clear Tools)
✅ "Compress files using gzip"
✅ "Parse TOML into data structure"
✅ "Calculate hash of data"

### Bad "Do X" (Hidden Systems)
❌ "Manage workspaces" (too vague)
❌ "Handle project initialization" (multiple responsibilities)
❌ "Integrate with build tools" (unbounded scope)

### When "Do X" is Unclear
1. **Split it**: Break into multiple black boxes
2. **Layer it**: Stable core + replaceable adapters
3. **Accept it**: Some code is glue - optimize for clarity over permanence

## Implementation Strategy

### For New Code
1. Identify which category the component falls into
2. Apply the appropriate pattern from the start
3. Document the expected lifetime and replacement strategy

### For Existing Code
1. Don't refactor working code just to match a pattern
2. Apply patterns when code needs to change anyway
3. Focus refactoring on code that's causing problems

### For LLM Development
When working with LLMs, frame tasks using this template:

```
Task: [Build a tool that does X]
Category: [Eternal Tool | Stable Adapter | Evolution Point]
Pattern: [Which pattern applies]
API: [2-5 public functions]
Success: [Measurable outcome]
Not Responsible For: [What it explicitly doesn't do]
```

## Flexibility Architecture

### Primary + Fallback + Future

For critical system components, maintain flexibility:

1. **Primary**: Best-in-class solution
2. **Fallback**: Mature, universal alternative
3. **Future**: Next-generation possibility

**Examples in Patina:**
- Container: Dagger (primary) → Docker (fallback) → Native (future)
- LLM: Claude (primary) → Gemini (fallback) → Local (future)
- Language: Rust (primary) → Go (when needed) → Native tool language (escape)

## Pattern Storage and Evolution

Patina tracks which patterns work where:

```yaml
# Pattern metadata for selection
patterns:
  dependable-rust:
    when:
      - language: rust
      - domain: stable
      - scope: tool
    success_rate: 0.92
    
  replaceable-component:
    when:
      - domain: unstable
      - integration: external
      - expected_lifetime: "<1 year"
    success_rate: 0.87
```

## Key Principles

1. **Not all code is equal** - Different code needs different patterns
2. **Tools over systems** - Decompose systems into tools for LLMs
3. **Explicit boundaries** - Clear interfaces enable replacement
4. **Document intentions** - State expected lifetime and replacement strategy
5. **Embrace impermanence** - Some code is meant to be replaced

## Conclusion

Patina's value isn't in forcing one pattern everywhere, but in knowing which pattern fits where. It maintains the system understanding that LLMs lack, enabling effective decomposition of complex systems into tool-sized pieces that LLMs can build successfully.

The pattern selection framework ensures that:
- Eternal code gets eternal patterns
- Evolving code gets flexibility
- LLMs get clear, bounded tasks
- Systems emerge from well-chosen tools

Remember: **Patina is a system to help build systems with LLMs** - it provides the persistent architectural memory that makes LLM-driven development practical for real projects.