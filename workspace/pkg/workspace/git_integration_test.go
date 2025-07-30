package workspace

import (
	"context"
	"os"
	"os/exec"
	"path/filepath"
	"testing"
)

func TestGitIntegration_CreateWorktree(t *testing.T) {
	// Skip if git is not available
	if _, err := exec.LookPath("git"); err != nil {
		t.Skip("git not available")
	}

	// Create temporary directories
	tempDir := t.TempDir()
	repoDir := filepath.Join(tempDir, "repo")
	worktreeRoot := filepath.Join(tempDir, "worktrees")

	// Initialize a git repository
	if err := os.MkdirAll(repoDir, 0755); err != nil {
		t.Fatal(err)
	}

	// Initialize git repo
	initCmd := exec.Command("git", "init", repoDir)
	if err := initCmd.Run(); err != nil {
		t.Fatalf("failed to init git repo: %v", err)
	}

	// Set up git config for the test repo (must be done before commits)
	configEmailCmd := exec.Command("git", "-C", repoDir, "config", "user.email", "test@patina.dev")
	if err := configEmailCmd.Run(); err != nil {
		t.Fatalf("failed to set git email: %v", err)
	}

	configNameCmd := exec.Command("git", "-C", repoDir, "config", "user.name", "Test User")
	if err := configNameCmd.Run(); err != nil {
		t.Fatalf("failed to set git name: %v", err)
	}

	// Create an initial commit
	createFileCmd := exec.Command("sh", "-c", "cd "+repoDir+" && echo 'test' > README.md && git add . && git commit -m 'Initial commit'")
	if err := createFileCmd.Run(); err != nil {
		t.Fatalf("failed to create initial commit: %v", err)
	}

	// Create git integration
	gi, err := NewGitIntegration(repoDir, worktreeRoot)
	if err != nil {
		t.Fatalf("failed to create git integration: %v", err)
	}

	ctx := context.Background()

	t.Run("create new worktree with new branch", func(t *testing.T) {
		worktreePath, err := gi.CreateWorktree(ctx, "test-workspace-1", "workspace-test-1")
		if err != nil {
			t.Fatalf("failed to create worktree: %v", err)
		}

		// Verify worktree exists
		if _, err := os.Stat(worktreePath); err != nil {
			t.Errorf("worktree path does not exist: %v", err)
		}

		// Verify branch name
		branchName, err := gi.GetBranchName(ctx, worktreePath)
		if err != nil {
			t.Fatalf("failed to get branch name: %v", err)
		}

		if branchName != "workspace-test-1" {
			t.Errorf("expected branch workspace-test-1, got %s", branchName)
		}
	})

	t.Run("create worktree with existing branch", func(t *testing.T) {
		// First create a branch
		createBranchCmd := exec.Command("git", "-C", repoDir, "branch", "existing-branch")
		if err := createBranchCmd.Run(); err != nil {
			t.Fatalf("failed to create branch: %v", err)
		}

		worktreePath, err := gi.CreateWorktree(ctx, "test-workspace-2", "existing-branch")
		if err != nil {
			t.Fatalf("failed to create worktree: %v", err)
		}

		// Verify branch name
		branchName, err := gi.GetBranchName(ctx, worktreePath)
		if err != nil {
			t.Fatalf("failed to get branch name: %v", err)
		}

		if branchName != "existing-branch" {
			t.Errorf("expected branch existing-branch, got %s", branchName)
		}
	})

	t.Run("remove worktree", func(t *testing.T) {
		// Create a worktree first
		worktreePath, err := gi.CreateWorktree(ctx, "test-workspace-remove", "workspace-remove")
		if err != nil {
			t.Fatalf("failed to create worktree: %v", err)
		}

		// Verify it exists
		if _, err := os.Stat(worktreePath); err != nil {
			t.Fatalf("worktree should exist before removal")
		}

		// Remove it
		if err := gi.RemoveWorktree(ctx, "test-workspace-remove"); err != nil {
			t.Fatalf("failed to remove worktree: %v", err)
		}

		// Verify it's gone
		if _, err := os.Stat(worktreePath); !os.IsNotExist(err) {
			t.Errorf("worktree should not exist after removal")
		}
	})

	t.Run("list worktrees", func(t *testing.T) {
		worktrees, err := gi.ListWorktrees(ctx)
		if err != nil {
			t.Fatalf("failed to list worktrees: %v", err)
		}

		// Should have main + test worktrees
		if len(worktrees) < 2 {
			t.Errorf("expected at least 2 worktrees, got %d", len(worktrees))
		}

		// Verify we can find our test worktree
		found := false
		for _, wt := range worktrees {
			if wt.Branch == "workspace-test-1" {
				found = true
				break
			}
		}

		if !found {
			t.Errorf("could not find test worktree in list")
		}
	})
}

func TestGitIntegration_NotAGitRepo(t *testing.T) {
	tempDir := t.TempDir()

	_, err := NewGitIntegration(tempDir, filepath.Join(tempDir, "worktrees"))
	if err == nil {
		t.Errorf("expected error for non-git directory")
	}
}
