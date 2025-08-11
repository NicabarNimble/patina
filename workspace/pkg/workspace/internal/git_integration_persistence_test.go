package internal

import (
	"context"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"testing"
	"time"
)

func TestGitIntegration_WorkspacePersistence(t *testing.T) {
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

	// Create git integration
	gi, err := NewGitIntegration(repoDir, worktreeRoot)
	if err != nil {
		t.Fatalf("failed to create git integration: %v", err)
	}

	ctx := context.Background()

	t.Run("save and load workspace state", func(t *testing.T) {
		// Create a worktree
		worktreePath, err := gi.CreateWorktree(ctx, "test-ws-1", "workspace-test-1")
		if err != nil {
			t.Fatalf("failed to create worktree: %v", err)
		}

		// Create a test workspace
		ws := &Workspace{
			ID:            "test-ws-1",
			Name:          "Test Workspace",
			ContainerID:   "container-123",
			BranchName:    "workspace-test-1",
			BaseImage:     "ubuntu:latest",
			CreatedAt:     time.Now(),
			UpdatedAt:     time.Now(),
			Status:        StatusReady,
			WorktreePath:  worktreePath,
			BaseCommit:    "abc123",
			CurrentCommit: "def456",
			Metadata: map[string]string{
				"test": "value",
			},
		}

		// Save workspace state
		if err := gi.SaveWorkspaceState(ctx, ws); err != nil {
			t.Fatalf("failed to save workspace state: %v", err)
		}

		// Load workspace state
		loaded, err := gi.LoadWorkspaceState(ctx, worktreePath)
		if err != nil {
			t.Fatalf("failed to load workspace state: %v", err)
		}

		// Verify loaded state matches
		if loaded.ID != ws.ID {
			t.Errorf("ID mismatch: got %s, want %s", loaded.ID, ws.ID)
		}
		if loaded.Name != ws.Name {
			t.Errorf("Name mismatch: got %s, want %s", loaded.Name, ws.Name)
		}
		if loaded.ContainerID != ws.ContainerID {
			t.Errorf("ContainerID mismatch: got %s, want %s", loaded.ContainerID, ws.ContainerID)
		}
		if loaded.Status != ws.Status {
			t.Errorf("Status mismatch: got %s, want %s", loaded.Status, ws.Status)
		}
		if loaded.Metadata["test"] != "value" {
			t.Errorf("Metadata mismatch: got %v", loaded.Metadata)
		}
	})

	t.Run("load all workspace states", func(t *testing.T) {
		// Create another worktree
		worktreePath2, err := gi.CreateWorktree(ctx, "test-ws-2", "workspace-test-2")
		if err != nil {
			t.Fatalf("failed to create second worktree: %v", err)
		}

		// Create another workspace
		ws2 := &Workspace{
			ID:           "test-ws-2",
			Name:         "Test Workspace 2",
			BranchName:   "workspace-test-2",
			BaseImage:    "ubuntu:latest",
			CreatedAt:    time.Now(),
			UpdatedAt:    time.Now(),
			Status:       StatusCreating,
			WorktreePath: worktreePath2,
		}

		// Save second workspace state
		if err := gi.SaveWorkspaceState(ctx, ws2); err != nil {
			t.Fatalf("failed to save second workspace state: %v", err)
		}

		// Load all workspace states
		workspaces, err := gi.LoadAllWorkspaceStates(ctx)
		if err != nil {
			t.Fatalf("failed to load all workspace states: %v", err)
		}

		// Debug: print what we found
		t.Logf("Found %d workspaces", len(workspaces))
		for _, ws := range workspaces {
			t.Logf("  - ID: %s, Name: %s", ws.ID, ws.Name)
		}

		// Should have 2 workspaces
		if len(workspaces) != 2 {
			t.Errorf("expected 2 workspaces, got %d", len(workspaces))
		}

		// Verify both workspaces are loaded
		foundWs1 := false
		foundWs2 := false
		for _, ws := range workspaces {
			if ws.ID == "test-ws-1" {
				foundWs1 = true
			}
			if ws.ID == "test-ws-2" {
				foundWs2 = true
			}
		}

		if !foundWs1 || !foundWs2 {
			t.Error("not all workspaces were loaded")
		}
	})

	t.Run("add workspace log entries", func(t *testing.T) {
		// Get a worktree path
		worktrees, err := gi.ListWorktrees(ctx)
		if err != nil || len(worktrees) == 0 {
			t.Skip("no worktrees available")
		}

		worktreePath := worktrees[1].Path // Skip main worktree

		// Add log entries
		entry1 := "Test log entry 1"
		entry2 := "Test log entry 2"

		if err := gi.AddWorkspaceLogEntry(ctx, worktreePath, entry1); err != nil {
			t.Fatalf("failed to add first log entry: %v", err)
		}

		if err := gi.AddWorkspaceLogEntry(ctx, worktreePath, entry2); err != nil {
			t.Fatalf("failed to add second log entry: %v", err)
		}

		// Verify log entries exist (git notes show)
		cmd := exec.CommandContext(ctx, "git", "-C", worktreePath,
			"notes", "--ref", gitNotesLogRef, "show")
		output, err := cmd.Output()
		if err != nil {
			t.Fatalf("failed to get log notes: %v", err)
		}

		logContent := string(output)
		if logContent == "" {
			t.Error("log entries not found")
		}

		// Both entries should be in the log
		if !strings.Contains(logContent, entry1) || !strings.Contains(logContent, entry2) {
			t.Errorf("log entries missing: got %s", logContent)
		}
	})

	t.Run("handle missing workspace state", func(t *testing.T) {
		// Create a worktree without state
		worktreePath, err := gi.CreateWorktree(ctx, "test-ws-no-state", "workspace-no-state")
		if err != nil {
			t.Fatalf("failed to create worktree: %v", err)
		}

		// Try to load non-existent state
		ws, err := gi.LoadWorkspaceState(ctx, worktreePath)
		if err != ErrWorkspaceNotFound {
			t.Errorf("expected ErrWorkspaceNotFound, got %v", err)
		}
		if ws != nil {
			t.Errorf("expected nil workspace, got %v", ws)
		}
	})
}
