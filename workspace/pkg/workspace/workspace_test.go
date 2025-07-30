package workspace

import (
	"testing"
)

func TestNewWorkspace(t *testing.T) {
	config := &Config{
		BaseImage: "ubuntu:22.04",
		WorkDir:   "/app",
	}
	
	ws := NewWorkspace("test-workspace", config)
	
	// Verify fields
	if ws.Name != "test-workspace" {
		t.Errorf("expected name 'test-workspace', got %s", ws.Name)
	}
	
	if ws.BranchName != "workspace-test-workspace" {
		t.Errorf("expected branch name 'workspace-test-workspace', got %s", ws.BranchName)
	}
	
	if ws.BaseImage != "ubuntu:22.04" {
		t.Errorf("expected base image 'ubuntu:22.04', got %s", ws.BaseImage)
	}
	
	if ws.Status != StatusCreating {
		t.Errorf("expected status %s, got %s", StatusCreating, ws.Status)
	}
	
	// Verify timestamps
	if ws.CreatedAt.IsZero() {
		t.Error("created_at should not be zero")
	}
	
	if ws.UpdatedAt.IsZero() {
		t.Error("updated_at should not be zero")
	}
	
	// Verify ID format (YYYYMMDD-HHMMSS.microseconds)
	if len(ws.ID) != 22 {
		t.Errorf("expected ID length 22, got %d (ID: %s)", len(ws.ID), ws.ID)
	}
}

func TestWorkspaceStatus(t *testing.T) {
	tests := []struct {
		status   Status
		expected string
	}{
		{StatusCreating, "creating"},
		{StatusReady, "ready"},
		{StatusError, "error"},
		{StatusDeleting, "deleting"},
	}
	
	for _, tt := range tests {
		if string(tt.status) != tt.expected {
			t.Errorf("status %s does not match expected %s", tt.status, tt.expected)
		}
	}
}