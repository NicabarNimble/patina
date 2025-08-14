package registry

import (
	"sync"
	"testing"
	"time"
)

// MockWorkspace simulates the workspace structure for testing
type MockWorkspace struct {
	id           string
	name         string
	status       string
	branchName   string
	worktreePath string
	baseImage    string
	createdAt    string
}

func (m *MockWorkspace) GetID() string           { return m.id }
func (m *MockWorkspace) GetName() string         { return m.name }
func (m *MockWorkspace) GetStatus() string       { return m.status }
func (m *MockWorkspace) GetBranchName() string   { return m.branchName }
func (m *MockWorkspace) GetWorktreePath() string { return m.worktreePath }
func (m *MockWorkspace) GetBaseImage() string    { return m.baseImage }
func (m *MockWorkspace) GetCreatedAt() string    { return m.createdAt }

func TestRegistry_Get(t *testing.T) {
	// Setup
	store := &sync.Map{}
	ws := &MockWorkspace{
		id:           "test-123",
		name:         "test-workspace",
		status:       "running",
		branchName:   "feature/test",
		worktreePath: "/tmp/test",
		baseImage:    "ubuntu:latest",
		createdAt:    time.Now().Format(time.RFC3339),
	}
	store.Store("test-123", ws)
	
	reg := NewRegistry(store)

	// Test successful get
	env, err := reg.Get("test-123")
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	if env.ID != "test-123" {
		t.Errorf("expected ID test-123, got %s", env.ID)
	}
	if env.Name != "test-workspace" {
		t.Errorf("expected name test-workspace, got %s", env.Name)
	}
	if env.Status != "running" {
		t.Errorf("expected status running, got %s", env.Status)
	}

	// Test not found
	_, err = reg.Get("non-existent")
	if err == nil {
		t.Error("expected error for non-existent environment")
	}
}

func TestRegistry_List(t *testing.T) {
	// Setup
	store := &sync.Map{}
	ws1 := &MockWorkspace{
		id:     "test-1",
		name:   "workspace-1",
		status: "running",
	}
	ws2 := &MockWorkspace{
		id:     "test-2",
		name:   "workspace-2",
		status: "stopped",
	}
	store.Store("test-1", ws1)
	store.Store("test-2", ws2)
	
	reg := NewRegistry(store)

	// Test list
	envs, err := reg.List()
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	if len(envs) != 2 {
		t.Errorf("expected 2 environments, got %d", len(envs))
	}

	// Verify both environments are present
	found := make(map[string]bool)
	for _, env := range envs {
		found[env.ID] = true
	}

	if !found["test-1"] || !found["test-2"] {
		t.Error("not all environments were returned")
	}
}

func TestRegistry_Exists(t *testing.T) {
	// Setup
	store := &sync.Map{}
	ws := &MockWorkspace{id: "test-123"}
	store.Store("test-123", ws)
	
	reg := NewRegistry(store)

	// Test exists
	exists, err := reg.Exists("test-123")
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if !exists {
		t.Error("expected environment to exist")
	}

	// Test not exists
	exists, err = reg.Exists("non-existent")
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if exists {
		t.Error("expected environment not to exist")
	}
}

func TestRegistry_EmptyList(t *testing.T) {
	// Setup with empty store
	store := &sync.Map{}
	reg := NewRegistry(store)

	envs, err := reg.List()
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	if len(envs) != 0 {
		t.Errorf("expected 0 environments, got %d", len(envs))
	}
}