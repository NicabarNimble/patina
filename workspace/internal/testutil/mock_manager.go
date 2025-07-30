package testutil

import (
	"context"
	"log/slog"

	"github.com/patina/workspace/pkg/workspace"
)

// MockManager is a test implementation of workspace manager
type MockManager struct {
	Workspaces map[string]*workspace.Workspace
	CreateErr  error
	GetErr     error
	ListErr    error
	DeleteErr  error
	ExecuteErr error
	logger     *slog.Logger
}

// NewMockManager creates a new mock manager
func NewMockManager() *MockManager {
	return &MockManager{
		Workspaces: make(map[string]*workspace.Workspace),
		logger:     slog.Default(),
	}
}

// CreateWorkspace mock implementation
func (m *MockManager) CreateWorkspace(ctx context.Context, name string, config *workspace.Config) (*workspace.Workspace, error) {
	if m.CreateErr != nil {
		return nil, m.CreateErr
	}

	ws := &workspace.Workspace{
		ID:     "test-" + name,
		Name:   name,
		Status: workspace.StatusCreating,
	}

	m.Workspaces[ws.ID] = ws
	return ws, nil
}

// GetWorkspace mock implementation
func (m *MockManager) GetWorkspace(id string) (*workspace.Workspace, error) {
	if m.GetErr != nil {
		return nil, m.GetErr
	}

	ws, ok := m.Workspaces[id]
	if !ok {
		return nil, workspace.ErrWorkspaceNotFound
	}

	return ws, nil
}

// ListWorkspaces mock implementation
func (m *MockManager) ListWorkspaces() ([]*workspace.Workspace, error) {
	if m.ListErr != nil {
		return nil, m.ListErr
	}

	var list []*workspace.Workspace
	for _, ws := range m.Workspaces {
		list = append(list, ws)
	}

	return list, nil
}

// DeleteWorkspace mock implementation
func (m *MockManager) DeleteWorkspace(ctx context.Context, id string) error {
	if m.DeleteErr != nil {
		return m.DeleteErr
	}

	if _, ok := m.Workspaces[id]; !ok {
		return workspace.ErrWorkspaceNotFound
	}

	delete(m.Workspaces, id)
	return nil
}

// Execute mock implementation
func (m *MockManager) Execute(ctx context.Context, workspaceID string, opts *workspace.ExecOptions) (*workspace.ExecResult, error) {
	if m.ExecuteErr != nil {
		return nil, m.ExecuteErr
	}

	ws, err := m.GetWorkspace(workspaceID)
	if err != nil {
		return nil, err
	}

	if ws.Status != workspace.StatusReady {
		return nil, workspace.ErrContainerNotReady
	}

	return &workspace.ExecResult{
		ExitCode: 0,
		Stdout:   "mock output",
		Stderr:   "",
	}, nil
}

// Close mock implementation
func (m *MockManager) Close(ctx context.Context) error {
	m.Workspaces = make(map[string]*workspace.Workspace)
	return nil
}

// Git operation mocks

// CreateBranch mock implementation
func (m *MockManager) CreateBranch(ctx context.Context, workspaceID, branchName string) error {
	ws, err := m.GetWorkspace(workspaceID)
	if err != nil {
		return err
	}

	ws.BranchName = branchName
	return nil
}

// GetGitStatus mock implementation
func (m *MockManager) GetGitStatus(ctx context.Context, workspaceID string) (*workspace.GitStatus, error) {
	if _, err := m.GetWorkspace(workspaceID); err != nil {
		return nil, err
	}

	return &workspace.GitStatus{
		Branch:        "main",
		Clean:         true,
		Modified:      []string{},
		Untracked:     []string{},
		CurrentCommit: "abc123",
	}, nil
}

// CommitChanges mock implementation
func (m *MockManager) CommitChanges(ctx context.Context, workspaceID string, opts *workspace.GitOptions) error {
	if _, err := m.GetWorkspace(workspaceID); err != nil {
		return err
	}

	return nil
}

// PushBranch mock implementation
func (m *MockManager) PushBranch(ctx context.Context, workspaceID string) error {
	if _, err := m.GetWorkspace(workspaceID); err != nil {
		return err
	}

	return nil
}
