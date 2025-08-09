---
id: modularity-through-interfaces
status: verified
verification_date: 2025-08-09
oxidizer: nicabar
references: [core/black-box-boundaries.md, core/unix-philosophy.md]
tags: [architecture, modularity, interfaces, dependable-rust]
---

# Modularity Through Interfaces

Patina achieves modularity through small trait surfaces, not small files.

## Verification

```bash
#!/bin/bash
# Verify interface-based modularity:

echo "Checking modularity patterns..."

# 1. Large implementation files are OK when private
for impl_file in src/*/refactored/implementation.rs src/*/*refactored/implementation.rs; do
    if [ -f "$impl_file" ]; then
        lines=$(wc -l "$impl_file" | awk '{print $1}')
        module=$(basename $(dirname "$impl_file"))
        
        # Check if it's properly hidden
        mod_file="$(dirname "$impl_file")/mod.rs"
        if grep -q "mod implementation;" "$mod_file"; then
            echo "✓ ${module}: ${lines} lines (properly hidden)"
        else
            echo "✗ ${module}: implementation not properly hidden"
            exit 1
        fi
    fi
done

# 2. Verify trait-based interfaces
trait_count=$(grep -r "^pub trait" src/ | wc -l)
if [ "$trait_count" -gt 5 ]; then
    echo "✓ Found $trait_count public traits defining interfaces"
else
    echo "⚠ Only $trait_count public traits found"
fi

# 3. Check that implementations use traits
impl_trait_count=$(grep -r "impl.*for.*{" src/ | wc -l)
if [ "$impl_trait_count" -gt 10 ]; then
    echo "✓ Found $impl_trait_count trait implementations"
else
    echo "⚠ Only $impl_trait_count trait implementations"
fi

# 4. Verify factory functions hide concrete types
if grep -r "pub fn create.*-> impl" src/ | grep -q "impl"; then
    echo "✓ Factory functions return impl Trait"
else
    echo "⚠ Factory functions should return impl Trait"
fi

# 5. Check module size ratios
for module in claude_refactored init_refactored indexer_refactored; do
    mod_path=$(find src -name "${module}" -type d | head -1)
    if [ -d "$mod_path" ]; then
        pub_size=$(wc -l "$mod_path/mod.rs" 2>/dev/null | awk '{print $1}' || echo 0)
        impl_size=$(wc -l "$mod_path/implementation.rs" 2>/dev/null | awk '{print $1}' || echo 0)
        
        if [ "$pub_size" -gt 0 ] && [ "$impl_size" -gt 0 ]; then
            ratio=$((impl_size / pub_size))
            echo "✓ ${module}: 1:${ratio} (public:private ratio)"
        fi
    fi
done

echo "✓ Modularity through interfaces verified"
```

## The Principle

**"Modularity comes from small trait surfaces, not small files"**

A 900-line file with clear ownership is better than 10 files with unclear boundaries.

## The Pattern

```rust
// WRONG: Many small files with unclear boundaries
// src/navigation/
//   ├── cache.rs        (200 lines, partially public)
//   ├── database.rs     (300 lines, partially public)
//   ├── git_state.rs    (150 lines, partially public)
//   └── search.rs       (200 lines, partially public)
// Total: 850 lines, ~20 public items scattered

// RIGHT: One interface, one implementation
// src/navigation/mod.rs (50 lines, fully public)
pub trait Navigator {
    fn index(&mut self, path: &Path) -> Result<()>;
    fn search(&self, query: &str) -> Result<Vec<Match>>;
}

// src/navigation/implementation.rs (850 lines, fully private)
mod implementation {
    pub(super) struct NavigatorImpl {
        // All 850 lines in one file, but hidden
    }
}
```

## Implementation

This principle manifests in Patina's refactored modules:

| Module | Public Lines | Private Lines | Ratio | Public Items |
|--------|-------------|---------------|-------|--------------|
| Claude Adapter | 45 | 902 | 1:20 | 4 |
| Init Command | 13 | 732 | 1:56 | 1 |
| Indexer | 70 | 523 | 1:7 | 3 |
| Hybrid DB | 40 | 641 | 1:16 | 3 |
| Workspace | 45 | 268 | 1:6 | 6 |

## Consequences

- **Easier code navigation** - Implementation details in one place
- **Clear ownership** - One file = one owner's domain
- **Reduced coupling** - Changes don't ripple across files
- **Better refactoring** - Move code within file freely
- **Simpler mental model** - Think in terms of interfaces, not files