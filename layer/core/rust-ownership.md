---
id: rust-ownership
status: verified
verification_date: 2025-08-02
oxidizer: nicabar
references: [core/principles.md, core/black-box-boundaries.md]
tags: [rust, architecture, ownership]
---

# Rust Ownership

Patina uses Rust's ownership system to enforce clear data responsibilities and prevent runtime errors.

## The Pattern

Ownership in Patina follows these principles:

1. **Layer owns patterns** - Not borrowed or shared
2. **Sessions own their data** - Clear lifecycle management
3. **Borrow by default** - Clone only when necessary
4. **Result for fallibility** - Every operation that can fail returns Result

## Implementation

```rust
// Layer owns its patterns
pub struct Layer {
    patterns: Vec<Pattern>,  // Owned, not &Vec or Rc<Vec>
}

// Methods borrow self appropriately
impl Layer {
    pub fn add_pattern(&mut self, pattern: Pattern) -> Result<()>
    pub fn list_patterns(&self) -> &[Pattern]
}
```

## Consequences

- No data races at compile time
- Clear lifecycle for all data
- Predictable memory usage
- Errors handled explicitly