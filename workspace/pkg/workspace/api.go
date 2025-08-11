// Package workspace provides container-based development environments using Dagger.
// It manages isolated workspaces with git worktree integration for parallel development.
package workspace

import (
	"context"
	"errors"
	"log/slog"
	"time"

	"dagger.io/dagger"
)

// Common errors returned by workspace operations
var (
	ErrNotFound       = errors.New("workspace not found")
	ErrNotReady       = errors.New("workspace not ready")
	ErrAlreadyExists  = errors.New("workspace already exists")
	ErrInvalidConfig  = errors.New("invalid configuration")
	ErrNoDaggerClient = errors.New("no dagger client provided")
	ErrClosed         = errors.New("manager is closed")
)

// WorkspaceManager defines the interface for workspace operations
type WorkspaceManager interface {
	CreateWorkspace(ctx context.Context, name string, config *Config) (*Workspace, error)
	GetWorkspace(id string) (*Workspace, error)
	ListWorkspaces() ([]*Workspace, error)
	DeleteWorkspace(ctx context.Context, id string) error
	Execute(ctx context.Context, workspaceID string, opts *ExecOptions) (*ExecResult, error)
	Close(ctx context.Context) error

	// Git operations
	CreateBranch(ctx context.Context, workspaceID, branchName string) error
	GetGitStatus(ctx context.Context, workspaceID string) (*GitStatus, error)
	CommitChanges(ctx context.Context, workspaceID string, opts *GitOptions) error
	PushBranch(ctx context.Context, workspaceID string) error
}

// Config holds configuration for creating a workspace
type Config struct {
	BaseImage string            `json:"base_image,omitempty"`
	Env       map[string]string `json:"env,omitempty"`
}

// Workspace represents an isolated development environment
type Workspace struct {
	ID           string            `json:"id"`
	Name         string            `json:"name"`
	Status       string            `json:"status"`
	BaseImage    string            `json:"base_image"`
	CreatedAt    time.Time         `json:"created_at"`
	BranchName   string            `json:"branch_name,omitempty"`
	WorktreePath string            `json:"worktree_path,omitempty"`
	Env          map[string]string `json:"env,omitempty"`
}

// ExecOptions configures command execution in a workspace
type ExecOptions struct {
	Command []string          `json:"command"`
	WorkDir string            `json:"work_dir,omitempty"`
	Env     map[string]string `json:"env,omitempty"`
	Timeout time.Duration     `json:"timeout,omitempty"`
}

// ExecResult contains the output from command execution
type ExecResult struct {
	Stdout   string `json:"stdout"`
	Stderr   string `json:"stderr"`
	ExitCode int    `json:"exit_code"`
}

// GitOptions configures git operations
type GitOptions struct {
	Message string `json:"message,omitempty"`
	Author  string `json:"author,omitempty"`
	Email   string `json:"email,omitempty"`
}

// GitStatus represents the git status of a workspace
type GitStatus struct {
	Branch     string   `json:"branch"`
	Modified   []string `json:"modified,omitempty"`
	Untracked  []string `json:"untracked,omitempty"`
	HasChanges bool     `json:"has_changes"`
}

// ManagerConfig holds configuration for the workspace manager
type ManagerConfig struct {
	ProjectRoot  string
	WorktreeRoot string // Directory for git worktrees
	DefaultImage string
}

// NewManager creates a new workspace manager instance
func NewManager(dag *dagger.Client, config *ManagerConfig, logger *slog.Logger) (WorkspaceManager, error) {
	return internal.NewManagerImpl(dag, config, logger)
}

// Error checking helpers

// IsNotFound returns true if the error is a "not found" error
func IsNotFound(err error) bool {
	return errors.Is(err, ErrNotFound)
}

// IsNotReady returns true if the error is a "not ready" error
func IsNotReady(err error) bool {
	return errors.Is(err, ErrNotReady)
}

// IsAlreadyExists returns true if the error indicates a workspace already exists
func IsAlreadyExists(err error) bool {
	return errors.Is(err, ErrAlreadyExists)
}