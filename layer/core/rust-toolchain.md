---
id: rust-toolchain
status: verified
verification_date: 2025-08-02
oxidizer: nicabar
references: [core/constraints.md]
tags: [rust, toolchain, dependencies]
---

# Rust Toolchain

Patina uses a specific, consistent Rust toolchain and dependencies.

## Verification

```bash
#!/bin/bash
# Verify Rust toolchain configuration:

echo "Checking Rust toolchain..."

# Core dependencies in Cargo.toml
grep -q 'anyhow = ' Cargo.toml || exit 1
grep -q 'clap = ' Cargo.toml || exit 1
grep -q 'serde = ' Cargo.toml || exit 1
grep -q 'toml = ' Cargo.toml || exit 1

# Async runtime when needed
grep -q 'tokio = ' Cargo.toml || echo "⚠ Tokio not used (no async needed yet)"

# Development tools configured
test -f rustfmt.toml || test -f .rustfmt.toml || echo "⚠ Using default rustfmt"
test -f clippy.toml || test -f .clippy.toml || echo "⚠ Using default clippy"

# CI runs these checks
test -f .github/workflows/ci.yml && grep -q "cargo fmt" .github/workflows/ci.yml || echo "⚠ No CI formatting check"
test -f .github/workflows/ci.yml && grep -q "cargo clippy" .github/workflows/ci.yml || echo "⚠ No CI clippy check"

echo "✓ Rust toolchain verified"
```

## The Pattern

Consistent toolchain across all Patina development:

- **Error handling**: `anyhow` for applications
- **CLI parsing**: `clap` with derive macros
- **Serialization**: `serde` + `toml`
- **Formatting**: `rustfmt` with defaults
- **Linting**: `clippy` with workspace lints

## Implementation

```toml
# Cargo.toml
[dependencies]
anyhow = "1.0"
clap = { version = "4.0", features = ["derive"] }
serde = { version = "1.0", features = ["derive"] }
toml = "0.8"

[lints.clippy]
all = "warn"
```

## Consequences

- Consistent code style
- Predictable error handling
- No dependency conflicts
- Easy onboarding