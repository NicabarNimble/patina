---
id: dev-environments-architecture
version: 1
created_date: 2025-08-08
confidence: high
oxidizer: nicabar
tags: [development, docker, dagger, environments, architecture]
promoted_from: projects/patina
---

# Development Environments in Patina

## Overview

Patina's development environment system provides a pluggable architecture for building and testing projects using different containerization strategies. This allows projects to leverage the best tools available while maintaining consistent interfaces.

## Architecture

### Core Trait

```rust
pub trait DevEnvironment {
    fn name(&self) -> &'static str;
    fn version(&self) -> &'static str;
    fn init_project(&self, project_path: &Path, project_name: &str, project_type: &str) -> Result<()>;
    fn build(&self, project_path: &Path) -> Result<()>;
    fn test(&self, project_path: &Path) -> Result<()>;
    fn is_available(&self) -> bool;
    fn fallback(&self) -> Option<&'static str>;
}
```

### Available Environments

1. **Docker** (Default)
   - Traditional containerization
   - Template-based Dockerfile generation
   - No external services required
   - Works everywhere Docker is installed

2. **Dagger** (Advanced)
   - Leverages workspace service for isolated builds
   - Each build/test gets a fresh container with git worktree
   - Requires `patina agent start` to run workspace service
   - Falls back to Docker if Go is not available

3. **Native** (Future)
   - Direct compilation without containers
   - For environments where containers aren't available

## How It Works

### During `patina init`

1. Environment detection determines best dev environment
2. Dev environment's `init_project()` is called
3. For Docker: Creates Dockerfile, docker-compose.yml, .dockerignore
4. For Dagger: Just prints message (no templates needed)

### During `patina build`

1. Reads `.patina/config.json` to get configured dev environment
2. Calls the dev environment's `build()` method
3. If primary fails and has fallback, tries fallback automatically

### Environment Selection

The init command intelligently selects the best environment:

```rust
if has_docker && has_go {
    "dagger"  // Fastest with caching
} else if has_docker {
    "docker"  // Standard containerization
} else {
    "native"  // No containers available
}
```

Users can override with:
- `patina init --dev=docker` during initialization
- `PATINA_DEV=docker` environment variable

## Docker Environment

### Approach
- Template-based file generation
- Standard Docker commands
- Simple and reliable

### Generated Files
- `Dockerfile` - Based on project type (app/tool/library)
- `docker-compose.yml` - For app projects
- `.dockerignore` - Excludes unnecessary files

### Commands
```bash
# Build
docker build -t project:latest .

# Test
docker run --rm project:latest cargo test
```

## Dagger Environment

### Approach
- Service-based dynamic containers
- HTTP API to workspace service
- Git worktree isolation

### Architecture
```
patina build
  ↓
DaggerEnvironment::build()
  ↓
WorkspaceClient (HTTP)
  ↓
Workspace Service (Go)
  ↓
Dagger SDK
  ↓
Container with excludes
```

### Key Features
1. **No Templates**: Everything is dynamic
2. **Workspace Isolation**: Each build gets a unique workspace
3. **Smart Excludes**: Prevents uploading large directories
4. **Git Integration**: Uses worktrees for proper isolation

### Requirements
- Go installed (for workspace service)
- `patina agent start` running
- Docker daemon available

## Fallback System

The trait includes a fallback mechanism:

```rust
impl DevEnvironment for DaggerEnvironment {
    fn fallback(&self) -> Option<&'static str> {
        Some("docker")  // Falls back to Docker if unavailable
    }
}
```

This ensures builds always work, even if the preferred environment isn't available.

## Evolution

### Past: Template-Based Dagger
- Generated Go pipelines with constraints
- 50-line function limits
- No interfaces allowed
- Turned Dagger into "worse version of itself"

### Present: Workspace Service
- Full Go service with proper architecture
- Dynamic workspace creation
- Professional code structure
- Inspired by container-use project

### Future: Additional Environments
- Nix for reproducible builds
- Bazel for large monorepos
- Native for simple projects

## Best Practices

1. **Let init choose**: The auto-detection usually picks the best option
2. **Start agent for Dagger**: Run `patina agent start` before using Dagger
3. **Commit config**: The `.patina/config.json` tracks your choice
4. **Don't mix approaches**: Pick one and stick with it per project

## Common Issues

### "Workspace service is not running"
- **Solution**: Run `patina agent start`
- **Why**: Dagger needs the Go workspace service

### "3.6GB upload" with Dagger
- **Solution**: Fixed by exclude patterns in workspace manager
- **Why**: Was uploading entire project including build artifacts

### Build fails with Dagger but works with Docker
- **Check**: Is `patina agent start` running?
- **Check**: Do you have Go installed?
- **Fallback**: Use `--dev=docker` during init