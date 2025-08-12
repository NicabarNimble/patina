package workspace

import (
	"context"
	"fmt"
	"log/slog"
	"sync"
	"time"

	"dagger.io/dagger"
)

// Manager handles workspace lifecycle operations
type Manager struct {
	dag        *dagger.Client
	workspaces sync.Map // Safe for concurrent access
	config     *ManagerConfig
	logger     *slog.Logger
	git        *GitIntegration
	closed     bool
	mu         sync.RWMutex // Protects closed state
}

// ManagerConfig holds configuration for the workspace manager
type ManagerConfig struct {
	ProjectRoot  string
	WorktreeRoot string // Directory for git worktrees
	DefaultImage string
}

// NewManager creates a new workspace manager
func NewManager(dag *dagger.Client, config *ManagerConfig, logger *slog.Logger) (*Manager, error) {
	if dag == nil {
		return nil, ErrNoDaggerClient
	}

	if config.DefaultImage == "" {
		config.DefaultImage = "ubuntu:latest"
	}

	m := &Manager{
		dag:    dag,
		config: config,
		logger: logger,
		closed: false,
	}

	// Git is required - fail fast if not available
	if config.ProjectRoot == "" {
		return nil, fmt.Errorf("PROJECT_ROOT is required")
	}

	if config.WorktreeRoot == "" {
		return nil, fmt.Errorf("WORKTREE_ROOT is required")
	}

	git, err := NewGitIntegration(config.ProjectRoot, config.WorktreeRoot)
	if err != nil {
		return nil, fmt.Errorf("failed to initialize git integration: %w", err)
	}

	m.git = git
	logger.Info("git integration initialized", "worktree_root", config.WorktreeRoot)

	return m, nil
}

// CreateWorkspace creates a new isolated workspace
func (m *Manager) CreateWorkspace(ctx context.Context, name string, config *Config) (*Workspace, error) {
	m.mu.RLock()
	if m.closed {
		m.mu.RUnlock()
		return nil, ErrManagerClosed
	}
	m.mu.RUnlock()

	// Validate input
	if name == "" {
		return nil, ErrInvalidConfig
	}

	if config == nil {
		config = &Config{
			BaseImage: m.config.DefaultImage,
		}
	}

	m.logger.Info("creating workspace", "name", name)

	// Create workspace instance
	ws := NewWorkspace(name, config)

	// Create git worktree
	worktreePath, err := m.git.CreateWorktree(ctx, ws.ID, ws.BranchName)
	if err != nil {
		return nil, fmt.Errorf("failed to create git worktree: %w", err)
	}

	ws.WorktreePath = worktreePath

	// Get base commit
	if commit, err := m.git.GetCurrentCommit(ctx, worktreePath); err == nil {
		ws.BaseCommit = commit
		ws.CurrentCommit = commit
	}

	m.logger.Info("created git worktree", "workspace", ws.ID, "branch", ws.BranchName, "path", worktreePath)

	// Save initial workspace state to git notes
	if err := m.git.SaveWorkspaceState(ctx, ws); err != nil {
		m.logger.Error("failed to save workspace state", "error", err)
		// Not fatal - continue without persistence
	}

	// Add log entry
	logEntry := fmt.Sprintf("Workspace created: %s (ID: %s)", ws.Name, ws.ID)
	if err := m.git.AddWorkspaceLogEntry(ctx, ws.WorktreePath, logEntry); err != nil {
		m.logger.Error("failed to add log entry", "error", err)
	}

	// Store workspace
	m.workspaces.Store(ws.ID, ws)

	// Create container in background
	go m.initializeContainer(context.Background(), ws)

	return ws, nil
}

// GetWorkspace retrieves a workspace by ID
func (m *Manager) GetWorkspace(id string) (*Workspace, error) {
	value, ok := m.workspaces.Load(id)
	if !ok {
		return nil, ErrWorkspaceNotFound
	}

	workspace, ok := value.(*Workspace)
	if !ok {
		return nil, fmt.Errorf("invalid workspace data for id %s", id)
	}

	return workspace, nil
}

// ListWorkspaces returns all active workspaces
func (m *Manager) ListWorkspaces() ([]*Workspace, error) {
	var workspaces []*Workspace

	m.workspaces.Range(func(key, value interface{}) bool {
		if ws, ok := value.(*Workspace); ok {
			workspaces = append(workspaces, ws)
		}
		return true
	})

	return workspaces, nil
}

// DeleteWorkspace removes a workspace and cleans up resources
func (m *Manager) DeleteWorkspace(ctx context.Context, id string) error {
	ws, err := m.GetWorkspace(id)
	if err != nil {
		return err
	}

	m.logger.Info("deleting workspace", "id", id, "name", ws.Name)

	// Update status
	ws.Status = StatusDeleting
	ws.UpdatedAt = time.Now()

	// Remove git worktree if present
	if m.git != nil && ws.WorktreePath != "" {
		m.logger.Info("removing git worktree", "workspace", id, "path", ws.WorktreePath)
		if err := m.git.RemoveWorktree(ctx, id); err != nil {
			m.logger.Error("failed to remove worktree", "error", err)
			// Continue with deletion even if worktree removal fails
		}
	}

	// Clean up container resources if Dagger is available
	if m.dag != nil && ws.ContainerID != "" {
		// Note: Dagger containers are ephemeral and cleaned up automatically
		// but we should still remove any cache volumes
		m.logger.Info("cleaning up workspace resources", "workspace", id)

		// Cache volumes are automatically cleaned up when no longer referenced
		// In a real implementation, we might want to explicitly remove them
	}

	// Remove from store
	m.workspaces.Delete(id)

	return nil
}

// initializeContainer sets up the container for a workspace
func (m *Manager) initializeContainer(ctx context.Context, ws *Workspace) {
	m.logger.Info("initializing container", "workspace", ws.ID)

	// Skip if no Dagger client (for testing)
	if m.dag == nil {
		m.logger.Warn("no Dagger client available, skipping container initialization")
		ws.Status = StatusError
		ws.UpdatedAt = time.Now()
		return
	}

	// Create container with proper setup
	container := m.dag.Container().
		From(ws.BaseImage).
		WithWorkdir("/workspace")

	// Install git if not present
	container = container.
		WithExec([]string{"sh", "-c", "which git || (apt-get update && apt-get install -y git)"})

	// Define common excludes for Dagger directory uploads
	excludes := []string{
		"target/",                   // Rust build artifacts
		"node_modules/",             // JS dependencies
		".git/",                     // Git history
		"dist/",                     // Build outputs
		"tmp/",                      // Temporary files
		"*.log",                     // Log files
		".dagger/",                  // Dagger's own cache
		"**/*.rs.bk",                // Rust backup files
		".DS_Store",                 // macOS files
		"__pycache__/",              // Python cache
		"*.pyc",                     // Python compiled files
		".pytest_cache/",            // Pytest cache
		".coverage",                 // Coverage files
		"htmlcov/",                  // Coverage HTML
		".mypy_cache/",              // MyPy cache
		".ruff_cache/",              // Ruff cache
		"venv/",                     // Python virtual env
		"env/",                      // Another venv name
		".env",                      // Environment files
		".venv/",                    // Yet another venv
		"build/",                    // General build dir
		".gradle/",                  // Gradle cache
		".idea/",                    // IntelliJ
		".vscode/",                  // VS Code
		"*.swp",                     // Vim swap files
		"*.swo",                     // Vim swap files
		"*.swn",                     // Vim swap files
		".terraform/",               // Terraform
		"*.tfstate*",                // Terraform state
		".next/",                    // Next.js
		"out/",                      // Next.js output
		".nuxt/",                    // Nuxt
		".output/",                  // Nuxt output
		".parcel-cache/",            // Parcel
		".turbo/",                   // Turborepo
		"coverage/",                 // General coverage
		".nyc_output/",              // NYC coverage
		"*.tsbuildinfo",             // TypeScript
		".angular/",                 // Angular
		".sass-cache/",              // Sass
		"*.class",                   // Java
		"*.jar",                     // Java archives
		"*.war",                     // Java web archives
		"target/",                   // Maven/Cargo
		"Cargo.lock",                // For libraries
		"package-lock.json",         // For libraries
		"yarn.lock",                 // For libraries
		"pnpm-lock.yaml",            // For libraries
		"poetry.lock",               // For libraries
		"Pipfile.lock",              // For libraries
		"composer.lock",             // For libraries
		"*.min.js",                  // Minified files
		"*.min.css",                 // Minified files
		"*.map",                     // Source maps
		".cache/",                   // General cache
		"*.tmp",                     // Temp files
		"*.temp",                    // Temp files
		"*.bak",                     // Backup files
		"*.backup",                  // Backup files
		"core",                      // Core dumps
		"core.*",                    // Core dumps
		"*.core",                    // Core dumps
		".patina/session.json",      // Patina sessions
		".claude/context/sessions/", // Claude sessions
		"layer/sessions/",           // Layer sessions
		"pipelines/target/",         // Dagger repo clone
		"workspace/target/",         // Go build artifacts
	}

	// Mount worktree or project directory
	if ws.WorktreePath != "" {
		// Use git worktree if available
		worktreeDir := m.dag.Host().Directory(ws.WorktreePath, dagger.HostDirectoryOpts{
			Exclude: excludes,
		})
		container = container.
			WithMountedDirectory("/workspace/project", worktreeDir).
			WithWorkdir("/workspace/project")

		m.logger.Info("mounted git worktree", "workspace", ws.ID, "path", ws.WorktreePath)
	} else if m.config.ProjectRoot != "" {
		// Fall back to project root
		projectDir := m.dag.Host().Directory(m.config.ProjectRoot, dagger.HostDirectoryOpts{
			Exclude: excludes,
		})
		container = container.
			WithMountedDirectory("/workspace/project", projectDir).
			WithWorkdir("/workspace/project")
	}

	// Initialize git config
	container = container.
		WithExec([]string{"git", "config", "--global", "user.email", "workspace@patina.dev"}).
		WithExec([]string{"git", "config", "--global", "user.name", "Patina Workspace"}).
		WithExec([]string{"git", "config", "--global", "init.defaultBranch", "main"}).
		WithExec([]string{"git", "config", "--global", "safe.directory", "/workspace/project"})

	// Create a cache volume for better performance
	cacheVolume := m.dag.CacheVolume("workspace-" + ws.ID)
	container = container.WithMountedCache("/workspace/.cache", cacheVolume)

	// Get container ID
	id, err := container.ID(ctx)
	if err != nil {
		m.logger.Error("failed to create container", "error", err)
		ws.Status = StatusError
		ws.UpdatedAt = time.Now()
		return
	}

	// Update workspace
	ws.ContainerID = string(id)
	ws.Status = StatusReady
	ws.UpdatedAt = time.Now()

	// Save updated state to git notes
	if err := m.git.SaveWorkspaceState(ctx, ws); err != nil {
		m.logger.Error("failed to save workspace state", "error", err)
	}

	// Add log entry
	logEntry := fmt.Sprintf("Container initialized for workspace %s", ws.ID)
	if err := m.git.AddWorkspaceLogEntry(ctx, ws.WorktreePath, logEntry); err != nil {
		m.logger.Error("failed to add log entry", "error", err)
	}

	m.logger.Info("container ready", "workspace", ws.ID, "container", id)
}

// LoadExistingWorkspaces loads workspace states from git notes on startup
func (m *Manager) LoadExistingWorkspaces(ctx context.Context) error {
	m.logger.Info("loading existing workspaces from git notes")

	workspaces, err := m.git.LoadAllWorkspaceStates(ctx)
	if err != nil {
		return fmt.Errorf("failed to load workspace states: %w", err)
	}

	for _, ws := range workspaces {
		m.logger.Info("loaded workspace", "id", ws.ID, "name", ws.Name, "status", ws.Status)

		// Store in memory
		m.workspaces.Store(ws.ID, ws)

		// If container was ready, try to reconnect
		if ws.Status == StatusReady && ws.ContainerID != "" {
			// Update status to indicate reconnection needed
			ws.Status = StatusCreating
			ws.UpdatedAt = time.Now()

			// Reinitialize container in background
			go m.initializeContainer(context.Background(), ws)
		}
	}

	m.logger.Info("loaded workspaces", "count", len(workspaces))
	return nil
}

// Close gracefully shuts down the manager
func (m *Manager) Close(ctx context.Context) error {
	m.mu.Lock()
	if m.closed {
		m.mu.Unlock()
		return nil
	}
	m.closed = true
	m.mu.Unlock()

	m.logger.Info("closing workspace manager")

	// Delete all workspaces
	workspaces, _ := m.ListWorkspaces()
	for _, ws := range workspaces {
		if err := m.DeleteWorkspace(ctx, ws.ID); err != nil {
			m.logger.Error("failed to delete workspace on close", "id", ws.ID, "error", err)
		}
	}

	return nil
}
