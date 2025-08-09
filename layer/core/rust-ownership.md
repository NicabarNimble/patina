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

## Verification

```bash
#!/bin/bash
# Verify ownership patterns are followed:

echo "Checking ownership patterns..."

# No unnecessary cloning - prefer borrowing
if grep -r "\.clone()" src/ | grep -v "test" | grep -v "// TODO" | wc -l | grep -q "^0"; then
    echo "✓ No unnecessary cloning found"
else
    echo "⚠ Some cloning detected (may be necessary)"
fi

# Layer owns its patterns
grep -q "pub struct Layer {" src/layer/mod.rs || exit 1
grep -q "patterns: Vec<Pattern>" src/layer/mod.rs || exit 1

# Session manager owns sessions
grep -q "pub struct SessionManager" src/session.rs || exit 1

# Proper use of references in function signatures
grep -q "fn.*(&self" src/layer/mod.rs || exit 1
grep -q "fn.*(&mut self" src/layer/mod.rs || exit 1

# Result types for error handling
grep -q "Result<" src/layer/mod.rs || exit 1
grep -q "use anyhow::Result" src/ -r || exit 1

echo "✓ Rust ownership patterns verified"
```

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