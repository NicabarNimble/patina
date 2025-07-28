# Dagger Integration Pattern

## Overview
Patina uses a template-based approach to integrate Dagger without creating hard dependencies. This pattern ensures LLMs stay within Rust boundaries while leveraging Go-based Dagger pipelines.

## Architecture

### Generation Phase (patina init)
```
patina init --dev=dagger
├── Creates Dockerfile (always - escape hatch)
├── Creates pipelines/
│   ├── main.go (from template)
│   ├── go.mod (from template)
│   └── README.md
└── Never modified again by LLMs
```

### Execution Phase (patina build)
```rust
// Smart fallback logic
if pipelines/main.go exists && go available {
    run_dagger_pipeline()
} else {
    docker_build()
}
```

## Key Principles

1. **Templates Over Runtime Generation**
   - Go code generated once during init
   - Templates versioned with Patina
   - LLMs cannot modify pipeline code

2. **Escape Hatch Philosophy**
   - Docker always available as fallback
   - No hard dependency on Go/Dagger
   - Clear feedback about what's being used

3. **Type-Safe Orchestration**
   - All logic in Rust
   - Compiler catches errors
   - External tools are just executables

## Implementation Details

### Template Structure
```
resources/templates/dagger/
├── main.go.tmpl    # Dagger pipeline
└── go.mod.tmpl     # Go module definition
```

### Template Variables
- `{{.name}}` - Project name
- Future: `{{.type}}`, `{{.features}}` based on PROJECT_DESIGN.toml

### Build Command Logic
```rust
pub fn execute() -> Result<()> {
    // Prefer Dagger if available
    if has_dagger_pipeline() && has_go() {
        run_go_pipeline()?;
    } else {
        run_docker_build()?;
    }
    Ok(())
}
```

## Benefits

1. **No SDK Maintenance** - Uses official Go SDK
2. **Full Dagger Power** - No limitations from experimental Rust SDK
3. **Clear Boundaries** - Rust orchestrates, Go executes
4. **Future Proof** - Easy to swap Dagger for next tool

## Pattern Evolution

This pattern can evolve to:
- Support multiple pipeline types (test, deploy, etc.)
- Generate based on PROJECT_DESIGN.toml features
- Include user-specific patterns
- Support other tools beyond Dagger