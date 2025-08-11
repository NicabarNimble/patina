package internal

import (
	"context"
	"fmt"
	"log/slog"
	"testing"
	"time"
)

// Test helper functions following rqlite pattern
// Moved to test_helpers.go for shared use

// mustNewTestManager is now mustNewTestManagerWithGit in test_helpers.go

func Test_NewManager(t *testing.T) {
	// Test with nil client
	_, err := NewManager(nil, &ManagerConfig{}, slog.Default())
	if err != ErrNoDaggerClient {
		t.Errorf("expected ErrNoDaggerClient, got %v", err)
	}

	// Test with valid client (would need mock in real test)
	// For now, we'll skip this as it requires Dagger setup
}

func Test_Manager_CreateWorkspace(t *testing.T) {
	m := mustNewTestManagerWithGit(t)

	tests := []struct {
		name      string
		wsName    string
		config    *Config
		wantError bool
		errorIs   error
	}{
		{
			name:      "valid workspace",
			wsName:    "test-ws",
			config:    &Config{BaseImage: "ubuntu:22.04"},
			wantError: false,
		},
		{
			name:      "empty name",
			wsName:    "",
			config:    &Config{BaseImage: "ubuntu:22.04"},
			wantError: true,
			errorIs:   ErrInvalidConfig,
		},
		{
			name:      "nil config uses defaults",
			wsName:    "default-ws",
			config:    nil,
			wantError: false,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			ws, err := m.CreateWorkspace(context.Background(), tt.wsName, tt.config)

			if tt.wantError {
				if err == nil {
					t.Error("expected error but got none")
				}
				if tt.errorIs != nil && err != tt.errorIs {
					t.Errorf("expected error %v, got %v", tt.errorIs, err)
				}
				return
			}

			if err != nil {
				t.Errorf("unexpected error: %v", err)
				return
			}

			// Verify workspace properties
			if ws.Name != tt.wsName {
				t.Errorf("expected name %s, got %s", tt.wsName, ws.Name)
			}

			if ws.Status != StatusCreating {
				t.Errorf("expected status %s, got %s", StatusCreating, ws.Status)
			}

			// Verify workspace was stored
			stored, err := m.GetWorkspace(ws.ID)
			if err != nil {
				t.Errorf("failed to retrieve stored workspace: %v", err)
			}

			if stored.ID != ws.ID {
				t.Error("stored workspace ID doesn't match")
			}
		})
	}
}

func Test_Manager_GetWorkspace(t *testing.T) {
	m := mustNewTestManagerWithGit(t)

	// Create a workspace first
	ws, err := m.CreateWorkspace(context.Background(), "test", nil)
	if err != nil {
		t.Fatalf("failed to create test workspace: %v", err)
	}

	// Test getting existing workspace
	retrieved, err := m.GetWorkspace(ws.ID)
	if err != nil {
		t.Errorf("failed to get workspace: %v", err)
	}

	if retrieved.ID != ws.ID {
		t.Error("retrieved workspace ID doesn't match")
	}

	// Test getting non-existent workspace
	_, err = m.GetWorkspace("non-existent")
	if !IsNotFound(err) {
		t.Errorf("expected ErrWorkspaceNotFound, got %v", err)
	}
}

func Test_Manager_ListWorkspaces(t *testing.T) {
	m := mustNewTestManagerWithGit(t)

	// Initially empty
	workspaces, err := m.ListWorkspaces()
	if err != nil {
		t.Errorf("unexpected error: %v", err)
	}

	if len(workspaces) != 0 {
		t.Errorf("expected 0 workspaces, got %d", len(workspaces))
	}

	// Create some workspaces
	for i := 0; i < 3; i++ {
		name := fmt.Sprintf("test-%d", i)
		_, err := m.CreateWorkspace(context.Background(), name, nil)
		if err != nil {
			t.Fatalf("failed to create workspace %s: %v", name, err)
		}
		time.Sleep(10 * time.Millisecond) // Ensure unique IDs
	}

	// List again
	workspaces, err = m.ListWorkspaces()
	if err != nil {
		t.Errorf("unexpected error: %v", err)
	}

	if len(workspaces) != 3 {
		t.Errorf("expected 3 workspaces, got %d", len(workspaces))
	}
}

func Test_Manager_DeleteWorkspace(t *testing.T) {
	m := mustNewTestManagerWithGit(t)

	// Create a workspace
	ws, err := m.CreateWorkspace(context.Background(), "to-delete", nil)
	if err != nil {
		t.Fatalf("failed to create workspace: %v", err)
	}

	// Delete it
	err = m.DeleteWorkspace(context.Background(), ws.ID)
	if err != nil {
		t.Errorf("failed to delete workspace: %v", err)
	}

	// Verify it's gone
	_, err = m.GetWorkspace(ws.ID)
	if !IsNotFound(err) {
		t.Error("workspace should not exist after deletion")
	}

	// Delete non-existent should error
	err = m.DeleteWorkspace(context.Background(), "non-existent")
	if !IsNotFound(err) {
		t.Errorf("expected ErrWorkspaceNotFound, got %v", err)
	}
}

func Test_Manager_Close(t *testing.T) {
	m := mustNewTestManagerWithGit(t)

	// Create some workspaces
	for i := 0; i < 2; i++ {
		name := fmt.Sprintf("test-%d", i)
		_, err := m.CreateWorkspace(context.Background(), name, nil)
		if err != nil {
			t.Fatalf("failed to create workspace %s: %v", name, err)
		}
	}

	// Close manager
	err := m.Close(context.Background())
	if err != nil {
		t.Errorf("failed to close manager: %v", err)
	}

	// Verify closed state
	if !m.closed {
		t.Error("manager should be marked as closed")
	}

	// Operations should fail
	_, err = m.CreateWorkspace(context.Background(), "after-close", nil)
	if err != ErrManagerClosed {
		t.Errorf("expected ErrManagerClosed, got %v", err)
	}

	// Close again should be idempotent
	err = m.Close(context.Background())
	if err != nil {
		t.Error("closing twice should not error")
	}
}

// Test error helper functions
func Test_ErrorHelpers(t *testing.T) {
	// Test IsNotFound
	if !IsNotFound(ErrWorkspaceNotFound) {
		t.Error("IsNotFound should return true for ErrWorkspaceNotFound")
	}

	if IsNotFound(ErrContainerNotReady) {
		t.Error("IsNotFound should return false for other errors")
	}

	// Test IsNotReady
	if !IsNotReady(ErrContainerNotReady) {
		t.Error("IsNotReady should return true for ErrContainerNotReady")
	}

	if IsNotReady(ErrWorkspaceNotFound) {
		t.Error("IsNotReady should return false for other errors")
	}
}

// Table-driven test example
func Test_Manager_Execute_Validation(t *testing.T) {
	m := mustNewTestManagerWithGit(t)

	// Close manager for closed state test
	closedManager := mustNewTestManagerWithGit(t)
	closedManager.Close(context.Background())

	tests := []struct {
		name        string
		manager     *Manager
		workspaceID string
		options     *ExecOptions
		wantError   error
	}{
		{
			name:        "closed manager",
			manager:     closedManager,
			workspaceID: "any",
			options:     &ExecOptions{Command: []string{"ls"}},
			wantError:   ErrManagerClosed,
		},
		{
			name:        "workspace not found",
			manager:     m,
			workspaceID: "non-existent",
			options:     &ExecOptions{Command: []string{"ls"}},
			wantError:   ErrWorkspaceNotFound,
		},
		{
			name:        "nil options",
			manager:     m,
			workspaceID: "non-existent",
			options:     nil,
			wantError:   ErrWorkspaceNotFound,
		},
		{
			name:        "empty command",
			manager:     m,
			workspaceID: "non-existent",
			options:     &ExecOptions{Command: []string{}},
			wantError:   ErrWorkspaceNotFound,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			_, err := tt.manager.Execute(context.Background(), tt.workspaceID, tt.options)
			if err != tt.wantError {
				t.Errorf("expected error %v, got %v", tt.wantError, err)
			}
		})
	}
}
