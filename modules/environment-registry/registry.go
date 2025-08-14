package registry

import (
	"fmt"
	"sync"
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

// Registry provides read-only access to environments
type Registry struct {
	environments *sync.Map
}

// NewRegistry creates a new environment registry
func NewRegistry(environments *sync.Map) *Registry {
	return &Registry{
		environments: environments,
	}
}

// Get retrieves an environment by ID
func (r *Registry) Get(id string) (*Environment, error) {
	value, ok := r.environments.Load(id)
	if !ok {
		return nil, fmt.Errorf("environment not found: %s", id)
	}

	// Convert from workspace type to our Environment type
	// This is a temporary shim until we fully migrate
	if ws, ok := value.(interface {
		GetID() string
		GetName() string
		GetStatus() string
		GetBranchName() string
		GetWorktreePath() string
		GetBaseImage() string
		GetCreatedAt() string
	}); ok {
		return &Environment{
			ID:           ws.GetID(),
			Name:         ws.GetName(),
			Status:       ws.GetStatus(),
			BranchName:   ws.GetBranchName(),
			WorktreePath: ws.GetWorktreePath(),
			BaseImage:    ws.GetBaseImage(),
			CreatedAt:    ws.GetCreatedAt(),
		}, nil
	}

	return nil, fmt.Errorf("invalid environment data for id %s", id)
}

// List returns all active environments
func (r *Registry) List() ([]*Environment, error) {
	var environments []*Environment

	r.environments.Range(func(key, value interface{}) bool {
		if ws, ok := value.(interface {
			GetID() string
			GetName() string
			GetStatus() string
			GetBranchName() string
			GetWorktreePath() string
			GetBaseImage() string
			GetCreatedAt() string
		}); ok {
			environments = append(environments, &Environment{
				ID:           ws.GetID(),
				Name:         ws.GetName(),
				Status:       ws.GetStatus(),
				BranchName:   ws.GetBranchName(),
				WorktreePath: ws.GetWorktreePath(),
				BaseImage:    ws.GetBaseImage(),
				CreatedAt:    ws.GetCreatedAt(),
			})
		}
		return true
	})

	return environments, nil
}

// Exists checks if an environment exists
func (r *Registry) Exists(id string) (bool, error) {
	_, ok := r.environments.Load(id)
	return ok, nil
}