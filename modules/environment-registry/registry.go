package registry

import (
	"fmt"
	"sync"
	"time"
)

// Environment represents a development environment
type Environment struct {
	ID           string
	Name         string
	Status       string
	BranchName   string
	WorktreePath string
	BaseImage    string
	CreatedAt    string
}

// Registry provides environment storage following the Eternal Tool pattern
// It owns its state and provides clear input->output transformations
type Registry struct {
	mu           sync.RWMutex
	environments map[string]*Environment
}

// NewRegistry creates a new environment registry that owns its state
func NewRegistry() *Registry {
	return &Registry{
		environments: make(map[string]*Environment),
	}
}

// Register adds or updates an environment (write operation)
func (r *Registry) Register(env *Environment) error {
	if env == nil {
		return fmt.Errorf("environment cannot be nil")
	}
	if env.ID == "" {
		return fmt.Errorf("environment ID is required")
	}

	r.mu.Lock()
	defer r.mu.Unlock()

	// If CreatedAt is empty, set it
	if env.CreatedAt == "" {
		env.CreatedAt = time.Now().Format(time.RFC3339)
	}

	// Store a copy to prevent external mutations
	envCopy := *env
	r.environments[env.ID] = &envCopy

	return nil
}

// Deregister removes an environment (write operation)
func (r *Registry) Deregister(id string) error {
	if id == "" {
		return fmt.Errorf("environment ID is required")
	}

	r.mu.Lock()
	defer r.mu.Unlock()

	if _, exists := r.environments[id]; !exists {
		return fmt.Errorf("environment not found: %s", id)
	}

	delete(r.environments, id)
	return nil
}

// Get retrieves an environment by ID (read operation)
func (r *Registry) Get(id string) (*Environment, error) {
	if id == "" {
		return nil, fmt.Errorf("environment ID is required")
	}

	r.mu.RLock()
	defer r.mu.RUnlock()

	env, exists := r.environments[id]
	if !exists {
		return nil, fmt.Errorf("environment not found: %s", id)
	}

	// Return a copy to prevent external mutations
	envCopy := *env
	return &envCopy, nil
}

// List returns all environments (read operation)
func (r *Registry) List() ([]*Environment, error) {
	r.mu.RLock()
	defer r.mu.RUnlock()

	// Create a slice with copies
	environments := make([]*Environment, 0, len(r.environments))
	for _, env := range r.environments {
		envCopy := *env
		environments = append(environments, &envCopy)
	}

	return environments, nil
}

// Exists checks if an environment exists (read operation)
func (r *Registry) Exists(id string) (bool, error) {
	if id == "" {
		return false, fmt.Errorf("environment ID is required")
	}

	r.mu.RLock()
	defer r.mu.RUnlock()

	_, exists := r.environments[id]
	return exists, nil
}

// UpdateStatus updates the status of an environment (write operation)
func (r *Registry) UpdateStatus(id string, status string) error {
	if id == "" {
		return fmt.Errorf("environment ID is required")
	}

	r.mu.Lock()
	defer r.mu.Unlock()

	env, exists := r.environments[id]
	if !exists {
		return fmt.Errorf("environment not found: %s", id)
	}

	env.Status = status
	return nil
}

// Count returns the number of registered environments (read operation)
func (r *Registry) Count() int {
	r.mu.RLock()
	defer r.mu.RUnlock()
	
	return len(r.environments)
}