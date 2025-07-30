package workspace

import (
	"time"
)

// Status represents the current state of a workspace
type Status string

const (
	StatusCreating Status = "creating"
	StatusReady    Status = "ready"
	StatusError    Status = "error"
	StatusDeleting Status = "deleting"
)

// Workspace represents an isolated development environment
type Workspace struct {
	ID          string            `json:"id"`
	Name        string            `json:"name"`
	ContainerID string            `json:"container_id"`
	BranchName  string            `json:"branch_name"`
	BaseImage   string            `json:"base_image"`
	CreatedAt   time.Time         `json:"created_at"`
	UpdatedAt   time.Time         `json:"updated_at"`
	Status      Status            `json:"status"`
	Metadata    map[string]string `json:"metadata,omitempty"`

	// Git integration fields
	WorktreePath  string `json:"worktree_path,omitempty"`
	BaseCommit    string `json:"base_commit,omitempty"`
	CurrentCommit string `json:"current_commit,omitempty"`
}

// Config holds configuration for workspace creation
type Config struct {
	BaseImage   string            `json:"base_image,omitempty"`
	WorkDir     string            `json:"work_dir,omitempty"`
	GitRemote   string            `json:"git_remote,omitempty"`
	Environment map[string]string `json:"environment,omitempty"`
}

// NewWorkspace creates a new workspace instance
func NewWorkspace(name string, config *Config) *Workspace {
	now := time.Now()
	return &Workspace{
		ID:         generateID(),
		Name:       name,
		BranchName: "workspace-" + name,
		BaseImage:  config.BaseImage,
		CreatedAt:  now,
		UpdatedAt:  now,
		Status:     StatusCreating,
		Metadata:   make(map[string]string),
	}
}

// generateID creates a unique workspace ID
func generateID() string {
	// Use timestamp with nanoseconds for uniqueness
	return time.Now().Format("20060102-150405.000000")
}
