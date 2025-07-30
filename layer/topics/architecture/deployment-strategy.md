---
id: deployment-strategy
version: 1
created_date: 2025-07-15
confidence: medium
oxidizer: nicabar
tags: []
---

# Deployment Strategy

## Core Decision
Different project types have different deployment strategies:

### Apps → Docker (Always)
- Consistent deployment target
- Works everywhere (Mac, Linux, Unraid)
- No platform-specific complexity
- Simple mental model: "Apps ship in containers"

### Tools → Native Binaries
- Compile for target platforms (Mac, Linux, Windows)
- No container overhead for CLI tools
- Direct distribution as executables
- Example: patina itself

### Libraries → crates.io
- Libraries don't "deploy", they publish
- Standard Rust ecosystem distribution
- Consumed by other projects

## Implementation
The `type` field in PROJECT_DESIGN.toml drives deployment:

```toml
[project]
type = "app"     # → Docker deployment
type = "tool"    # → Multi-platform binaries  
type = "library" # → Crates.io publishing
```

## Rationale
- Simplifies user mental model
- Opinionated approach reduces decision fatigue
- Each type has one clear deployment path
- Aligns with ecosystem expectations