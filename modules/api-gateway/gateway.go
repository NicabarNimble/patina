package gateway

import (
	"context"
	"fmt"
	"sync"

	"dagger.io/dagger"
	executor "github.com/your-org/patina/modules/code-executor"
	provider "github.com/your-org/patina/modules/environment-provider"
	registry "github.com/your-org/patina/modules/environment-registry"
	gitmanager "github.com/your-org/patina/modules/git-manager"
)

// Gateway coordinates the modular workspace system
type Gateway struct {
	provider *provider.Provider
	registry *registry.Registry
	executor *executor.Executor
	git      *gitmanager.Manager
	
	// Container storage (registry owns environment data)
	containers   map[string]*dagger.Container
	mu           sync.RWMutex
}

// Config for gateway initialization
type Config struct {
	RepoPath     string
	WorktreeRoot string
}

// New creates a new API gateway
func New(client *dagger.Client, config *Config) (*Gateway, error) {
	gitMgr, err := gitmanager.NewManager(config.RepoPath, config.WorktreeRoot)
	if err != nil {
		return nil, fmt.Errorf("failed to create git manager: %w", err)
	}

	return &Gateway{
		provider:     provider.New(client),
		registry:     registry.NewRegistry(),
		executor:     executor.New(client),
		git:          gitMgr,
		containers:   make(map[string]*dagger.Container),
	}, nil
}

// CreateWorkspace creates a new workspace using the modules
func (g *Gateway) CreateWorkspace(ctx context.Context, name, branch string) (string, error) {
	// Create git worktree
	worktreePath, err := g.git.CreateWorktree(ctx, name, branch)
	if err != nil {
		return "", fmt.Errorf("failed to create worktree: %w", err)
	}

	// Create environment with copied worktree (writable)
	env, err := g.provider.Create(ctx, &provider.Config{
		Name:      name,
		BaseImage: "ubuntu:latest",
		Copies: map[string]string{
			worktreePath: "/workspace/project",
		},
	})
	if err != nil {
		// Clean up worktree on failure
		g.git.RemoveWorktree(ctx, name)
		return "", fmt.Errorf("failed to create environment: %w", err)
	}

	// Register environment in the registry
	err = g.registry.Register(&registry.Environment{
		ID:           env.ID,
		Name:         env.Name,
		Status:       "ready",
		BranchName:   branch,
		WorktreePath: worktreePath,
		BaseImage:    env.Config.BaseImage,
		CreatedAt:    env.CreatedAt.Format("2006-01-02T15:04:05Z07:00"),
	})
	if err != nil {
		// Clean up on failure
		g.git.RemoveWorktree(ctx, name)
		return "", fmt.Errorf("failed to register environment: %w", err)
	}

	// Store container reference
	g.mu.Lock()
	g.containers[env.ID] = env.Container
	g.mu.Unlock()

	return env.ID, nil
}

// Execute runs a command in a workspace
func (g *Gateway) Execute(ctx context.Context, workspaceID string, command []string) (*executor.Result, error) {
	// Get container
	g.mu.RLock()
	container, ok := g.containers[workspaceID]
	g.mu.RUnlock()
	
	if !ok {
		return nil, fmt.Errorf("workspace not found: %s", workspaceID)
	}

	// Execute command
	return g.executor.ExecuteSimple(ctx, container, command...)
}

// GetWorkspace retrieves workspace info
func (g *Gateway) GetWorkspace(id string) (*registry.Environment, error) {
	return g.registry.Get(id)
}

// ListWorkspaces lists all workspaces
func (g *Gateway) ListWorkspaces() ([]*registry.Environment, error) {
	return g.registry.List()
}

// DeleteWorkspace removes a workspace and cleans up resources
func (g *Gateway) DeleteWorkspace(ctx context.Context, id string) error {
	// Check if exists
	exists, err := g.registry.Exists(id)
	if err != nil {
		return err
	}
	if !exists {
		return fmt.Errorf("workspace not found: %s", id)
	}

	// Deregister from registry
	if err := g.registry.Deregister(id); err != nil {
		return fmt.Errorf("failed to deregister environment: %w", err)
	}
	
	// Remove container reference
	g.mu.Lock()
	delete(g.containers, id)
	g.mu.Unlock()

	// Remove git worktree
	if err := g.git.RemoveWorktree(ctx, id); err != nil {
		// Log but don't fail - worktree might already be gone
		fmt.Printf("warning: failed to remove worktree: %v\n", err)
	}

	return nil
}

// GetGitStatus returns git status for a workspace
func (g *Gateway) GetGitStatus(ctx context.Context, workspaceID string) (*gitmanager.Status, error) {
	env, err := g.registry.Get(workspaceID)
	if err != nil {
		return nil, err
	}
	
	return g.git.GetStatus(ctx, env.WorktreePath)
}

// CreateBranch creates a new branch in a workspace
func (g *Gateway) CreateBranch(ctx context.Context, workspaceID, branchName string) error {
	env, err := g.registry.Get(workspaceID)
	if err != nil {
		return err
	}
	
	return g.git.CreateBranch(ctx, env.WorktreePath, branchName)
}

// CommitChanges commits changes in a workspace
func (g *Gateway) CommitChanges(ctx context.Context, workspaceID, message, author, email string) error {
	env, err := g.registry.Get(workspaceID)
	if err != nil {
		return err
	}
	
	return g.git.Commit(ctx, env.WorktreePath, message, author, email)
}

// PushBranch pushes the current branch
func (g *Gateway) PushBranch(ctx context.Context, workspaceID string) error {
	env, err := g.registry.Get(workspaceID)
	if err != nil {
		return err
	}
	
	return g.git.Push(ctx, env.WorktreePath)
}

// Adapter to make our environment compatible with registry
type workspaceAdapter struct {
	id           string
	name         string
	status       string
	branchName   string
	worktreePath string
	baseImage    string
	createdAt    string
}

func (w *workspaceAdapter) GetID() string           { return w.id }
func (w *workspaceAdapter) GetName() string         { return w.name }
func (w *workspaceAdapter) GetStatus() string       { return w.status }
func (w *workspaceAdapter) GetBranchName() string   { return w.branchName }
func (w *workspaceAdapter) GetWorktreePath() string { return w.worktreePath }
func (w *workspaceAdapter) GetBaseImage() string    { return w.baseImage }
func (w *workspaceAdapter) GetCreatedAt() string    { return w.createdAt }