package workspace

import "context"

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
