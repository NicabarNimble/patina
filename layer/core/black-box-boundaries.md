---
id: black-box-boundaries
status: verified
verification_date: 2025-08-09
oxidizer: nicabar
references: [core/adapter-pattern.md, core/rust-ownership.md]
tags: [architecture, modularity, black-box, dependable-rust]
---

# Black-Box Boundaries

Patina achieves modularity through small public interfaces that hide large implementations.

## Verification

```bash
#!/bin/bash
# Verify black-box boundaries in codebase:

echo "Checking black-box boundaries..."

# 1. Check refactored modules have minimal public APIs
for module in claude_refactored init_refactored indexer_refactored; do
    if [ -d "src/*/${module}" ] || [ -d "src/commands/${module}" ] || [ -d "src/adapters/${module}" ]; then
        # Find the module
        mod_file=$(find src -name "${module}" -type d | head -1)/mod.rs
        if [ -f "$mod_file" ]; then
            # Count public items (should be < 150 lines total)
            pub_lines=$(wc -l "$mod_file" | awk '{print $1}')
            pub_items=$(grep -c "^pub " "$mod_file" || echo 0)
            
            if [ "$pub_lines" -lt 150 ]; then
                echo "✓ ${module}: ${pub_lines} lines, ${pub_items} public items"
            else
                echo "✗ ${module}: ${pub_lines} lines exceeds 150 line limit"
                exit 1
            fi
        fi
    fi
done

# 2. Verify implementation modules are private
grep -q "mod implementation;" src/adapters/claude_refactored/mod.rs || exit 1
grep -q "mod implementation;" src/commands/init_refactored/mod.rs || exit 1

# 3. Check for proper visibility markers
if grep -r "pub(super)" src/adapters/claude_refactored/implementation.rs | grep -q "impl"; then
    echo "✓ Implementation uses pub(super) for internal visibility"
else
    echo "⚠ Implementation should use pub(super)"
fi

# 4. Verify no excessive public exports in refactored modules
for mod_file in src/*/refactored/mod.rs src/*/*refactored/mod.rs; do
    if [ -f "$mod_file" ]; then
        exports=$(grep -c "^pub use" "$mod_file" || echo 0)
        if [ "$exports" -gt 3 ]; then
            echo "✗ $(basename $(dirname "$mod_file")) has $exports public exports (limit: 3)"
            exit 1
        fi
    fi
done

# 5. Check that original modules still exist (gradual migration)
[ -f "src/adapters/claude.rs" ] || exit 1
[ -f "src/commands/init/mod.rs" ] || exit 1

echo "✓ Black-box boundaries verified"
```

## The Pattern

Black-box boundaries separate public contracts from private implementations:

```rust
// Public module (< 150 lines total)
pub struct PatternIndexer {
    inner: Box<implementation::IndexerImpl>,
}

impl PatternIndexer {
    pub fn new() -> Self {
        Self { inner: Box::new(implementation::IndexerImpl::new()) }
    }
    
    pub fn index(&self, path: &Path) -> Result<()> {
        self.inner.index(path)
    }
}

// Private implementation (any size)
mod implementation {
    pub(super) struct IndexerImpl {
        // Can be 1000+ lines - completely hidden
        cache: Arc<Mutex<NavigationMap>>,
        db: Option<DatabaseBackend>,
        state_machine: Arc<Mutex<StateMachine>>,
    }
}
```

## Implementation

The pattern is applied consistently across Patina:

1. **Claude Adapter**: 902 lines → <50 line public API
2. **Init Command**: 732 lines → 12 line public API
3. **Indexer Module**: 17 public exports → 1 facade
4. **Hybrid Database**: 641 lines → ~40 line public API
5. **Workspace Client**: 9 public structs → 0 public structs

Each refactored module:
- Has a minimal `mod.rs` with public interface
- Hides implementation in private `implementation.rs`
- Uses `pub(super)` for internal visibility
- Is switchable via environment variables

## Consequences

- **Single ownership** - One person owns each black box
- **Implementation freedom** - Refactor internals without breaking callers
- **Gradual migration** - Both versions coexist during transition
- **Clear boundaries** - Public API is the only contract
- **Better testing** - Test at the boundary, not internals