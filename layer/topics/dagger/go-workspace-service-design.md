---
id: go-workspace-service-design
version: 1
created_date: 2025-07-29
confidence: medium
oxidizer: nicabar
tags: []
promoted_from: projects/patina
---

---
id: go-workspace-service-design
version: 1
created_date: 2025-07-29
confidence: design
oxidizer: nicabar
tags: [architecture, go, dagger, workspace, service]
---

# Go Workspace Service Design for Patina

## Overview

Transform the current constrained Dagger pipeline approach into a proper Go microservice that provides workspace management capabilities inspired by container-use. This service will be orchestrated by Rust but operate as an independent service with clear boundaries.

## Problem Statement

Current approach has several limitations:
- Single 500+ line main.go violates our own constraints
- Template-based generation prevents evolution
- Artificial constraints prevent using Go idioms
- No proper testing structure
- Monolithic design prevents modularity

## Solution Architecture

### Service Responsibilities

1. **Workspace Management**
   - Create isolated container environments
   - Each workspace gets dedicated git branch
   - Track active workspaces with metadata
   - Resource cleanup and lifecycle management

2. **Container Operations**
   - Execute commands in workspaces
   - Stream logs and output in real-time
   - Manage container lifecycle
   - Enable parallel operations

3. **API Surface**
   - HTTP REST API for simplicity
   - JSON request/response format
   - WebSocket for streaming operations
   - Health checks and observability

## Directory Structure

```
workspace/                          # New Go module
├── go.mod                         # module github.com/patina/workspace
├── go.sum
├── Makefile                       # Build, test, coverage commands
├── README.md                      # Service documentation
│
├── cmd/
│   └── workspace-server/
│       └── main.go               # Service entry point
│
├── pkg/
│   ├── workspace/
│   │   ├── manager.go           # Core workspace manager
│   │   ├── manager_test.go      # Manager unit tests
│   │   ├── workspace.go         # Workspace type and methods
│   │   ├── workspace_test.go    # Workspace unit tests
│   │   └── errors.go            # Custom error types
│   │
│   ├── container/
│   │   ├── container.go         # Container operations
│   │   ├── container_test.go    # Container unit tests
│   │   └── exec.go              # Command execution
│   │
│   └── api/
│       ├── handlers.go          # HTTP handlers
│       ├── handlers_test.go     # Handler tests
│       ├── types.go             # Request/response types
│       └── middleware.go        # Logging, auth, etc.
│
├── internal/
│   └── testutil/
│       ├── dagger_mock.go       # Mock Dagger client
│       └── helpers.go           # Test utilities
│
└── integration/
    ├── workspace_test.go        # Integration tests
    └── parallel_test.go         # Parallel ops tests
```

## Core Components

### Workspace Manager

```go
type Manager struct {
    dag        *dagger.Client
    workspaces sync.Map  // Safe for concurrent access
    config     *Config
    logger     *slog.Logger
}

type Workspace struct {
    ID          string    `json:"id"`
    Name        string    `json:"name"`
    ContainerID string    `json:"container_id"`
    BranchName  string    `json:"branch_name"`
    BaseImage   string    `json:"base_image"`
    CreatedAt   time.Time `json:"created_at"`
    UpdatedAt   time.Time `json:"updated_at"`
    Status      Status    `json:"status"`
}

type Status string

const (
    StatusCreating Status = "creating"
    StatusReady    Status = "ready"
    StatusError    Status = "error"
    StatusDeleting Status = "deleting"
)
```

### API Design

#### Endpoints

```
POST   /workspaces              # Create new workspace
GET    /workspaces              # List all workspaces
GET    /workspaces/{id}         # Get workspace details
DELETE /workspaces/{id}         # Delete workspace
POST   /workspaces/{id}/exec    # Execute command
GET    /workspaces/{id}/logs    # Stream logs (WebSocket)
POST   /workspaces/{id}/commit  # Commit changes to branch
GET    /health                  # Health check
GET    /metrics                 # Prometheus metrics
```

#### Request/Response Types

```go
// Create workspace
type CreateWorkspaceRequest struct {
    Name      string            `json:"name"`
    BaseImage string            `json:"base_image,omitempty"`
    GitBranch string            `json:"git_branch,omitempty"`
    Env       map[string]string `json:"env,omitempty"`
}

type CreateWorkspaceResponse struct {
    Workspace *Workspace `json:"workspace"`
}

// Execute command
type ExecRequest struct {
    Command []string          `json:"command"`
    Env     map[string]string `json:"env,omitempty"`
    WorkDir string            `json:"work_dir,omitempty"`
}

type ExecResponse struct {
    ExitCode int    `json:"exit_code"`
    Stdout   string `json:"stdout"`
    Stderr   string `json:"stderr"`
}
```

## Testing Strategy

Following the testing pyramid philosophy from distributed systems:

### Unit Tests (70%)
- Test workspace manager logic with mocked Dagger client
- Test API handlers with mocked manager
- Test container operations in isolation
- Focus on business logic and error cases

### Integration Tests (25%)
- Test with real Dagger client
- Verify container isolation
- Test parallel workspace operations
- Validate cleanup and resource management

### E2E Tests (5%)
- Smoke tests only
- Full API flow: create → exec → delete
- Basic health check validation
- Minimal scope to avoid fragility

## Implementation Phases

### Phase 1: Foundation (Week 1)
1. Set up Go module structure
2. Define core types and interfaces
3. Implement basic workspace manager
4. Create mock Dagger client for testing

### Phase 2: Core Features (Week 2)
1. Implement workspace CRUD operations
2. Add container command execution
3. Build HTTP API handlers
4. Add comprehensive unit tests

### Phase 3: Advanced Features (Week 3)
1. WebSocket log streaming
2. Parallel workspace operations
3. Git branch management
4. Integration test suite

### Phase 4: Production Ready (Week 4)
1. Add observability (logs, metrics, traces)
2. Implement graceful shutdown
3. Add rate limiting and auth
4. Performance testing

### Phase 5: Rust Integration
1. Update Rust to use HTTP API
2. Remove old pipeline approach
3. Add Rust integration tests
4. Update documentation

## Benefits Over Current Approach

1. **Professional Go Architecture**
   - Proper package structure
   - Interface-based design
   - Comprehensive testing
   - Standard Go idioms

2. **Container-Use Pattern**
   - Isolated workspaces for AI agents
   - Parallel experimentation
   - Full audit trail
   - Resource management

3. **Clear Separation of Concerns**
   - Rust: CLI, patterns, orchestration
   - Go: Container management, Dagger operations
   - HTTP API: Clean interface between them

4. **Maintainability**
   - Each component independently testable
   - Clear interfaces and contracts
   - Easy to extend and modify
   - No artificial constraints

## Testing Philosophy (Updated with rqlite Learnings)

### Key Principles from rqlite
1. **Tests Next to Code** - `workspace.go` → `workspace_test.go` in same directory
2. **Standard Library Only** - No testing frameworks, just `testing` package
3. **Error Variables** - Define errors as package variables (`var ErrNotOpen = errors.New(...)`)
4. **Helper Functions** - `mustNewWorkspace(t)`, `mustConnect(t)` for test setup
5. **Descriptive Test Names** - `Test_WorkspaceCreation`, `Test_NonReadyManager`

### Testing Approach
1. **No Mocks for Core Logic** - Test real behavior whenever possible
2. **Clear Test Separation**:
   - Unit tests: Business logic without external dependencies
   - Integration tests: Real Dagger operations with `//go:build integration`
   - System tests: Minimal end-to-end smoke tests only
3. **Table-Driven Tests** - Use Go's subtests for multiple scenarios

### Recommended File Structure
```
workspace/
├── errors.go            # Package error definitions
├── workspace.go         # Workspace types
├── workspace_test.go    # Workspace tests
├── manager.go          # Manager implementation
├── manager_test.go     # Manager tests
└── integration_test.go # Integration tests (build tagged)
```

## Success Metrics

- Unit test coverage > 80%
- Integration tests for all critical paths
- API response time < 100ms (excluding container operations)
- Zero goroutine leaks
- Graceful handling of Dagger disconnections

## Future Considerations

- gRPC API for better performance
- Distributed workspace management
- Persistent workspace state (PostgreSQL)
- Multi-region support
- Workspace templates and presets