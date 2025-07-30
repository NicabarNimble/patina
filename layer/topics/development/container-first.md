---
id: container-first
version: 1
created_date: 2025-07-15
confidence: medium
oxidizer: nicabar
tags: []
---

# Container-First Development Pattern

## Philosophy
Start every project with containers in mind. Development happens in containers, testing happens in containers, deployment is just moving containers.

## Benefits
1. **Consistency** - Same environment everywhere
2. **Reproducibility** - No "works on my machine"
3. **Isolation** - Clean separation of concerns
4. **Portability** - Deploy anywhere

## Implementation

### Development Workflow
```
Local Code → Container Build → Test in Container → Deploy Container
```

### Tool Hierarchy
1. **Dagger** (when available) - Fast, cached, programmable
2. **Docker** (always) - Universal, simple, reliable
3. **Future** - Whatever comes next

### Project Structure
```
project/
├── Dockerfile          # Always present
├── pipelines/         # Optional Dagger
│   └── main.go
├── src/               # Application code
└── .patina/           # Context management
```

## Patterns

### Multi-Stage Builds
```dockerfile
# Build stage
FROM rust:latest AS builder
WORKDIR /app
COPY . .
RUN cargo build --release

# Runtime stage
FROM debian:slim
COPY --from=builder /app/target/release/app /usr/local/bin/
CMD ["app"]
```

### Development vs Production
- Dev: Full toolchain, hot reload, debugging
- Prod: Minimal image, security hardened, optimized

### Caching Strategy
- Language-specific caches (cargo, npm, etc.)
- Build artifact caches
- Test result caches

## Integration with Patina

### Project Types
- **Apps** → Always containerized
- **Tools** → Can be containerized for testing
- **Libraries** → Containerized test environments

### Brain Patterns
- Store successful Dockerfile patterns
- Capture caching strategies
- Document deployment configurations

### Environment Detection
- Check for Docker/Podman
- Detect orchestration tools
- Adapt to available resources

## Escape Hatches

1. **No Docker?** → Native builds with warnings
2. **No Dagger?** → Standard Docker commands
3. **Air-gapped?** → Local registry patterns

## Evolution Path

This pattern enables:
- Easy CI/CD integration
- Cloud-native deployment
- Microservice architectures
- Serverless adaptations