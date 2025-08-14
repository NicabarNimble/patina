package gitmanager

import (
	"context"
	"os"
	"os/exec"
	"path/filepath"
	"testing"
)

func setupTestRepo(t *testing.T) (string, func()) {
	// Create temp directory
	tmpDir, err := os.MkdirTemp("", "git-manager-test-*")
	if err != nil {
		t.Fatal(err)
	}

	// Initialize git repo
	cmd := exec.Command("git", "init")
	cmd.Dir = tmpDir
	if err := cmd.Run(); err != nil {
		os.RemoveAll(tmpDir)
		t.Fatal(err)
	}

	// Configure git
	exec.Command("git", "-C", tmpDir, "config", "user.email", "test@example.com").Run()
	exec.Command("git", "-C", tmpDir, "config", "user.name", "Test User").Run()

	// Create initial commit
	testFile := filepath.Join(tmpDir, "README.md")
	if err := os.WriteFile(testFile, []byte("# Test"), 0644); err != nil {
		os.RemoveAll(tmpDir)
		t.Fatal(err)
	}

	exec.Command("git", "-C", tmpDir, "add", ".").Run()
	exec.Command("git", "-C", tmpDir, "commit", "-m", "Initial commit").Run()

	cleanup := func() {
		os.RemoveAll(tmpDir)
	}

	return tmpDir, cleanup
}

func TestManager_CreateWorktree(t *testing.T) {
	repoPath, cleanup := setupTestRepo(t)
	defer cleanup()

	worktreeRoot := filepath.Join(repoPath, ".worktrees")
	manager, err := NewManager(repoPath, worktreeRoot)
	if err != nil {
		t.Fatal(err)
	}

	ctx := context.Background()

	// Test creating worktree with new branch
	path, err := manager.CreateWorktree(ctx, "test-1", "feature/test")
	if err != nil {
		t.Fatalf("failed to create worktree: %v", err)
	}

	// Verify worktree exists
	if _, err := os.Stat(path); os.IsNotExist(err) {
		t.Error("worktree directory not created")
	}

	// Verify it's a git worktree
	cmd := exec.Command("git", "-C", path, "status")
	if err := cmd.Run(); err != nil {
		t.Error("created path is not a valid git worktree")
	}
}

func TestManager_RemoveWorktree(t *testing.T) {
	repoPath, cleanup := setupTestRepo(t)
	defer cleanup()

	worktreeRoot := filepath.Join(repoPath, ".worktrees")
	manager, err := NewManager(repoPath, worktreeRoot)
	if err != nil {
		t.Fatal(err)
	}

	ctx := context.Background()

	// Create a worktree
	path, err := manager.CreateWorktree(ctx, "test-remove", "feature/remove")
	if err != nil {
		t.Fatal(err)
	}

	// Remove it
	if err := manager.RemoveWorktree(ctx, "test-remove"); err != nil {
		t.Fatalf("failed to remove worktree: %v", err)
	}

	// Verify it's gone
	if _, err := os.Stat(path); !os.IsNotExist(err) {
		t.Error("worktree directory still exists after removal")
	}
}

func TestManager_GetStatus(t *testing.T) {
	repoPath, cleanup := setupTestRepo(t)
	defer cleanup()

	worktreeRoot := filepath.Join(repoPath, ".worktrees")
	manager, err := NewManager(repoPath, worktreeRoot)
	if err != nil {
		t.Fatal(err)
	}

	ctx := context.Background()

	// Create worktree
	path, err := manager.CreateWorktree(ctx, "test-status", "feature/status")
	if err != nil {
		t.Fatal(err)
	}

	// Get clean status
	status, err := manager.GetStatus(ctx, path)
	if err != nil {
		t.Fatalf("failed to get status: %v", err)
	}

	if status.Branch != "feature/status" {
		t.Errorf("expected branch feature/status, got %s", status.Branch)
	}

	if !status.Clean {
		t.Error("expected clean status")
	}

	// Create untracked file
	untrackedFile := filepath.Join(path, "untracked.txt")
	if err := os.WriteFile(untrackedFile, []byte("test"), 0644); err != nil {
		t.Fatal(err)
	}

	// Get status with untracked
	status, err = manager.GetStatus(ctx, path)
	if err != nil {
		t.Fatal(err)
	}

	if status.Clean {
		t.Error("expected dirty status")
	}

	if len(status.Untracked) != 1 || status.Untracked[0] != "untracked.txt" {
		t.Errorf("expected untracked.txt, got %v", status.Untracked)
	}
}