---
id: dependable-go
status: active
created: 2025-08-11
references: [dependable-rust]
tags: [architecture, go, black-box, dependable, core]
---

# Dependable Go - Code Structure

**Purpose:** Keep a tiny, stable external interface and push changeable details behind internal packages. Easy to review, test, and evolve.

**Sister Pattern:** See `layer/core/dependable-rust.md` for the Rust equivalent.

---

## Canonical layout

```
module/
├── cmd/
│   └── app/
│       └── main.go        # Minimal entry point (≤100 lines)
├── pkg/
│   └── module/
│       ├── api.go          # Public API (≤150 lines)
│       └── internal/       # Private implementation
│           ├── core.go
│           ├── state.go
│           └── helpers.go
└── internal/               # Module-private packages
    └── impl/
```

## External interface (`api.go`) rules

* Keep ≤150 lines: package docs, type definitions, constructors, minimal methods
* No references to `internal/` in public signatures
* Provide clear error types (not raw errors.New)
* Include at least one example test

## Internal implementation (`internal/`) rules

* Everything defaults to unexported
* Only `api.go` decides what becomes public
* Keep heavy logic in internal packages
* Use interfaces for testing/mocking

## Wiring options

**A) Facade pattern (recommended)**
```go
// api.go - Public facade
package workspace

type Manager struct {
    impl *internal.ManagerImpl
}

func NewManager(config Config) (*Manager, error) {
    impl, err := internal.NewManagerImpl(config)
    return &Manager{impl: impl}, err
}

func (m *Manager) CreateWorkspace(name string) (*Workspace, error) {
    return m.impl.CreateWorkspace(name)
}
```

**B) Interface + factory (for multiple implementations)**
```go
// api.go
type Manager interface {
    CreateWorkspace(name string) (*Workspace, error)
}

func NewManager(config Config) (Manager, error) {
    return internal.NewDefaultManager(config)
}
```

## Error handling pattern

```go
// api.go - Public errors
var (
    ErrNotFound = errors.New("workspace not found")
    ErrInvalid  = errors.New("invalid configuration")
)

// internal/ - Wrap with context
return fmt.Errorf("%w: %s", ErrNotFound, id)
```

## Testing strategy

* **Example tests** in `api_test.go` show usage
* **Unit tests** in `internal/*_test.go` for logic
* **Integration tests** in `test/` exercise full API
* **Table-driven tests** for comprehensive coverage

## CI guards (lightweight)

```bash
# scripts/check_interface.sh
file="pkg/*/api.go"
[ $(wc -l < "$file") -le 150 ] || { echo "Interface too large: $file"; exit 1; }
```

## Go-specific patterns

### Context propagation
```go
func (m *Manager) CreateWorkspace(ctx context.Context, name string) (*Workspace, error)
```

### Options pattern for extensibility
```go
type Option func(*config)

func WithTimeout(d time.Duration) Option {
    return func(c *config) { c.timeout = d }
}

func NewManager(opts ...Option) (*Manager, error)
```

### Graceful shutdown
```go
func (m *Manager) Close() error {
    // Clean shutdown logic
}
```

## Cross-language comparison
* **Rust**: `mod.rs` (small) + `internal.rs` ≈ Go's `api.go` + `internal/`
* **TypeScript**: `index.ts` (exports) + `impl.ts` ≈ Go's pattern
* **C**: `.h` (interface) + `.c` (implementation) ≈ Go's approach