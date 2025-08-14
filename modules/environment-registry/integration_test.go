// +build integration

package registry

import (
	"sync"
	"testing"
)

// This test verifies the registry can work with workspace's sync.Map
// without creating circular dependencies
func TestIntegration_WorkspaceCompatibility(t *testing.T) {
	// This simulates how the workspace manager would use the registry
	workspaceStore := &sync.Map{}
	
	// Add some mock workspaces
	ws1 := &MockWorkspace{
		id:     "ws-1",
		name:   "dev-env",
		status: "running",
	}
	ws2 := &MockWorkspace{
		id:     "ws-2", 
		name:   "test-env",
		status: "stopped",
	}
	
	workspaceStore.Store("ws-1", ws1)
	workspaceStore.Store("ws-2", ws2)
	
	// Create registry with the same store
	reg := NewRegistry(workspaceStore)
	
	// Verify registry can read the data
	envs, err := reg.List()
	if err != nil {
		t.Fatalf("failed to list environments: %v", err)
	}
	
	if len(envs) != 2 {
		t.Errorf("expected 2 environments, got %d", len(envs))
	}
	
	// Verify Get works
	env, err := reg.Get("ws-1")
	if err != nil {
		t.Fatalf("failed to get environment: %v", err)
	}
	
	if env.Name != "dev-env" {
		t.Errorf("expected name dev-env, got %s", env.Name)
	}
}