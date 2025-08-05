# Feature: Interactive Design Command

This branch adds an interactive `patina design` command to help developers create thoughtful PROJECT_DESIGN.toml files.

## Integration

To add this feature to main, only two files need modification:

### 1. Add to `src/commands/mod.rs`:
```rust
pub mod design;
```

### 2. Add to `src/main.rs`:

In the `Commands` enum:
```rust
/// Create an interactive PROJECT_DESIGN.toml
Design(commands::design::DesignCommand),
```

In the match statement:
```rust
Commands::Design(cmd) => {
    tokio::runtime::Runtime::new()?.block_on(cmd.execute())?;
}
```

## Usage

```bash
# Interactive mode (default)
patina design

# Quick mode with defaults  
patina design --no-scan

# Specify output
patina design --output my-design.toml
```

## Features

- **Environment Scanning**: Auto-detects language, dependencies, test setup
- **Guided Interview**: Smart questions based on project context
- **TOML Generation**: Creates properly formatted PROJECT_DESIGN.toml
- **Review & Refine**: Option to improve the generated design

## Files Added

- `src/commands/design.rs` - Complete implementation