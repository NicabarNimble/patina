package workspace

import (
	"context"
	"log/slog"
	"os"
	"os/exec"
	"path/filepath"
	"testing"

	"dagger.io/dagger"
)

// testDaggerClient returns a Dagger client for testing
// Returns nil if Dagger is not available
func testDaggerClient(t *testing.T) *dagger.Client {
	t.Helper()

	// Try to connect to Dagger
	ctx := context.Background()
	client, err := dagger.Connect(ctx, dagger.WithLogOutput(os.Stderr))
	if err != nil {
		return nil
	}

	return client
}

// setupTestGitRepo creates a test git repository
func setupTestGitRepo(t *testing.T) (repoDir, worktreeRoot string) {
	t.Helper()

	tempDir := t.TempDir()
	repoDir = filepath.Join(tempDir, "repo")
	worktreeRoot = filepath.Join(tempDir, "worktrees")

	// Initialize git repo
	if err := os.MkdirAll(repoDir, 0755); err != nil {
		t.Fatal(err)
	}

	initCmd := exec.Command("git", "init", repoDir)
	if err := initCmd.Run(); err != nil {
		t.Fatalf("failed to init git repo: %v", err)
	}

	// Set git config
	configEmailCmd := exec.Command("git", "-C", repoDir, "config", "user.email", "test@patina.dev")
	if err := configEmailCmd.Run(); err != nil {
		t.Fatalf("failed to set git email: %v", err)
	}

	configNameCmd := exec.Command("git", "-C", repoDir, "config", "user.name", "Test User")
	if err := configNameCmd.Run(); err != nil {
		t.Fatalf("failed to set git name: %v", err)
	}

	// Create initial commit
	createFileCmd := exec.Command("sh", "-c",
		"cd "+repoDir+" && echo 'test' > README.md && git add . && git commit -m 'Initial commit'")
	if err := createFileCmd.Run(); err != nil {
		t.Fatalf("failed to create initial commit: %v", err)
	}

	return repoDir, worktreeRoot
}

// mustNewTestManagerWithGit creates a test manager with real git integration
// For tests that don't need Dagger
func mustNewTestManagerWithGit(t *testing.T) *Manager {
	t.Helper()

	repoDir, worktreeRoot := setupTestGitRepo(t)

	config := &ManagerConfig{
		ProjectRoot:  repoDir,
		WorktreeRoot: worktreeRoot,
		DefaultImage: "ubuntu:latest",
	}

	logger := slog.New(slog.NewTextHandler(os.Stderr, nil))

	// Create without Dagger for unit tests
	git, err := NewGitIntegration(config.ProjectRoot, config.WorktreeRoot)
	if err != nil {
		t.Fatalf("failed to create git integration: %v", err)
	}

	m := &Manager{
		dag:    nil,
		config: config,
		logger: logger,
		git:    git,
		closed: false,
	}

	return m
}

// mustNewTestManagerWithDagger creates a test manager with both git and Dagger
// For integration tests
func mustNewTestManagerWithDagger(t *testing.T) *Manager {
	t.Helper()

	dag := testDaggerClient(t)
	if dag == nil {
		t.Skip("Dagger not available")
	}

	repoDir, worktreeRoot := setupTestGitRepo(t)

	config := &ManagerConfig{
		ProjectRoot:  repoDir,
		WorktreeRoot: worktreeRoot,
		DefaultImage: "ubuntu:latest",
	}

	logger := slog.New(slog.NewTextHandler(os.Stderr, nil))

	m, err := NewManager(dag, config, logger)
	if err != nil {
		t.Fatalf("failed to create manager: %v", err)
	}

	return m
}
