package registry

import (
	"testing"
)

func TestNewRegistry(t *testing.T) {
	reg := NewRegistry()
	
	if reg == nil {
		t.Fatal("expected registry to be created")
	}
	
	if reg.Count() != 0 {
		t.Errorf("expected empty registry, got %d environments", reg.Count())
	}
}

func TestRegistry_Register(t *testing.T) {
	reg := NewRegistry()
	
	env := &Environment{
		ID:           "test-123",
		Name:         "test-env",
		Status:       "running",
		BranchName:   "main",
		WorktreePath: "/tmp/test",
		BaseImage:    "ubuntu:latest",
	}
	
	// Test successful registration
	if err := reg.Register(env); err != nil {
		t.Fatalf("failed to register environment: %v", err)
	}
	
	if reg.Count() != 1 {
		t.Errorf("expected 1 environment, got %d", reg.Count())
	}
	
	// Test nil environment
	if err := reg.Register(nil); err == nil {
		t.Error("expected error for nil environment")
	}
	
	// Test empty ID
	envNoID := &Environment{Name: "test"}
	if err := reg.Register(envNoID); err == nil {
		t.Error("expected error for environment without ID")
	}
}

func TestRegistry_Get(t *testing.T) {
	reg := NewRegistry()
	
	env := &Environment{
		ID:           "test-123",
		Name:         "test-env",
		Status:       "running",
		BranchName:   "main",
		WorktreePath: "/tmp/test",
		BaseImage:    "ubuntu:latest",
		CreatedAt:    "2024-01-01T00:00:00Z",
	}
	
	reg.Register(env)
	
	// Test successful get
	retrieved, err := reg.Get("test-123")
	if err != nil {
		t.Fatalf("failed to get environment: %v", err)
	}
	
	if retrieved.ID != env.ID {
		t.Errorf("expected ID %s, got %s", env.ID, retrieved.ID)
	}
	
	if retrieved.Name != env.Name {
		t.Errorf("expected name %s, got %s", env.Name, retrieved.Name)
	}
	
	// Test modification isolation
	retrieved.Name = "modified"
	retrieved2, _ := reg.Get("test-123")
	if retrieved2.Name != "test-env" {
		t.Error("external modification affected internal state")
	}
	
	// Test non-existent
	_, err = reg.Get("non-existent")
	if err == nil {
		t.Error("expected error for non-existent environment")
	}
	
	// Test empty ID
	_, err = reg.Get("")
	if err == nil {
		t.Error("expected error for empty ID")
	}
}

func TestRegistry_List(t *testing.T) {
	reg := NewRegistry()
	
	env1 := &Environment{
		ID:     "test-1",
		Name:   "env-1",
		Status: "running",
	}
	
	env2 := &Environment{
		ID:     "test-2",
		Name:   "env-2",
		Status: "stopped",
	}
	
	reg.Register(env1)
	reg.Register(env2)
	
	// Test list
	envs, err := reg.List()
	if err != nil {
		t.Fatalf("failed to list environments: %v", err)
	}
	
	if len(envs) != 2 {
		t.Errorf("expected 2 environments, got %d", len(envs))
	}
	
	// Test modification isolation
	envs[0].Name = "modified"
	envs2, _ := reg.List()
	for _, e := range envs2 {
		if e.Name == "modified" {
			t.Error("external modification affected internal state")
		}
	}
}

func TestRegistry_Deregister(t *testing.T) {
	reg := NewRegistry()
	
	env := &Environment{
		ID:   "test-123",
		Name: "test-env",
	}
	
	reg.Register(env)
	
	// Test successful deregister
	if err := reg.Deregister("test-123"); err != nil {
		t.Fatalf("failed to deregister environment: %v", err)
	}
	
	if reg.Count() != 0 {
		t.Errorf("expected 0 environments, got %d", reg.Count())
	}
	
	// Test non-existent
	if err := reg.Deregister("non-existent"); err == nil {
		t.Error("expected error for non-existent environment")
	}
	
	// Test empty ID
	if err := reg.Deregister(""); err == nil {
		t.Error("expected error for empty ID")
	}
}

func TestRegistry_UpdateStatus(t *testing.T) {
	reg := NewRegistry()
	
	env := &Environment{
		ID:     "test-123",
		Name:   "test-env",
		Status: "running",
	}
	
	reg.Register(env)
	
	// Test successful update
	if err := reg.UpdateStatus("test-123", "stopped"); err != nil {
		t.Fatalf("failed to update status: %v", err)
	}
	
	retrieved, _ := reg.Get("test-123")
	if retrieved.Status != "stopped" {
		t.Errorf("expected status 'stopped', got %s", retrieved.Status)
	}
	
	// Test non-existent
	if err := reg.UpdateStatus("non-existent", "running"); err == nil {
		t.Error("expected error for non-existent environment")
	}
}

func TestRegistry_EmptyList(t *testing.T) {
	reg := NewRegistry()
	
	envs, err := reg.List()
	if err != nil {
		t.Fatalf("failed to list environments: %v", err)
	}
	
	if len(envs) != 0 {
		t.Errorf("expected empty list, got %d environments", len(envs))
	}
}

func TestRegistry_Exists(t *testing.T) {
	reg := NewRegistry()
	
	env := &Environment{ID: "test-123"}
	reg.Register(env)
	
	// Test exists
	exists, err := reg.Exists("test-123")
	if err != nil {
		t.Fatalf("failed to check existence: %v", err)
	}
	if !exists {
		t.Error("expected environment to exist")
	}
	
	// Test not exists
	exists, err = reg.Exists("non-existent")
	if err != nil {
		t.Fatalf("failed to check existence: %v", err)
	}
	if exists {
		t.Error("expected environment not to exist")
	}
}