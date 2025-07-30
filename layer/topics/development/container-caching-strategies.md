---
id: container-caching-strategies
version: 1
created_date: 2025-07-29
confidence: medium
oxidizer: nicabar
tags: []
---

# Container Caching Strategies

## Overview
Strategies for optimizing container builds through intelligent caching, reducing build times and improving development velocity.

## Core Principles

1. **Layer Optimization**: Structure Dockerfiles to maximize cache hits
2. **Dependency Isolation**: Separate frequently changing from stable layers
3. **Tool-Specific Caches**: Leverage language-specific caching mechanisms
4. **Multi-Stage Efficiency**: Use build stages to cache intermediate artifacts

## Implementation Patterns

### 1. Dockerfile Layer Ordering

```dockerfile
# Base dependencies (rarely change)
FROM rust:1.75 AS base
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev

# Language dependencies (change occasionally)
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
RUN cargo fetch

# Source code (changes frequently)
COPY src ./src
RUN cargo build --release
```

### 2. Language-Specific Caching

#### Rust Projects
```dockerfile
# Cache cargo registry
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    cargo build --release

# Or with CARGO_HOME
ENV CARGO_HOME=/cache/cargo
RUN --mount=type=cache,target=/cache/cargo \
    cargo build --release
```

#### Node.js Projects
```dockerfile
# Cache npm packages
RUN --mount=type=cache,target=/root/.npm \
    npm ci --only=production
```

#### Python Projects
```dockerfile
# Cache pip packages
RUN --mount=type=cache,target=/root/.cache/pip \
    pip install -r requirements.txt
```

### 3. Dagger Caching Patterns

```go
// Persistent cache volumes
cacheVolume := client.CacheVolume("cargo-cache")

container := client.Container().
    From("rust:1.75").
    WithMountedCache("/usr/local/cargo/registry", cacheVolume).
    WithDirectory("/app", source).
    WithExec([]string{"cargo", "build", "--release"})
```

### 4. Build Context Optimization

```
# .dockerignore
target/
.git/
*.log
tmp/
.env
```

### 5. Multi-Stage Cache Sharing

```dockerfile
# Build stage with full toolchain
FROM rust:1.75 AS builder
WORKDIR /app
COPY . .
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    cargo build --release

# Test stage reuses build cache
FROM builder AS tester
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    cargo test

# Minimal runtime stage
FROM debian:slim AS runtime
COPY --from=builder /app/target/release/app /usr/local/bin/
```

## Advanced Strategies

### 1. Dependency Checksum Caching
```dockerfile
# Only rebuild if dependencies change
COPY Cargo.toml Cargo.lock ./
RUN cargo build --release --locked
COPY src ./src
RUN touch src/main.rs && cargo build --release
```

### 2. Incremental Build Caching
```dockerfile
# Enable incremental compilation
ENV CARGO_INCREMENTAL=1
RUN --mount=type=cache,target=/app/target \
    cargo build --release
```

### 3. Remote Build Caching
```bash
# Push cache to registry
docker buildx build \
  --cache-to type=registry,ref=myapp:buildcache \
  --cache-from type=registry,ref=myapp:buildcache \
  -t myapp:latest .
```

### 4. GitHub Actions Cache
```yaml
- uses: docker/build-push-action@v5
  with:
    cache-from: type=gha
    cache-to: type=gha,mode=max
```

## Monitoring and Optimization

### Cache Hit Analysis
```bash
# Check Docker build cache usage
docker system df
docker builder prune --keep-storage 10GB

# Analyze layer sizes
docker history myimage:latest
```

### Build Time Metrics
```bash
# Time builds to measure improvements
time docker build -t test .

# Use buildx for detailed metrics
docker buildx build --progress=plain .
```

## Best Practices

1. **Order by Change Frequency**: Most stable layers first
2. **Minimize Layer Count**: Combine related RUN commands
3. **Use Specific Versions**: Avoid cache busting from updates
4. **Clean After Install**: Remove package managers caches
5. **Test Cache Effectiveness**: Measure build times regularly

## Common Pitfalls

1. **Cache Busting Commands**: Avoid `RUN apt-get update` alone
2. **Wide COPY Commands**: Use specific paths over `COPY . .`
3. **Dynamic Timestamps**: Avoid embedding dates in builds
4. **Missing .dockerignore**: Include unnecessary files
5. **Order Dependencies**: Wrong order defeats caching

## Tool-Specific Integration

### Patina + Dagger
- Use Dagger's native cache volumes
- Share caches across pipeline steps
- Persist caches between runs

### Docker Buildx
- Enable BuildKit features
- Use cache mounts
- Leverage inline cache

### CI/CD Systems
- Configure registry-based caching
- Use platform-specific cache stores
- Monitor cache storage limits

## Evolution and Monitoring

1. **Start Simple**: Basic layer ordering
2. **Measure Impact**: Track build times
3. **Add Advanced Features**: Cache mounts, remote caching
4. **Optimize Continuously**: Refine based on metrics
5. **Share Patterns**: Document what works

## Related Patterns
- Container-First Development
- Multi-Stage Builds
- CI/CD Optimization