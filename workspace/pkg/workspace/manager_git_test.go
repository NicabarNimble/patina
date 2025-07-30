package workspace

import (
	"context"
	"log/slog"
	"os"
	"os/exec"
	"path/filepath"
	"testing"
	"time"
)

func TestManager_GitIntegration(t *testing.T) {
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
	
	// Set up git config
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
	
	// Create manager with git integration
	config := &ManagerConfig{
		ProjectRoot:  repoDir,
		WorktreeRoot: worktreeRoot,
		DefaultImage: "ubuntu:latest",
	}
	
	logger := slog.New(slog.NewTextHandler(os.Stderr, nil))
	
	// Use mock Dagger client for testing
	manager, err := NewManager(nil, config, logger)
	if err == nil {
		t.Fatal("expected error with nil Dagger client")
	}
	
	// Create manager without Dagger for git-only testing
	manager = &Manager{
		config: config,
		logger: logger,
		closed: false,
	}
	
	// Initialize git integration manually
	git, err := NewGitIntegration(config.ProjectRoot, config.WorktreeRoot)
	if err != nil {
		t.Fatalf("failed to create git integration: %v", err)
	}
	manager.git = git
	
	ctx := context.Background()
	
	t.Run("create workspace with git worktree", func(t *testing.T) {
		ws, err := manager.CreateWorkspace(ctx, "test-feature", nil)
		if err != nil {
			t.Fatalf("failed to create workspace: %v", err)
		}
		
		// Verify workspace has git integration
		if ws.WorktreePath == "" {
			t.Error("expected worktree path to be set")
		}
		
		if ws.BranchName != "workspace-test-feature" {
			t.Errorf("expected branch name workspace-test-feature, got %s", ws.BranchName)
		}
		
		if ws.BaseCommit == "" {
			t.Error("expected base commit to be set")
		}
		
		// Verify worktree exists on disk
		if _, err := os.Stat(ws.WorktreePath); err != nil {
			t.Errorf("worktree path does not exist: %v", err)
		}
		
		// Make a change in the worktree
		testFile := filepath.Join(ws.WorktreePath, "test.txt")
		if err := os.WriteFile(testFile, []byte("test content"), 0644); err != nil {
			t.Fatalf("failed to write test file: %v", err)
		}
		
		// Verify we can retrieve the workspace
		retrieved, err := manager.GetWorkspace(ws.ID)
		if err != nil {
			t.Fatalf("failed to get workspace: %v", err)
		}
		
		if retrieved.ID != ws.ID {
			t.Errorf("retrieved workspace ID mismatch")
		}
	})
	
	t.Run("list workspaces", func(t *testing.T) {
		// Create another workspace
		ws2, err := manager.CreateWorkspace(ctx, "another-feature", nil)
		if err != nil {
			t.Fatalf("failed to create second workspace: %v", err)
		}
		
		workspaces, err := manager.ListWorkspaces()
		if err != nil {
			t.Fatalf("failed to list workspaces: %v", err)
		}
		
		if len(workspaces) < 2 {
			t.Errorf("expected at least 2 workspaces, got %d", len(workspaces))
		}
		
		// Verify both workspaces are in the list
		foundFirst := false
		foundSecond := false
		for _, ws := range workspaces {
			if ws.Name == "test-feature" {
				foundFirst = true
			}
			if ws.Name == "another-feature" {
				foundSecond = true
			}
		}
		
		if !foundFirst || !foundSecond {
			t.Error("not all workspaces found in list")
		}
		
		// Clean up
		if err := manager.DeleteWorkspace(ctx, ws2.ID); err != nil {
			t.Errorf("failed to delete workspace: %v", err)
		}
	})
	
	t.Run("delete workspace removes worktree", func(t *testing.T) {
		ws, err := manager.CreateWorkspace(ctx, "to-delete", nil)
		if err != nil {
			t.Fatalf("failed to create workspace: %v", err)
		}
		
		worktreePath := ws.WorktreePath
		
		// Verify worktree exists
		if _, err := os.Stat(worktreePath); err != nil {
			t.Fatalf("worktree should exist before deletion")
		}
		
		// Delete workspace
		if err := manager.DeleteWorkspace(ctx, ws.ID); err != nil {
			t.Fatalf("failed to delete workspace: %v", err)
		}
		
		// Verify worktree is removed
		if _, err := os.Stat(worktreePath); !os.IsNotExist(err) {
			t.Error("worktree should be removed after deletion")
		}
		
		// Verify workspace is removed from manager
		if _, err := manager.GetWorkspace(ws.ID); err != ErrWorkspaceNotFound {
			t.Error("workspace should not be found after deletion")
		}
	})
	
	t.Run("close manager cleans up all workspaces", func(t *testing.T) {
		// Create multiple workspaces
		ws1, _ := manager.CreateWorkspace(ctx, "cleanup-1", nil)
		ws2, _ := manager.CreateWorkspace(ctx, "cleanup-2", nil)
		
		// Close manager
		if err := manager.Close(ctx); err != nil {
			t.Errorf("failed to close manager: %v", err)
		}
		
		// Verify all workspaces are deleted
		time.Sleep(100 * time.Millisecond) // Give it time to clean up
		
		if _, err := os.Stat(ws1.WorktreePath); !os.IsNotExist(err) {
			t.Error("worktree 1 should be removed after close")
		}
		
		if _, err := os.Stat(ws2.WorktreePath); !os.IsNotExist(err) {
			t.Error("worktree 2 should be removed after close")
		}
	})
}